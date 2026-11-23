//! Suppliers screen (legacy VIEWS.suppliers): per-supplier purchase count and amount bought.
use super::{ConfirmationFixture, PurchaseError, PurchaseSupplierId, owner};
use crate::{Money, Role};

impl PurchaseSupplierId {
    /// Fixture-local position, for carrying a selection across IPC.
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplierBookRow {
    pub id: PurchaseSupplierId,
    pub name: String,
    /// Purchase orders with at least one receipt (`poBought(po) > 0`).
    pub purchases: usize,
    /// Received quantity times receipt rate, summed over every receipt (`poBought`).
    pub bought: Money,
}

/// Typed comparison quote: blank (or zero) clears it; otherwise exact non-negative rupees.
pub fn parse_quote(text: &str) -> Result<Option<Money>, crate::RupeesError> {
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Money::parse_rupees(text).map(|rate| (!rate.is_zero()).then_some(rate))
    }
}

impl ConfirmationFixture {
    /// Owner-only suppliers sorted by name.
    pub fn supplier_book(&self, role: Role) -> Result<Vec<SupplierBookRow>, PurchaseError> {
        owner(role)?;
        let mut rows = Vec::new();
        for supplier in &self.purchases.suppliers {
            let mut purchases = 0;
            let mut bought = 0_i64;
            for order in self
                .purchases
                .orders
                .iter()
                .filter(|o| o.supplier_id == supplier.id)
            {
                let mut order_total = 0_i64;
                for receipt in order.items.iter().flat_map(|line| line.receipts()) {
                    let value = receipt
                        .rate
                        .minor_units()
                        .checked_mul(i64::from(receipt.quantity.get()))
                        .ok_or(PurchaseError::ReceiptOverflow)?;
                    order_total = order_total
                        .checked_add(value)
                        .ok_or(PurchaseError::ReceiptOverflow)?;
                }
                if order_total > 0 {
                    purchases += 1;
                    bought = bought
                        .checked_add(order_total)
                        .ok_or(PurchaseError::ReceiptOverflow)?;
                }
            }
            rows.push(SupplierBookRow {
                id: supplier.id,
                name: supplier.name.clone(),
                purchases,
                bought: Money::from_minor_units(bought),
            });
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_quote;
    use crate::confirmation::PurchaseError;
    use crate::confirmation::demo_fixture::{fixture, seed_purchasing};
    use crate::{Money, Role};

    #[test]
    fn supplier_book_and_quote_text_follow_core_rules() {
        assert_eq!(parse_quote(" "), Ok(None));
        assert_eq!(parse_quote("0"), Ok(None));
        assert_eq!(
            parse_quote("68.50"),
            Ok(Some(Money::from_minor_units(6_850)))
        );
        assert!(parse_quote("-1").is_err());
        let mut fixture = fixture().unwrap();
        seed_purchasing(&mut fixture).unwrap();
        let rows = fixture.supplier_book(Role::Owner).unwrap();
        let names: Vec<_> = rows
            .iter()
            .map(|r| (r.name.as_str(), r.purchases, r.bought.minor_units()))
            .collect();
        assert_eq!(
            names,
            [("Synthetic Depot", 0, 0), ("Synthetic Wholesale", 0, 0)]
        );
        assert_eq!(
            fixture.supplier_book(Role::Manager),
            Err(PurchaseError::AccessDenied)
        );
    }
}
