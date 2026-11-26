//! The catalog as Owner book rows, and back.
use std::collections::BTreeMap;

use crate::DraftId;

use super::physical_count::PhysicalCountState;
use super::product_entry::ProductDetails;
use super::{AllocationState, Catalog, Lot, Product, ProductPricing};
use crate::confirmation::{ConfirmationError, ProductTaxOverrides};
use crate::projection::book::{BookAllocation, BookLot, BookProduct, BookProductDetails, BookSale};
use crate::{Money, ProductId};

fn money(value: Option<i64>) -> Option<Money> {
    value.map(Money::from_minor_units)
}

fn index(value: usize) -> Result<u64, ConfirmationError> {
    u64::try_from(value).map_err(|_| ConfirmationError::Overflow)
}

fn position(value: u64) -> Result<usize, ConfirmationError> {
    usize::try_from(value).map_err(|_| ConfirmationError::Overflow)
}

/// A sale's frozen cost, frozen line unit costs and lot allocations.
pub(in crate::confirmation) type SaleCosts = (
    Option<Money>,
    Vec<(u64, Option<Money>)>,
    Vec<BookAllocation>,
);

impl Catalog {
    pub(in crate::confirmation) fn book_products(&self) -> Vec<BookProduct> {
        self.product_order
            .iter()
            .filter_map(|id| Some((id.value(), self.products.get(&id.value())?)))
            .map(|(id, p)| BookProduct {
                id,
                details: p.details.as_ref().map(|d| BookProductDetails {
                    item: d.item.clone(),
                    brand: d.brand.clone(),
                    reorder_level: d.reorder_level,
                    gst_rate: d.tax.gst_rate,
                    cess: d.tax.cess,
                    hsn: d.tax.hsn.clone(),
                }),
                buy: p.pricing.buy(),
                margin: p.pricing.margin(),
                sell: p.pricing.sell(),
                stock: p.stock,
                seeded_opening: p.seeded_opening,
                fridge: p.physical_count.fridge,
                lots: p
                    .lots
                    .iter()
                    .map(|lot| BookLot {
                        date: lot.date,
                        remaining: lot.remaining,
                        unit_cost: money(lot.unit_cost),
                    })
                    .collect(),
                counts: p.physical_count.movements.clone(),
            })
            .collect()
    }

    pub(in crate::confirmation) fn book_sale_costs(
        &self,
        sale: DraftId,
    ) -> Result<SaleCosts, ConfirmationError> {
        let a = &self.allocations;
        let cost = money(a.sale_costs.get(&sale).copied().flatten());
        // The sale's own entries only (keys start with the sale): a range, not a full scan.
        let units = a
            .line_units
            .range((sale, 0)..)
            .take_while(|((s, _), _)| *s == sale)
            .map(|(&(_, line), unit)| Ok((index(line)?, money(*unit))))
            .collect::<Result<_, ConfirmationError>>()?;
        let allocations = a
            .records
            .range((sale, 0, 0, 0)..)
            .take_while(|((s, ..), _)| *s == sale)
            .map(|(&(_, line, product, lot), &(quantity, cost))| {
                Ok(BookAllocation {
                    line: index(line)?,
                    product,
                    lot: index(lot)?,
                    quantity,
                    cost: money(cost),
                })
            })
            .collect::<Result<_, ConfirmationError>>()?;
        Ok((cost, units, allocations))
    }

    pub(in crate::confirmation) fn from_book(
        products: &[BookProduct],
        sales: &[BookSale],
    ) -> Result<Self, ConfirmationError> {
        let mut catalog = Self {
            products: BTreeMap::new(),
            product_order: Vec::new(),
            allocations: AllocationState {
                records: BTreeMap::new(),
                sale_costs: BTreeMap::new(),
                line_units: BTreeMap::new(),
            },
        };
        for p in products {
            let product = Product {
                stock: p.stock,
                seeded_opening: p.seeded_opening,
                pricing: ProductPricing::from_parts(p.buy, p.margin, p.sell),
                details: p.details.as_ref().map(|d| ProductDetails {
                    item: d.item.clone(),
                    brand: d.brand.clone(),
                    reorder_level: d.reorder_level,
                    tax: ProductTaxOverrides {
                        gst_rate: d.gst_rate,
                        cess: d.cess,
                        hsn: d.hsn.clone(),
                    },
                }),
                lots: p
                    .lots
                    .iter()
                    .map(|lot| Lot {
                        remaining: lot.remaining,
                        unit_cost: lot.unit_cost.map(Money::minor_units),
                        date: lot.date,
                    })
                    .collect(),
                physical_count: PhysicalCountState {
                    fridge: p.fridge,
                    movements: p.counts.clone(),
                },
            };
            if catalog.products.insert(p.id, product).is_some() {
                return Err(ConfirmationError::InvalidDraft);
            }
            catalog.product_order.push(ProductId::new(p.id));
        }
        let a = &mut catalog.allocations;
        for sale in sales {
            a.sale_costs
                .insert(sale.draft, sale.cost.map(Money::minor_units));
            for (line, unit) in &sale.line_units {
                a.line_units
                    .insert((sale.draft, position(*line)?), unit.map(Money::minor_units));
            }
            for r in &sale.allocations {
                a.records.insert(
                    (sale.draft, position(r.line)?, r.product, position(r.lot)?),
                    (r.quantity, r.cost.map(Money::minor_units)),
                );
            }
        }
        Ok(catalog)
    }
}
