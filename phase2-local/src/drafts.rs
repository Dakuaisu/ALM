use std::num::NonZeroU32;

/// Identity of a draft; not an invoice number.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DraftId(u64);

impl DraftId {
    /// Wraps a synthetic local identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

/// Identity of a product referenced by a draft line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductId(u64);

impl ProductId {
    /// Wraps a synthetic product identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

/// A requested quantity, not a stock reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftLine {
    product_id: ProductId,
    quantity: NonZeroU32,
}

impl DraftLine {
    /// Builds a line with a positive whole-unit quantity.
    pub const fn new(product_id: ProductId, quantity: NonZeroU32) -> Self {
        Self {
            product_id,
            quantity,
        }
    }

    /// Returns the requested product.
    pub const fn product_id(self) -> ProductId {
        self.product_id
    }

    /// Returns the requested quantity.
    pub const fn quantity(self) -> NonZeroU32 {
        self.quantity
    }
}

/// Case corresponds to the legacy Packet unit, not a multiple of the pack size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaleUnit {
    Piece,
    Case,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrateComponent {
    pub product: ProductId,
    pub quantity: NonZeroU32,
    pub unit: SaleUnit,
}

impl CrateComponent {
    pub const fn new(product: ProductId, quantity: NonZeroU32, unit: SaleUnit) -> Self {
        Self {
            product,
            quantity,
            unit,
        }
    }
}

/// Editable crate row. Order and repeated products are preserved until review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrateRow {
    quantity: NonZeroU32,
    components: Vec<CrateComponent>,
}

impl CrateRow {
    pub const fn new(quantity: NonZeroU32, components: Vec<CrateComponent>) -> Self {
        Self {
            quantity,
            components,
        }
    }

    pub const fn quantity(&self) -> NonZeroU32 {
        self.quantity
    }

    pub fn components(&self) -> &[CrateComponent] {
        &self.components
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DraftCustomerId(String);

impl DraftCustomerId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

/// Customer input owned by a provisional draft.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DraftCustomer {
    Unassigned,
    Typed(String),
    Selected { id: DraftCustomerId, name: String },
}

/// Editable local draft with no payment, stock, or final invoice state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvisionalDraft {
    id: DraftId,
    lines: Vec<DraftLine>,
    crate_row: Option<CrateRow>,
    customer: DraftCustomer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrateRowAlreadyPresent;

impl ProvisionalDraft {
    /// Takes ownership of the draft lines, which may be empty while editing.
    pub const fn new(id: DraftId, lines: Vec<DraftLine>) -> Self {
        Self {
            id,
            lines,
            crate_row: None,
            customer: DraftCustomer::Unassigned,
        }
    }

    pub fn with_customer(mut self, customer: DraftCustomer) -> Self {
        self.customer = customer;
        self
    }

    /// Adds the first crate row without replacing an existing one.
    ///
    /// # Errors
    /// Returns [`CrateRowAlreadyPresent`] without changing the draft if it already has a crate row.
    pub fn add_crate_row(&mut self, row: CrateRow) -> Result<(), CrateRowAlreadyPresent> {
        if self.crate_row.is_some() {
            return Err(CrateRowAlreadyPresent);
        }
        self.crate_row = Some(row);
        Ok(())
    }

    /// Replaces an existing crate row, returning the previous row. An empty slot is unchanged.
    pub fn replace_crate_row(&mut self, row: CrateRow) -> Option<CrateRow> {
        self.crate_row
            .as_mut()
            .map(|existing| std::mem::replace(existing, row))
    }

    pub const fn crate_row(&self) -> Option<&CrateRow> {
        self.crate_row.as_ref()
    }

    pub const fn customer(&self) -> &DraftCustomer {
        &self.customer
    }

    /// Returns the draft identity.
    pub const fn id(&self) -> DraftId {
        self.id
    }

    /// Returns draft lines without owner-only data.
    pub fn lines(&self) -> &[DraftLine] {
        &self.lines
    }
}
