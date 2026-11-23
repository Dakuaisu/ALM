use super::{ConfirmationFixture, ProductEntryError, Role, owner};

impl ConfirmationFixture {
    /// Sets the trusted synthetic fallback used when an Owner saves a blank reorder field.
    pub const fn with_product_reorder_default(mut self, default: u32) -> Self {
        self.product_reorder_default = default;
        self
    }

    /// Saves the synthetic fallback for future product entries.
    ///
    /// # Errors
    /// Denies Managers without changing the fallback.
    pub fn save_product_reorder_default(
        &mut self,
        role: Role,
        default: u32,
    ) -> Result<(), ProductEntryError> {
        owner(role)?;
        self.product_reorder_default = default;
        Ok(())
    }
}
