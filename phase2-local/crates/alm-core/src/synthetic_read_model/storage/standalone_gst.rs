use std::io;

use super::{Archive, Error, ReadModel, Records, owner};
use crate::Role;
use crate::confirmation::{GstBill, GstBillError, GstBillId, GstBillInvoice};

impl ReadModel {
    pub fn create_with_standalone_gst(
        role: Role,
        records: Records,
        bills: Vec<GstBill>,
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            Archive {
                standalone_gst: Some(bills),
                ..Archive::new(records)
            },
            create_new,
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
