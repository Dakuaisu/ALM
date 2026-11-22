use std::num::{NonZeroU32, NonZeroUsize};

use crate::{BillDate, Money};

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

#[cfg(test)]
mod quantity_tests {
    use super::*;

    #[test]
    fn replaces_only_the_selected_ordinary_line_quantity() {
        // Given an ordinary draft with two lines and independent customer, date, and crate data.
        let one = NonZeroU32::MIN;
        let two = NonZeroU32::new(2).unwrap();
        let first = DraftLine::new(ProductId::new(1), two);
        let second = DraftLine::new(ProductId::new(2), one);
        let mut draft = ProvisionalDraft::new(DraftId::new(7), vec![first, second])
            .with_bill_date(Some(BillDate::new(2025, 2, 12).unwrap()))
            .with_customer(DraftCustomer::Selected {
                id: DraftCustomerId::new("customer-1".into()),
                name: "Synthetic Customer".into(),
            });
        draft.add_crate_row(CrateRow::new(one, vec![])).unwrap();
        let mut expected = draft.clone();
        expected.lines[1] = DraftLine::new(ProductId::new(2), two);

        // When the second ordinary line is edited.
        let previous = draft.replace_line_quantity(NonZeroUsize::new(2).unwrap(), two);

        // Then only its quantity changes.
        assert_eq!(previous, Some(one));
        assert_eq!(draft, expected);
    }
}

/// Identity of a product referenced by a draft line.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
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

impl std::fmt::Display for ProductId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
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
    price: Option<CrateRowPrice>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CrateRowPrice {
    name: String,
    rate: Money,
}

impl CrateRow {
    pub const fn new(quantity: NonZeroU32, components: Vec<CrateComponent>) -> Self {
        Self {
            quantity,
            components,
            price: None,
        }
    }

    pub(crate) fn with_price(mut self, name: String, rate: Money) -> Self {
        self.price = Some(CrateRowPrice { name, rate });
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.price.as_ref().map(|price| price.name.as_str())
    }

    pub fn rate(&self) -> Option<Money> {
        self.price.as_ref().map(|price| price.rate)
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
    bill_date: Option<BillDate>,
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
            bill_date: None,
        }
    }

    pub fn with_customer(mut self, customer: DraftCustomer) -> Self {
        self.customer = customer;
        self
    }

    /// Sets or clears the explicit bill date while editing.
    pub const fn with_bill_date(mut self, bill_date: Option<BillDate>) -> Self {
        self.bill_date = bill_date;
        self
    }

    pub const fn bill_date(&self) -> Option<BillDate> {
        self.bill_date
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

    /// Replaces the quantity of an ordinary line by its one-based position.
    /// Returns the previous quantity, or `None` if the line does not exist.
    pub fn replace_line_quantity(
        &mut self,
        line_number: NonZeroUsize,
        quantity: NonZeroU32,
    ) -> Option<NonZeroU32> {
        let line = self.lines.get_mut(line_number.get() - 1)?;
        Some(std::mem::replace(&mut line.quantity, quantity))
    }
}
