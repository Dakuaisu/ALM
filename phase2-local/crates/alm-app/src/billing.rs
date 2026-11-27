use crate::ipc::State;
use alm_core::Money;
use alm_core::confirmation::owner_billing::{self, OwnerBillingError, SaleTerms};
use alm_sync::{Intent, Outcome, SaleIntent};

use crate::ipc::{IpcDraft, IpcId};
use crate::{AppState, refusal, views, with};

#[derive(serde::Deserialize)]
pub struct Terms {
    pub date: String,
    pub paid: String,
}

fn posting_error(error: OwnerBillingError) -> String {
    match error {
        OwnerBillingError::Customer => {
            "Select a connected customer on New Bill and save the draft.".into()
        }
        OwnerBillingError::NoSavedDraft => "Save or reopen the Manager draft first.".into(),
        OwnerBillingError::NoReview => "Review the sale before confirming.".into(),
        OwnerBillingError::ReviewLimit => "Review limit reached.".into(),
        OwnerBillingError::Confirmation(error) => refusal(error),
        OwnerBillingError::Date(error) => refusal(error),
    }
}

impl crate::Trial {
    /// Reviews the saved draft on a copy of the authority's state (nothing is registered or
    /// posted) and keeps the reviewed terms as the intent `sale_confirm` sends.
    pub(crate) fn sale_prepare(&mut self, terms: &Terms) -> Result<(), String> {
        let role = self.require_owner()?;
        self.reviewed = None;
        let saved = self
            .current
            .as_ref()
            .and_then(|(draft, saved)| saved.then_some(draft));
        let mut scratch = self.posted.fixture.clone();
        self.posting
            .prepare(
                &mut scratch,
                &self.directory,
                role,
                saved,
                SaleTerms {
                    date: &terms.date,
                    paid: &terms.paid,
                },
            )
            .map_err(posting_error)?;
        let draft = self
            .posting
            .pending(role)
            .ok_or("Review the sale before confirming.")?
            .review()
            .draft()
            .clone();
        let prices = owner_billing::priced_draft(&scratch, role, &draft)
            .map_err(refusal)?
            .lines
            .iter()
            .map(|line| line.unit_price)
            .collect();
        self.reviewed = Some(SaleIntent {
            date: draft.bill_date().ok_or("Enter a valid bill date.")?,
            paid: Money::parse_rupees(&terms.paid).map_err(refusal)?,
            draft,
            prices,
        });
        Ok(())
    }

    /// Sends the reviewed sale through the outbox; `serial` must be the review on screen.
    pub(crate) fn sale_confirm(&mut self, serial: u64) -> Result<u64, String> {
        let role = self.require_owner()?;
        if self.posting.pending(role).map(|p| p.serial()) != Some(serial) {
            return Err(posting_error(OwnerBillingError::Confirmation(
                alm_core::confirmation::ConfirmationError::StaleReview,
            )));
        }
        let sale = self
            .reviewed
            .clone()
            .ok_or("Review the sale before confirming.")?;
        let outcome = self.send(&Intent::PostSale(sale))?;
        self.posting.invalidate();
        self.reviewed = None;
        match outcome {
            Outcome::Sale(receipt) => Ok(receipt.invoice),
            Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_) => {
                Err("Unexpected outcome.".into())
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Post {
    #[serde(default)]
    pub paid: String,
}

impl crate::Trial {
    /// DEC-034: posts the saved bill (a Manager's, when online) through the same idempotent
    /// `post_sale` path as the Owner's review: each line at the selling rate this device holds,
    /// each crate row at its copied rate. The authority re-checks prices, stock and terms; stale
    /// terms send the bill back for review. Only this explicit action posts a draft.
    pub(crate) fn bill_post(&mut self, paid: &str) -> Result<u64, String> {
        let role = self.require(alm_core::permissions::Capability::PostSale)?;
        let (draft, saved) = self.current.clone().ok_or("No draft is open.")?;
        if !saved {
            return Err("Save the bill as a draft first.".into());
        }
        let prices = owner_billing::priced_draft(&self.posted, role, &draft)
            .map_err(refusal)?
            .lines
            .iter()
            .map(|line| line.unit_price)
            .collect();
        let paid = Money::parse_rupees(if paid.trim().is_empty() {
            "0"
        } else {
            paid.trim()
        })
        .map_err(|_| "Enter the amount paid in rupees.")?;
        let sale = SaleIntent {
            date: draft.bill_date().ok_or("Enter a valid bill date.")?,
            paid,
            draft,
            prices,
        };
        match self.send(&Intent::PostSale(sale)) {
            Ok(Outcome::Sale(receipt)) => {
                self.current = None;
                self.message = format!("Bill #{} posted ✓", receipt.invoice);
                Ok(receipt.invoice)
            }
            Ok(Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_)) => {
                Err("Unexpected outcome.".into())
            }
            Err(error)
                if error == crate::device::refused(&alm_sync::AuthorityError::StaleReview) =>
            {
                self.refresh()?;
                Err(format!("{error} The bill shows the current terms."))
            }
            Err(error) => Err(error),
        }
    }
}

#[tauri::command(async)]
pub fn bill_post(state: State<'_, AppState>, terms: Post) -> Result<String, String> {
    with(&state, "bill", |trial| {
        trial.bill_post(&terms.paid).map(drop)
    })
}

#[tauri::command(async)]
pub fn sale_prepare(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "review", |trial| trial.sale_prepare(&terms))
}

#[tauri::command(async)]
pub fn sale_confirm(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        let invoice = trial.sale_confirm(id)?;
        trial.message = format!("Bill #{invoice} confirmed {}.", trial.authority_name());
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sales_filter(
    state: State<'_, AppState>,
    terms: views::SalesFilter,
) -> Result<String, String> {
    let filter = terms.checked().map_err(refusal)?;
    with(&state, "sales", |trial| {
        trial.require(alm_core::permissions::Capability::Sales)?;
        trial.sales_filter = filter;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_cancel(state: State<'_, AppState>, id: IpcDraft) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        trial.send(&Intent::CancelSale { draft: id }).map(drop)
    })
}

impl crate::Trial {
    pub(crate) fn sale_edit_save(
        &mut self,
        mut terms: std::collections::HashMap<String, String>,
    ) -> Result<(), String> {
        let role = self.require(alm_core::permissions::Capability::EditSale)?;
        let id = terms
            .remove("id")
            .and_then(|text| alm_core::DraftId::parse(&text).ok())
            .ok_or("Unknown bill.")?;
        let draft = self
            .posted
            .current_sale(role, id)
            .map_err(refusal)?
            .0
            .draft();
        // Ordinary lines, then each crate line (crates and rate).
        let lines = draft.lines().len() + draft.crate_rows().len();
        let edits = (0..lines)
            .map(|i| {
                let qty = terms.remove(&format!("qty_{i}")).unwrap_or_default();
                let rate = terms.remove(&format!("rate_{i}")).unwrap_or_default();
                alm_core::confirmation::LineEdit::from_text(&qty, &rate)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                "Each line needs a whole quantity above zero and a rate in rupees.".to_owned()
            })?;
        let lines = edits
            .iter()
            .map(|edit| (edit.quantity.get(), edit.price.minor_units()))
            .collect();
        let outcome = self
            .send(&Intent::EditSale { draft: id, lines })
            .map_err(|error| {
                if error == crate::refusals::ENTER_AMOUNT {
                    "The new total is below the amount already paid.".to_owned()
                } else {
                    error
                }
            })?;
        self.sale_edit = None;
        if let Outcome::Sale(receipt) = outcome {
            self.message = format!("Bill #{} saved ✓", receipt.invoice);
        }
        Ok(())
    }
}

#[tauri::command(async)]
pub fn sale_edit_open(state: State<'_, AppState>, id: IpcDraft) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        let role = trial.require(alm_core::permissions::Capability::EditSale)?;
        trial.posted.current_sale(role, id).map_err(refusal)?;
        trial.sale_edit = Some(id);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_edit_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "sales", |trial| {
        trial.sale_edit = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_edit_save(
    state: State<'_, AppState>,
    terms: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "sales", |trial| trial.sale_edit_save(terms))
}

#[tauri::command(async)]
pub fn sale_restore(state: State<'_, AppState>, id: IpcDraft) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        trial.send(&Intent::RestoreSale { draft: id }).map(drop)
    })
}
