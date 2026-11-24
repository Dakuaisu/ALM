//! The simulation's catalog, purchasing, stock, opening, godown and GST commands: each runs
//! the same core rule the screens used to call directly, on the state copy `commit` rolls back.
use std::num::NonZeroU32;

use alm_core::confirmation::{
    CrateTemplate, GstBill, GstBillId, ManualGodownError, ManualGodownInput, OpeningBalance,
    PhysicalCountInput, ProductText, PurchaseDraft, PurchaseLine, PurchaseOrderId,
    PurchaseSupplier, PurchaseSupplierId, ReceiveLine, ReceiveRequest, StockCountReason,
    SupplierPriceEdit,
};
use alm_core::{CrateComponent, DraftCustomerId, ProductId, Role, SaleUnit};

use super::{State, rejected};
use crate::{AuthorityError, Intent, Outcome, gst_bill_key, stock_count_reason};

const ROLE: Role = Role::Owner;

fn said(text: impl Into<String>) -> AuthorityError {
    AuthorityError::Rejected(text.into())
}

fn order_id(state: &State, index: u64) -> Result<PurchaseOrderId, AuthorityError> {
    state
        .fixture
        .purchase_orders(ROLE)
        .map_err(rejected)?
        .iter()
        .map(|order| order.id)
        .find(|id| u64::try_from(id.index()).is_ok_and(|i| i == index))
        .ok_or_else(|| said("Purchase order not found"))
}

fn supplier_id(state: &State, index: u64) -> Result<PurchaseSupplierId, AuthorityError> {
    state
        .fixture
        .purchase_suppliers(ROLE)
        .map_err(rejected)?
        .iter()
        .map(|supplier| supplier.id)
        .find(|id| u64::try_from(id.index()).is_ok_and(|i| i == index))
        .ok_or_else(|| said("Unknown supplier"))
}

/// Rejects as stale when a PO's remaining quantities differ from the reviewed ones.
fn check_remaining(
    state: &State,
    id: PurchaseOrderId,
    remaining: &[u32],
) -> Result<(), AuthorityError> {
    let order = state
        .fixture
        .purchase_orders(ROLE)
        .map_err(rejected)?
        .iter()
        .find(|order| order.id == id)
        .ok_or_else(|| said("Purchase order not found"))?;
    let current: Vec<u32> = order
        .items
        .iter()
        .map(alm_core::confirmation::SavedPurchaseLine::remaining)
        .collect();
    if current == remaining {
        Ok(())
    } else {
        Err(AuthorityError::StaleReview)
    }
}

fn index(value: usize) -> Result<u64, AuthorityError> {
    u64::try_from(value).map_err(|_| AuthorityError::Invalid)
}

fn bill(text: &str) -> Result<GstBill, AuthorityError> {
    GstBill::from_record_text(text).map_err(|_| AuthorityError::Invalid)
}

fn gst_error(error: alm_core::confirmation::GstBillError) -> AuthorityError {
    error.warning().map_or_else(|| rejected(error), said)
}

#[allow(clippy::too_many_lines)]
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
        Intent::ClosePurchaseOrder { po, remaining } => {
            let id = order_id(state, *po)?;
            check_remaining(state, id, remaining)?;
            state
                .fixture
                .close_remaining(ROLE, id, true)
                .map_err(|e| said(e.to_string()))?;
            Ok(Outcome::Done)
        }
        Intent::SaveCrate(c) => {
            let components = c
                .components
                .iter()
                .map(|(product, quantity, piece)| {
                    Some(CrateComponent::new(
                        ProductId::new(*product),
                        NonZeroU32::new(*quantity)?,
                        if *piece {
                            SaleUnit::Piece
                        } else {
                            SaleUnit::Case
                        },
                    ))
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(AuthorityError::Invalid)?;
            let template = CrateTemplate {
                name: c.name.trim().to_owned(),
                components,
                price: c.price,
            };
            let fixture = &mut state.fixture;
            match c.id {
                Some(value) => {
                    let id = fixture.crate_template_id(ROLE, value).map_err(rejected)?;
                    fixture
                        .edit_crate_template(ROLE, id, template)
                        .map_err(rejected)?;
                    Ok(Outcome::Created(value))
                }
                None => {
                    let id = fixture
                        .save_crate_template(ROLE, template)
                        .map_err(rejected)?;
                    Ok(Outcome::Created(id.value()))
                }
            }
        }
        Intent::DeleteCrate { id } => {
            let fixture = &mut state.fixture;
            let id = fixture.crate_template_id(ROLE, *id).map_err(rejected)?;
            fixture.delete_crate_template(ROLE, id).map_err(rejected)?;
            Ok(Outcome::Done)
        }
        Intent::SaveOpening {
            customer,
            amount,
            date,
        } => {
            state
                .fixture
                .save_customer_opening(
                    ROLE,
                    &DraftCustomerId::new(customer.clone()),
                    OpeningBalance {
                        amount: *amount,
                        date: *date,
                    },
                )
                .map_err(rejected)?;
            Ok(Outcome::Done)
        }
        Intent::SaveGodown { item, cases, note } => {
            state
                .fixture
                .save_manual_godown(ROLE, ManualGodownInput { item, cases, note })
                .map_err(|error| match error {
                    ManualGodownError::ItemNameRequired => said("Type an item name"),
                    ManualGodownError::InvalidCases => said("Enter quantity (whole cases)"),
                    ManualGodownError::AccessDenied => AuthorityError::Denied,
                })?;
            Ok(Outcome::Done)
        }
        Intent::CountStock {
            product,
            counted,
            fridge,
            date,
            reason,
            note,
            on_hand,
        } => {
            let product = ProductId::new(*product);
            let current = state
                .fixture
                .open_physical_count(ROLE, product)
                .map_err(rejected)?
                .on_hand;
            if current != *on_hand {
                return Err(AuthorityError::StaleReview);
            }
            let reason: Option<StockCountReason> = match reason {
                Some(code) => Some(stock_count_reason(code).ok_or(AuthorityError::Invalid)?),
                None => None,
            };
            let counted = counted.to_string();
            let fridge = fridge.map(|f| f.to_string());
            state
                .fixture
                .submit_physical_count(
                    ROLE,
                    PhysicalCountInput {
                        product,
                        counted: &counted,
                        fridge: fridge.as_deref(),
                        date: *date,
                        reason,
                        note: note.as_deref(),
                    },
                )
                .map_err(rejected)?;
            Ok(Outcome::Done)
        }
        Intent::SaveGstBill { bill: text } => {
            let bill = bill(text)?;
            let ledger = state.fixture.gst_bill_ledger(ROLE).map_err(rejected)?;
            let expected = match ledger.bills.iter().find(|b| b.id == bill.id) {
                Some(saved) => Some(saved.number.0),
                None => state.gst.get(&gst_bill_key(bill.id.0)).copied(),
            };
            if expected != Some(bill.number.0) {
                return Err(AuthorityError::StaleReview);
            }
            let number = bill.number.0;
            state
                .fixture
                .put_gst_bill_record(ROLE, bill)
                .map_err(gst_error)?;
            Ok(Outcome::GstNumber(number))
        }
        Intent::CancelGstBill { id } => {
            state
                .fixture
                .cancel_gst_bill(ROLE, GstBillId(*id))
                .map_err(gst_error)?;
            Ok(Outcome::Done)
        }
        Intent::DeleteGstBill { id } => {
            state
                .fixture
                .delete_gst_bill(ROLE, GstBillId(*id))
                .map_err(gst_error)?;
            // Deleting the highest-numbered bill frees its number again, as in the HTML.
            state.gst.remove(&gst_bill_key(*id));
            Ok(Outcome::Done)
        }
        Intent::ImportGstBill {
            bill: text,
            deleted,
        } => {
            let bill = text.as_deref().map(bill).transpose()?;
            let number = bill.as_ref().map(|b| (b.id.0, b.number.0));
            state
                .fixture
                .import_gst_bill_record(ROLE, bill, deleted.map(GstBillId))
                .map_err(gst_error)?;
            if let Some((id, number)) = number {
                state.gst.insert(gst_bill_key(id), number);
            }
            Ok(Outcome::Done)
        }
        Intent::PostSale(_)
        | Intent::CancelSale { .. }
        | Intent::RestoreSale { .. }
        | Intent::EditSale { .. }
        | Intent::RecordReceipt { .. }
        | Intent::SaveCashDay { .. }
        | Intent::DeleteCashDay { .. } => Err(AuthorityError::Invalid),
    }
}
