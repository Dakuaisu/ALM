use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

use serde::{Deserialize, Serialize};

use super::{GstBill, GstBillDraft, GstBillError, GstBillId, GstBillInvoice, owner};
use crate::Role;
use crate::confirmation::ConfirmationFixture;

/// Stored standalone GST bills plus deletion markers (`DB.deleted` entries for `gstBills`).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct GstBillLedger {
    pub bills: Vec<GstBill>,
    pub deleted: Vec<GstBillId>,
}

/// Device storage for standalone GST bills; each write commits atomically or not at all.
pub trait GstBillStore {
    type Error;

    /// Loads every stored bill and deletion marker.
    ///
    /// # Errors
    /// Returns the store's error if the records cannot be read.
    fn load_gst_bills(&self) -> Result<GstBillLedger, Self::Error>;

    /// Inserts or replaces one bill and all of its lines.
    ///
    /// # Errors
    /// Returns the store's error if the write cannot be committed.
    fn put_gst_bill(&mut self, bill: &GstBill) -> Result<(), Self::Error>;

    /// Removes a bill and its lines and records its deletion marker.
    ///
    /// # Errors
    /// Returns the store's error if the write cannot be committed.
    fn delete_gst_bill(&mut self, id: GstBillId) -> Result<(), Self::Error>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum GstBillStoreError<E> {
    Bill(GstBillError),
    Store(E),
}

impl GstBillStoreError<Infallible> {
    pub(super) fn into_bill(self) -> GstBillError {
        match self {
            Self::Bill(error) => error,
            Self::Store(never) => match never {},
        }
    }
}

/// The in-memory fixture path: nothing outlives the process.
pub(super) struct Unpersisted;

impl GstBillStore for Unpersisted {
    type Error = Infallible;

    fn load_gst_bills(&self) -> Result<GstBillLedger, Infallible> {
        Ok(GstBillLedger::default())
    }

    fn put_gst_bill(&mut self, _: &GstBill) -> Result<(), Infallible> {
        Ok(())
    }

    fn delete_gst_bill(&mut self, _: GstBillId) -> Result<(), Infallible> {
        Ok(())
    }
}

use GstBillStoreError::{Bill, Store};

impl ConfirmationFixture {
    /// Saves like [`Self::save_gst_bill`], committing to `store` before memory changes.
    pub fn save_gst_bill_to<S: GstBillStore>(
        &mut self,
        role: Role,
        draft: &GstBillDraft<'_>,
        store: &mut S,
        sink: &mut impl FnMut(&GstBillInvoice),
    ) -> Result<(), GstBillStoreError<S::Error>> {
        let bill = self.gst_bill_record(role, draft).map_err(Bill)?;
        let projection = self.gst_invoice(&bill).map_err(Bill)?;
        store.put_gst_bill(&bill).map_err(Store)?;
        self.gst_bills.insert(draft.id, bill);
        sink(&projection);
        Ok(())
    }

    /// The record `save_gst_bill` would store for `draft`, validated, without saving it.
    ///
    /// # Errors
    /// As [`Self::save_gst_bill`].
    pub fn gst_bill_record(
        &self,
        role: Role,
        draft: &GstBillDraft<'_>,
    ) -> Result<GstBill, GstBillError> {
        owner(role)?;
        if !draft
            .lines
            .iter()
            .any(|line| !line.item.is_empty() && line.quantity != 0)
        {
            return Err(GstBillError::MissingItems);
        }
        if draft.customer.is_empty() {
            return Err(GstBillError::MissingCustomer);
        }
        let date = draft.date.ok_or(GstBillError::MissingDate)?;
        super::validate_references(draft.gstin, draft.eway_number)?;
        self.check_gst_writable(draft.id)?;
        GstBill::from_draft(draft, date, |item| {
            self.catalog.product_id_by_exact_name(item)
        })
    }

    fn check_gst_writable(&self, id: GstBillId) -> Result<(), GstBillError> {
        if self.gst_bills.get(&id).is_some_and(|bill| bill.cancelled) {
            return Err(GstBillError::Cancelled);
        }
        if self.gst_deleted.contains(&id) {
            return Err(GstBillError::Deleted);
        }
        Ok(())
    }

    /// Stores an already built record (the authority path): re-checks that the bill is
    /// writable and printable, then inserts or replaces it.
    ///
    /// # Errors
    /// Denies Managers; rejects cancelled, deleted or unprintable bills.
    pub fn put_gst_bill_record(&mut self, role: Role, bill: GstBill) -> Result<(), GstBillError> {
        owner(role)?;
        self.check_gst_writable(bill.id)?;
        if bill.cancelled {
            return Err(GstBillError::Cancelled);
        }
        self.gst_invoice(&bill)?;
        self.gst_bills.insert(bill.id, bill);
        Ok(())
    }

    /// Imports a bill or deletion marker exactly as a device stored it (one-time migration of
    /// device GST tables into the authority). An identical bill already present is a no-op.
    ///
    /// # Errors
    /// Denies Managers; rejects a different bill under the same ID or a duplicate number.
    pub fn import_gst_bill_record(
        &mut self,
        role: Role,
        bill: Option<GstBill>,
        deleted: Option<GstBillId>,
    ) -> Result<(), GstBillError> {
        owner(role)?;
        if let Some(bill) = bill {
            match self.gst_bills.get(&bill.id) {
                Some(existing) if *existing == bill => return Ok(()),
                Some(_) => return Err(GstBillError::Conflict),
                None => {}
            }
            if self.gst_deleted.contains(&bill.id)
                || self.gst_bills.values().any(|b| b.number == bill.number)
            {
                return Err(GstBillError::Conflict);
            }
            self.gst_invoice(&bill)?;
            self.gst_bills.insert(bill.id, bill);
        }
        if let Some(id) = deleted {
            if self.gst_bills.contains_key(&id) {
                return Err(GstBillError::Conflict);
            }
            self.gst_deleted.insert(id);
        }
        Ok(())
    }

    /// Cancels like [`Self::cancel_gst_bill`], committing to `store` first.
    pub fn cancel_gst_bill_to<S: GstBillStore>(
        &mut self,
        role: Role,
        id: GstBillId,
        store: &mut S,
    ) -> Result<(), GstBillStoreError<S::Error>> {
        owner(role).map_err(Bill)?;
        let mut bill = self
            .gst_bills
            .get(&id)
            .ok_or(Bill(GstBillError::NotFound))?
            .clone();
        bill.cancelled = true;
        store.put_gst_bill(&bill).map_err(Store)?;
        self.gst_bills.insert(id, bill);
        Ok(())
    }

    /// Deletes like [`Self::delete_gst_bill`], committing to `store` first.
    pub fn delete_gst_bill_to<S: GstBillStore>(
        &mut self,
        role: Role,
        id: GstBillId,
        store: &mut S,
    ) -> Result<(), GstBillStoreError<S::Error>> {
        owner(role).map_err(Bill)?;
        if !self.gst_bills.contains_key(&id) {
            return Err(Bill(GstBillError::NotFound));
        }
        store.delete_gst_bill(id).map_err(Store)?;
        self.gst_bills.remove(&id);
        self.gst_deleted.insert(id);
        Ok(())
    }

    /// Replaces the in-memory bills and markers with the store's, after validating every bill.
    pub fn load_gst_bills_from<S: GstBillStore>(
        &mut self,
        role: Role,
        store: &S,
    ) -> Result<(), GstBillStoreError<S::Error>> {
        owner(role).map_err(Bill)?;
        let ledger = store.load_gst_bills().map_err(Store)?;
        let deleted: BTreeSet<_> = ledger.deleted.into_iter().collect();
        let mut bills = BTreeMap::new();
        for bill in ledger.bills {
            bill.validate_saved().map_err(Bill)?;
            if deleted.contains(&bill.id) {
                return Err(Bill(GstBillError::Deleted));
            }
            bills.insert(bill.id, bill);
        }
        self.gst_bills = bills;
        self.gst_deleted = deleted;
        Ok(())
    }
}
