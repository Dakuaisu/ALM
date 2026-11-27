//! The activity log: the HTML's `logAct` entries the simulated authority records for the
//! commands it commits, with the time, device and account of the request.
use alm_core::Role;
use alm_core::projection::book::{ActivityRow, OwnerBook, indian_number};

use super::State;
use crate::{Intent, Outcome};

fn book(state: &State) -> Option<OwnerBook> {
    state.fixture.owner_book(Role::Owner, &state.cashbook).ok()
}

/// A bill's `#no customer` and total, and whether it is cancelled (read from the bill itself,
/// not by building the whole book).
fn bill(state: &State, draft: alm_core::DraftId) -> Option<(String, alm_core::Money, bool)> {
    let (invoice, name, total, cancelled) = state.fixture.sale_summary(Role::Owner, draft).ok()?;
    Some((format!("#{invoice} {name}"), total, cancelled))
}

fn gst(book: &OwnerBook, id: u64) -> Option<(String, bool)> {
    let bill = book.gst_bills.iter().find(|b| b.id.0 == id)?;
    Some((
        format!("GST {} {}", bill.number.0, bill.customer),
        bill.cancelled,
    ))
}

/// The Stock adjust dialog's reason labels by intent code.
pub(super) fn reason_label(code: &str) -> &'static str {
    match code {
        "count" => "Count correction",
        "damage" => "Damage / leakage",
        "expired" => "Expired",
        "theft" => "Theft / unknown",
        "own-use" => "Own use / sample",
        _ => "Other",
    }
}

/// `(kind, subject, amount, detail)` of the entry `intent` records, if any.
#[allow(clippy::too_many_lines)]
fn entry(
    before: &State,
    after: &State,
    directory: &alm_core::confirmation::owner_billing::BillingDirectory,
    intent: &Intent,
    outcome: &Outcome,
) -> Option<(&'static str, String, Option<alm_core::Money>, String)> {
    let none = String::new;
    Some(match (intent, outcome) {
        (Intent::PostSale(_) | Intent::SellCrate(_), Outcome::Sale(r)) => {
            let (subject, ..) = bill(after, r.draft)?;
            ("bill-save", subject, Some(r.total), none())
        }
        (Intent::EditSale { draft, .. }, Outcome::Sale(r)) => {
            let (subject, ..) = bill(after, *draft)?;
            ("bill-edit", subject, Some(r.total), none())
        }
        (Intent::CancelSale { draft }, _) => {
            let (_, _, was) = bill(before, *draft)?;
            let (subject, total, now) = bill(after, *draft)?;
            if was || !now {
                return None;
            }
            ("bill-cancel", subject, Some(total), none())
        }
        (Intent::RestoreSale { draft }, _) => {
            let (_, _, was) = bill(before, *draft)?;
            let (subject, _, now) = bill(after, *draft)?;
            if !was || now {
                return None;
            }
            ("bill-restore", subject, None, none())
        }
        (
            Intent::RecordReceipt {
                customer,
                amount,
                online,
                ..
            },
            _,
        ) => {
            let name = directory
                .customers
                .iter()
                .find(|c| c.id == *customer)
                .map_or(customer.as_str(), |c| c.name.as_str());
            (
                "payment",
                name.to_owned(),
                Some(*amount),
                if *online { "Online" } else { "Cash" }.to_owned(),
            )
        }
        (Intent::ReceivePurchaseOrder { po, .. }, _) => {
            let after = book(after)?;
            let order = after.purchasing.orders.iter().find(|o| o.id == *po)?;
            (
                "grn",
                format!("PO-{:03} {}", po + 1, order.supplier_name),
                None,
                none(),
            )
        }
        (Intent::DeleteProduct { id }, _) => {
            let before = book(before)?;
            let product = before.products.iter().find(|p| p.id == *id)?;
            (
                "prod-delete",
                product.details.as_ref()?.item.clone(),
                None,
                none(),
            )
        }
        (Intent::SaveCrate(c), _) => (
            if c.id.is_some() {
                "crate-edit"
            } else {
                "crate-save"
            },
            c.name.trim().to_owned(),
            None,
            none(),
        ),
        (Intent::RecordRestore, _) => ("restore", "Backup file restored".into(), None, none()),
        (Intent::DeleteCrate { id }, _) => {
            let before = book(before)?;
            let template = before.crates.templates.iter().find(|t| t.id == *id)?;
            ("crate-delete", template.name.clone(), None, none())
        }
        (
            Intent::CountStock {
                product,
                counted,
                on_hand,
                reason,
                ..
            },
            _,
        ) => {
            if counted == on_hand {
                return None;
            }
            let after = book(after)?;
            let item = after
                .products
                .iter()
                .find(|p| p.id == *product)?
                .details
                .as_ref()?
                .item
                .clone();
            let moved = format!(
                "{} → {}",
                indian_number(i64::from(*on_hand)),
                indian_number(i64::from(*counted))
            );
            let detail = match reason {
                Some(code) => format!("{moved} ({})", reason_label(code)),
                None => moved,
            };
            ("adjust", item, None, detail)
        }
        (Intent::SaveGstBill { bill: text }, _) => {
            let id = alm_core::confirmation::GstBill::from_record_text(text)
                .ok()?
                .id
                .0;
            let existed = gst(&book(before)?, id).is_some();
            let (subject, _) = gst(&book(after)?, id)?;
            (
                if existed { "gst-edit" } else { "gst-save" },
                subject,
                None,
                none(),
            )
        }
        (Intent::CancelGstBill { id }, _) => {
            let (subject, was) = gst(&book(before)?, *id)?;
            if was {
                return None;
            }
            ("gst-cancel", subject, None, none())
        }
        (Intent::DeleteGstBill { id }, _) => {
            let (subject, _) = gst(&book(before)?, *id)?;
            ("gst-delete", subject, None, none())
        }
        _ => return None,
    })
}

/// The entry for a committed `intent`, stamped with the request's time, device and account.
pub(super) fn record(
    before: &State,
    after: &State,
    directory: &alm_core::confirmation::owner_billing::BillingDirectory,
    intent: &Intent,
    outcome: &Outcome,
    (at_ms, device, account): (u64, &str, &str),
) -> Option<ActivityRow> {
    let (kind, subject, amount, detail) = entry(before, after, directory, intent, outcome)?;
    Some(ActivityRow {
        at_ms,
        device: device.to_owned(),
        account: account.to_owned(),
        kind: kind.to_owned(),
        subject,
        amount,
        detail,
    })
}
