//! A role downgrade or revocation seen on a pull: further commands are blocked at once and the
//! account's disallowed projection rows, queued intents and output and temporary files are
//! removed from the device (with their bytes overwritten in the SQLite files).
use alm_core::Role;
use alm_store::ProjectionStore;

use crate::{Trial, text};

impl Trial {
    /// The authority now grants `granted` (a Manager): the Owner projection file, the outbox and
    /// every output and temporary file go, and the device continues as that role.
    pub(super) fn downgrade(&mut self, granted: Role) -> Result<(), String> {
        self.forget(Role::Owner)?;
        self.owner = granted == Role::Owner;
        self.message = "This account is no longer an Owner on the authority: Owner data and queued actions were removed from this device.".into();
        self.refresh()
    }

    /// The authority refused this account's pull: everything it held here goes and every
    /// command is blocked until another account is chosen.
    pub(super) fn revoke(&mut self) -> Result<(), String> {
        if self.revoked {
            return Ok(());
        }
        let other = match self.role() {
            Role::Owner => Role::Manager,
            Role::Manager => Role::Owner,
        };
        ProjectionStore::remove_scoped(&self.device_root, &self.account, other).map_err(text)?;
        self.projection.purge().map_err(text)?;
        self.forget_queued()?;
        self.revoked = true;
        self.posted = crate::device::Posted::empty()?;
        self.message = "Access to this account was revoked by the authority: its data was removed from this device.".into();
        Ok(())
    }

    /// Removes the `role` projection of this account (the open store moves to the other role).
    fn forget(&mut self, role: Role) -> Result<(), String> {
        if self.projection.role() == role {
            self.projection.purge().map_err(text)?;
            let other = match role {
                Role::Owner => Role::Manager,
                Role::Manager => Role::Owner,
            };
            self.projection = ProjectionStore::open_scoped(&self.device_root, &self.account, other)
                .map_err(text)?;
        }
        ProjectionStore::remove_scoped(&self.device_root, &self.account, role).map_err(text)?;
        self.posted = crate::device::Posted::empty()?;
        self.forget_queued()
    }

    /// Queued intents, open reviews and forms, and output and temporary files.
    fn forget_queued(&mut self) -> Result<(), String> {
        self.outbox.purge().map_err(text)?;
        self.reviewed = None;
        self.posting.invalidate();
        self.sale_edit = None;
        self.cash_form = None;
        self.gst_form = None;
        self.receiving = None;
        self.crate_form = None;
        self.print_view = None;
        self.restored = None;
        for dir in [self.outputs.clone(), self.device_root.join("restored")] {
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        match std::fs::remove_file(self.device_root.join("shop.json.restoring")) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}
