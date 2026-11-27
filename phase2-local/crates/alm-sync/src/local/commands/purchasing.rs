//! Products, supplier quotes and purchase orders.
use std::num::NonZeroU32;

use alm_core::ProductId;
use alm_core::confirmation::{
    ProductText, PurchaseDraft, PurchaseLine, PurchaseSupplier, ReceiveLine, ReceiveRequest,
    SupplierPriceEdit,
};

use super::{ROLE, State, check_remaining, index, order_id, rejected, said, supplier_id};
use crate::{AuthorityError, Intent, Outcome};

pub(super) fn apply(state: &mut State, intent: &Intent) -> Result<Outcome, AuthorityError> {
    match intent {
        Intent::SaveProduct(p) => {
            let id = ProductId::new(p.id);
            let fixture = &mut state.fixture;
            let mut entry = if p.new {
                fixture.begin_product_entry(ROLE, id)
            } else {
                fixture.open_product_entry(ROLE, id)
            }
            .map_err(rejected)?;
            if let Some((buy, sell)) = p.reviewed {
                let pricing = entry.pricing();
                if (pricing.buy(), pricing.sell()) != (buy, sell) {
                    return Err(AuthorityError::StaleReview);
                }
            }
            let reorder = p.reorder.map(|r| r.to_string()).unwrap_or_default();
            let text = ProductText {
                item: &p.item,
                brand: &p.brand,
                buy: &p.buy.to_rupee_text(),
                sell: &p.sell.to_rupee_text(),
                reorder_level: &reorder,
            };
            let failed = |error: alm_core::confirmation::ProductEntryError| {
                error
                    .duplicate_warning_text()
                    .map_or_else(|| rejected(error), said)
            };
            entry.apply_text(&text).map_err(failed)?;
            fixture.save_product_entry(ROLE, &entry).map_err(failed)?;
            Ok(Outcome::Done)
        }
        Intent::DeleteProduct { id } => {
            state
                .fixture
                .delete_product(ROLE, ProductId::new(*id))
                .map_err(rejected)?;
            Ok(Outcome::Done)
        }
        Intent::AddSupplier { name } => {
            let saved = state
                .fixture
                .add_comparison_supplier(ROLE, name)
                .map_err(rejected)?
                .ok_or_else(|| said("Enter a new supplier name."))?;
            Ok(Outcome::Created(index(saved.id.index())?))
        }
        Intent::SetSupplierPrices { product, rates } => {
            for (supplier, rate) in rates {
                let supplier = supplier_id(state, *supplier)?;
                state
                    .fixture
                    .edit_comparison_price(
                        ROLE,
                        SupplierPriceEdit {
                            product: ProductId::new(*product),
                            supplier,
                            rate: *rate,
                        },
                    )
                    .map_err(rejected)?;
            }
            Ok(Outcome::Done)
        }
        Intent::SavePurchaseOrder(po) => {
            let supplier = match &po.supplier {
                Some((id, name)) => Some(PurchaseSupplier {
                    id: id.map(|i| supplier_id(state, i)).transpose()?,
                    name: name.clone(),
                }),
                None => None,
            };
            let items = po
                .lines
                .iter()
                .map(|line| {
                    Ok(PurchaseLine {
                        item: line.item.clone(),
                        product_id: line.product.map(ProductId::new),
                        ordered: NonZeroU32::new(line.ordered).ok_or(AuthorityError::Invalid)?,
                        rate: line.rate,
                    })
                })
                .collect::<Result<_, AuthorityError>>()?;
            let draft = PurchaseDraft {
                supplier,
                date: po.date,
                note: po.note.clone(),
                items,
            };
            let id = state
                .fixture
                .save_purchase_order(ROLE, &draft)
                .map_err(|e| said(e.to_string()))?;
            Ok(Outcome::Created(index(id.index())?))
        }
        Intent::ReceivePurchaseOrder {
            po,
            date,
            today,
            lines,
            remaining,
        } => {
            let id = order_id(state, *po)?;
            check_remaining(state, id, remaining)?;
            let lines: Vec<ReceiveLine> = lines
                .iter()
                .map(|(quantity, rate)| ReceiveLine {
                    quantity: *quantity,
                    rate: *rate,
                })
                .collect();
            let result = state
                .fixture
                .receive_purchase_order(ReceiveRequest {
                    role: ROLE,
                    id,
                    date: *date,
                    today: *today,
                    lines: &lines,
                })
                .map_err(|e| said(e.to_string()))?;
            match result.warning() {
                Some(warning) => Err(said(warning)),
                None => Ok(Outcome::Done),
            }
        }
        Intent::ReceiveGoods {
            po,
            date,
            today,
            lines,
            remaining,
        } => {
            let receive = Intent::ReceivePurchaseOrder {
                po: *po,
                date: *date,
                today: *today,
                lines: lines
                    .iter()
                    .map(|q| (*q, alm_core::Money::from_minor_units(0)))
                    .collect(),
                remaining: remaining.clone(),
            };
            apply(state, &receive)
        }
        Intent::ClosePurchaseOrder { po, remaining } => {
            let id = order_id(state, *po)?;
            check_remaining(state, id, remaining)?;
            state
                .fixture
                .close_remaining(ROLE, id, true)
                .map_err(|e| said(e.to_string()))?;
            Ok(Outcome::Done)
        }
        _ => Err(AuthorityError::Invalid),
    }
}
