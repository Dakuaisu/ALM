//! Crates, openings, godown rows, stock counts and standalone GST bills.
use std::num::NonZeroU32;

use alm_core::confirmation::{
    CrateTemplate, GstBillId, ManualGodownError, ManualGodownInput, OpeningBalance,
    PhysicalCountInput, StockCountReason,
};
use alm_core::{CrateComponent, DraftCustomerId, ProductId, SaleUnit};

use super::{ROLE, State, bill, gst_error, rejected, said};
use crate::{AuthorityError, Intent, Outcome, gst_bill_key, stock_count_reason};

pub(super) fn apply(state: &mut State, intent: &Intent) -> Result<Outcome, AuthorityError> {
    match intent {
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
        Intent::RecordRestore => Ok(Outcome::Done),
        Intent::ImportLegacy(plan) => Ok(Outcome::Created(import_legacy(
            &mut state.legacy,
            &mut state.legacy_sources,
            plan,
        )?)),
        Intent::SetReorderDefault { level } => {
            state
                .fixture
                .save_product_reorder_default(ROLE, *level)
                .map_err(rejected)?;
            Ok(Outcome::Done)
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
        _ => Err(AuthorityError::Invalid),
    }
}

/// The authority's legacy import: the source file must hash to the plan's digest; it is kept
/// by digest and the plan applied (`legacy::import::apply`).
pub(in crate::local) fn import_legacy(
    imports: &mut alm_core::legacy::import::LegacyImports,
    sources: &mut std::collections::BTreeMap<String, std::sync::Arc<str>>,
    plan: &alm_core::legacy::import::ImportPlan,
) -> Result<u64, AuthorityError> {
    alm_core::legacy::import::check_source(plan).map_err(rejected)?;
    let id = alm_core::legacy::import::apply(imports, plan);
    sources
        .entry(plan.digest.clone())
        .or_insert_with(|| plan.source.as_str().into());
    Ok(id)
}
