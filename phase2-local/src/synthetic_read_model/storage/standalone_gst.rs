use std::path::Path;

use super::{Archive, Error, Records, Store, owner};
use crate::Role;
use crate::confirmation::{GstBill, GstBillError, GstBillId, GstBillInvoice};

impl Store {
    pub fn create_with_standalone_gst(
        path: &Path,
        role: Role,
        records: Records,
        bills: Vec<GstBill>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            path,
            Archive {
                standalone_gst: Some(bills),
                ..Archive::new(records)
            },
        )
    }

    pub fn emit_standalone_gst(
        &self,
        role: Role,
        id: GstBillId,
        sink: &mut impl FnMut(&GstBillInvoice),
    ) -> Result<(), GstBillError> {
        owner(role).map_err(|_| GstBillError::AccessDenied)?;
        let bill = self
            .standalone_gst
            .iter()
            .flatten()
            .find(|bill| bill.id == id)
            .ok_or(GstBillError::NotFound)?;
        sink(&bill.invoice()?);
        Ok(())
    }
}
