//! Products screen (legacy VIEWS.products): price list rows, brand chips and the edit form text.
use std::collections::BTreeSet;

use crate::confirmation::{ConfirmationError, ConfirmationFixture, ProductPriceInput};
use crate::{Money, ProductId, Role};

use super::product_entry::owner;
use super::{ProductEntry, ProductEntryError, StockStatus};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductListRow {
    pub product: ProductId,
    pub item: String,
    pub brand: String,
    pub buy: Money,
    pub sell: Money,
    /// `((sell - buy) / buy * 100).toFixed(1) + "%"`, or `None` when buy is 0 (shown "—").
    pub margin_percent: Option<String>,
    pub on_hand: u32,
    pub status: StockStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductList {
    pub rows: Vec<ProductListRow>,
    /// Distinct non-empty brands, sorted, for the filter chips.
    pub brands: Vec<String>,
}

/// The edit form as typed; blank prices keep the current value, blank reorder uses the default.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProductText<'a> {
    pub item: &'a str,
    pub brand: &'a str,
    pub buy: &'a str,
    pub sell: &'a str,
    pub reorder_level: &'a str,
}

/// One decimal of a percent, rounded half away from zero on exact integers.
fn margin_percent(buy: Money, sell: Money) -> Option<String> {
    let buy = i128::from(buy.minor_units());
    if buy == 0 {
        return None;
    }
    let tenths_x2 = (i128::from(sell.minor_units()) - buy) * 2_000 / buy;
    let tenths = (tenths_x2 + tenths_x2.signum()) / 2;
    if tenths == 0 {
        return None;
    }
    let sign = if tenths < 0 { "-" } else { "" };
    let abs = tenths.unsigned_abs();
    Some(format!("{sign}{}.{}%", abs / 10, abs % 10))
}

/// Which price box the user last typed in (`buy` / `marg` / `sell` input listeners).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PriceField {
    Buy,
    Margin,
    Sell,
}

/// `+input.value || 0` for a price box; the margin may be negative when sell is below buy.
fn price_text(text: &str, signed: bool) -> Result<Money, ProductEntryError> {
    let invalid = || ProductEntryError::Catalog(ConfirmationError::InvalidAmount);
    let text = text.trim();
    if text.is_empty() {
        return Ok(Money::from_minor_units(0));
    }
    match text.strip_prefix('-') {
        Some(rest) if signed => Money::parse_rupees(rest)
            .ok()
            .and_then(|m| m.minor_units().checked_neg())
            .map(Money::from_minor_units)
            .ok_or_else(invalid),
        _ => Money::parse_rupees(text).map_err(|_| invalid()),
    }
}

impl ProductEntry {
    /// The live price recalculation (`recalc`): typing buy or margin sets sell = buy + margin;
    /// typing sell sets margin = sell - buy. The entry's prices are replaced; nothing is saved.
    ///
    /// # Errors
    /// Rejects inexact or negative buy/sell text and arithmetic overflow, leaving prices unchanged.
    pub fn preview_prices(
        &mut self,
        buy: &str,
        margin: &str,
        sell: &str,
        changed: PriceField,
    ) -> Result<(), ProductEntryError> {
        let (buy, margin, sell) = (
            price_text(buy, false)?,
            price_text(margin, true)?,
            price_text(sell, false)?,
        );
        let mut next = self.clone();
        let steps = match changed {
            PriceField::Buy => [
                ProductPriceInput::Margin(margin),
                ProductPriceInput::Buy(buy),
            ],
            PriceField::Margin => [
                ProductPriceInput::Buy(buy),
                ProductPriceInput::Margin(margin),
            ],
            PriceField::Sell => [ProductPriceInput::Buy(buy), ProductPriceInput::Sell(sell)],
        };
        for step in steps {
            next.apply_price(step).map_err(ProductEntryError::Catalog)?;
        }
        *self = next;
        Ok(())
    }

    /// Applies typed text with the HTML's order (buy first, then sell; margin follows).
    ///
    /// # Errors
    /// Rejects amounts that are not exact non-negative rupees and non-numeric reorder levels.
    pub fn apply_text(&mut self, text: &ProductText<'_>) -> Result<(), ProductEntryError> {
        let invalid = ProductEntryError::Catalog(ConfirmationError::InvalidAmount);
        self.item = text.item.trim().to_owned();
        self.brand = text.brand.trim().to_owned();
        for (value, input) in [
            (
                text.buy,
                ProductPriceInput::Buy as fn(Money) -> ProductPriceInput,
            ),
            (text.sell, ProductPriceInput::Sell),
        ] {
            if !value.trim().is_empty() {
                let money = Money::parse_rupees(value).map_err(|_| invalid.clone())?;
                self.apply_price(input(money))
                    .map_err(ProductEntryError::Catalog)?;
            }
        }
        let reorder = text.reorder_level.trim();
        self.reorder_level = if reorder.is_empty() {
            None
        } else if reorder.bytes().all(|b| b.is_ascii_digit()) {
            Some(reorder.parse().map_err(|_| invalid.clone())?)
        } else {
            return Err(invalid);
        };
        Ok(())
    }
}

impl ConfirmationFixture {
    /// Owner-only price list in catalog order, filtered by brand and a case-insensitive name.
    pub fn product_list(
        &self,
        role: Role,
        query: &str,
        brand: Option<&str>,
    ) -> Result<ProductList, ProductEntryError> {
        owner(role)?;
        let needle = query.trim().to_lowercase();
        let mut brands = BTreeSet::new();
        let mut rows = Vec::new();
        for (id, on_hand, reorder) in self
            .catalog
            .order_planner_stock(self.product_reorder_default)
        {
            let Some(product) = self.catalog.products.get(&id.value()) else {
                continue;
            };
            let Some(details) = product.details.as_ref() else {
                continue;
            };
            if !details.brand.is_empty() {
                brands.insert(details.brand.clone());
            }
            if brand.is_some_and(|b| b != details.brand)
                || !details.item.to_lowercase().contains(&needle)
            {
                continue;
            }
            let pricing = product.pricing;
            rows.push(ProductListRow {
                product: id,
                item: details.item.clone(),
                brand: details.brand.clone(),
                buy: pricing.buy(),
                sell: pricing.sell(),
                margin_percent: margin_percent(pricing.buy(), pricing.sell()),
                on_hand,
                status: if on_hand == 0 {
                    StockStatus::Out
                } else if on_hand <= reorder {
                    StockStatus::Low
                } else {
                    StockStatus::Ok
                },
            });
        }
        Ok(ProductList {
            rows,
            brands: brands.into_iter().collect(),
        })
    }

    /// A product ID above every catalog product, for a new entry.
    pub fn next_product_id(&self, role: Role) -> Result<ProductId, ProductEntryError> {
        owner(role)?;
        self.catalog
            .products
            .keys()
            .max()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .map(ProductId::new)
            .ok_or(ProductEntryError::Catalog(ConfirmationError::Overflow))
    }
}

#[cfg(test)]
mod tests {
    use super::{ProductText, margin_percent};
    use crate::confirmation::demo_fixture::fixture;
    use crate::confirmation::{ConfirmationError, ProductEntryError, StockStatus};
    use crate::{Money, ProductId, Role};

    #[test]
    fn preview_recalculates_like_the_price_listeners() {
        use super::PriceField;
        let fixture = fixture().unwrap();
        let mut entry = fixture
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap();
        entry
            .preview_prices("38", "", "62.50", PriceField::Sell)
            .unwrap();
        let p = entry.pricing();
        assert_eq!(
            (
                p.buy().minor_units(),
                p.margin().minor_units(),
                p.sell().minor_units()
            ),
            (3_800, 2_450, 6_250)
        );
        entry
            .preview_prices("40", "24.50", "62.50", PriceField::Buy)
            .unwrap();
        assert_eq!(entry.pricing().sell().minor_units(), 6_450);
        entry
            .preview_prices("40", "-2", "", PriceField::Margin)
            .unwrap();
        assert_eq!(entry.pricing().sell().minor_units(), 3_800);
        entry
            .preview_prices("40", "", "30", PriceField::Sell)
            .unwrap();
        assert_eq!(entry.pricing().margin().minor_units(), -1_000);
        let before = entry.clone();
        assert!(entry.preview_prices("-1", "", "", PriceField::Buy).is_err());
        assert!(
            entry
                .preview_prices("1", "x", "", PriceField::Margin)
                .is_err()
        );
        assert_eq!(entry, before);
    }

    #[test]
    fn margin_percent_follows_to_fixed_one() {
        let m = |b, s| margin_percent(Money::from_minor_units(b), Money::from_minor_units(s));
        assert_eq!(m(7_000, 12_050), Some("72.1%".into()));
        assert_eq!(m(3_000, 3_100), Some("3.3%".into()));
        assert_eq!(m(0, 100), None);
        assert_eq!(m(100, 100), None);
        assert_eq!(m(1_000, 900), Some("-10.0%".into()));
    }

    #[test]
    fn product_list_and_text_form_follow_core_pricing() {
        let mut fixture = fixture().unwrap();
        let id = fixture.next_product_id(Role::Owner).unwrap();
        assert_eq!(id, ProductId::new(3));
        let mut entry = fixture.begin_product_entry(Role::Owner, id).unwrap();
        let text = ProductText {
            item: " Synthetic Tonic ",
            brand: "Synthetic Co",
            buy: "30",
            sell: "31",
            reorder_level: "4",
        };
        entry.apply_text(&text).unwrap();
        assert_eq!(entry.pricing().margin().minor_units(), 100);
        fixture.save_product_entry(Role::Owner, &entry).unwrap();
        let list = fixture.product_list(Role::Owner, "", None).unwrap();
        let tonic = list.rows.iter().find(|r| r.product == id).unwrap();
        assert_eq!(
            (
                tonic.item.as_str(),
                tonic.buy.minor_units(),
                tonic.sell.minor_units()
            ),
            ("Synthetic Tonic", 3_000, 3_100)
        );
        assert_eq!(tonic.margin_percent.as_deref(), Some("3.3%"));
        assert_eq!(tonic.status, StockStatus::Out);
        assert_eq!(list.brands, ["Synthetic Co"]);
        let only = fixture
            .product_list(Role::Owner, "", Some("Synthetic Co"))
            .unwrap();
        assert_eq!(only.rows.len(), 1);
        assert_eq!(
            fixture
                .product_list(Role::Owner, "LIME", None)
                .unwrap()
                .rows
                .len(),
            1
        );
        let mut bad = fixture.open_product_entry(Role::Owner, id).unwrap();
        assert_eq!(
            bad.apply_text(&ProductText {
                buy: "-1",
                ..text.clone()
            }),
            Err(ProductEntryError::Catalog(ConfirmationError::InvalidAmount))
        );
        assert_eq!(
            fixture.product_list(Role::Manager, "", None),
            Err(ProductEntryError::AccessDenied)
        );
    }
}
