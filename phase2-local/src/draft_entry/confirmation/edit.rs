use super::BillEntryState;
use crate::Role;
use crate::confirmation::{ConfirmationError, ConfirmationFixture, Receipt, SaleEdit};

impl BillEntryState {
    pub fn open_edit(&mut self, role: Role, edit: SaleEdit) -> Result<(), ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        self.edit = Some(edit);
        Ok(())
    }

    pub fn cancel_edit(&mut self) {
        self.edit = None;
    }

    pub fn save_edit(
        &mut self,
        role: Role,
        authority: &mut ConfirmationFixture,
    ) -> Result<Receipt, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let edit = self.edit.as_ref().ok_or(ConfirmationError::InvalidDraft)?;
        let receipt = authority.save_edit(role, edit)?;
        self.preview = Some(receipt.clone());
        self.edit = None;
        Ok(receipt)
    }
}
