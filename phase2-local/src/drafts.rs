use std::num::NonZeroU32;

/// Identity of a draft; not an invoice number.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DraftId(u64);

impl DraftId {
    /// Wraps a synthetic local identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
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

/// Editable local draft with no payment, stock, or final invoice state.
#[derive(Debug, Eq, PartialEq)]
pub struct ProvisionalDraft {
    id: DraftId,
    lines: Vec<DraftLine>,
}

impl ProvisionalDraft {
    /// Takes ownership of the draft lines, which may be empty while editing.
    pub const fn new(id: DraftId, lines: Vec<DraftLine>) -> Self {
        Self { id, lines }
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
