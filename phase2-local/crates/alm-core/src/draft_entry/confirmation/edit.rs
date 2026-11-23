use super::BillEntryState;
use crate::Role;
use crate::confirmation::{ConfirmationError, ConfirmationFixture, CreationKey, Receipt, SaleEdit};
use std::time::{SystemTime, UNIX_EPOCH};

fn current_creation_key() -> Result<CreationKey, ConfirmationError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ConfirmationError::Overflow)?
        .as_millis();
    Ok(CreationKey::new(
        u64::try_from(millis).map_err(|_| ConfirmationError::Overflow)?,
    ))
}

impl BillEntryState {
    #[cfg(test)]
    pub(crate) fn set_edit_clock_for_test(
        &mut self,
        clock: fn() -> Result<CreationKey, ConfirmationError>,
    ) {
        self.edit_clock = Some(clock);
    }

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
        #[cfg(test)]
        let clock = self.edit_clock.unwrap_or(current_creation_key);
        #[cfg(not(test))]
        let clock = current_creation_key;
        let receipt = authority.save_edit_with_clock(role, edit, clock)?;
        self.preview = Some(receipt.clone());
        self.edit = None;
        Ok(receipt)
    }
}
