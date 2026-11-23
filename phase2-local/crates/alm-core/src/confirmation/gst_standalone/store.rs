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
        owner(role).map_err(Bill)?;
        if !draft
            .lines
            .iter()
            .any(|line| !line.item.is_empty() && line.quantity != 0)
        {
            return Err(Bill(GstBillError::MissingItems));
        }
        if draft.customer.is_empty() {
            return Err(Bill(GstBillError::MissingCustomer));
        }
        let date = draft.date.ok_or(Bill(GstBillError::MissingDate))?;
        super::validate_references(draft.gstin, draft.eway_number).map_err(Bill)?;
        if self
            .gst_bills
            .get(&draft.id)
            .is_some_and(|bill| bill.cancelled)
        {
            return Err(Bill(GstBillError::Cancelled));
        }
        if self.gst_deleted.contains(&draft.id) {
            return Err(Bill(GstBillError::Deleted));
        }
        let bill = GstBill::from_draft(draft, date, |item| {
            self.catalog.product_id_by_exact_name(item)
        })
        .map_err(Bill)?;
        let projection = self.gst_invoice(&bill).map_err(Bill)?;
        store.put_gst_bill(&bill).map_err(Store)?;
        self.gst_bills.insert(draft.id, bill);
        sink(&projection);
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
