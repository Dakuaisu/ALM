use alm_core::confirmation::{ConfirmationError, ConfirmationFixture, Receipt, RequestId, Review};
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, Money, ProductId, ProvisionalDraft, Role,
};
use tauri::State;

use crate::{AppState, debug, views, with};

pub const CUSTOMER: &str = "Synthetic Customer";
pub const DAY: &str = "2025-04-30";

pub fn customer_id() -> DraftCustomerId {
    DraftCustomerId::new("batch1-customer".into())
}

pub fn fixture() -> Result<ConfirmationFixture, String> {
    let product_id = ProductId::new(1);
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        product_id,
        &[(50, Money::from_minor_units(7_000))],
        Money::from_minor_units(12_050),
    )
    .map_err(debug)?;
    let mut product = fixture
        .open_product_entry(Role::Owner, product_id)
        .map_err(debug)?;
    product.item = "Synthetic Lime".into();
    fixture
        .save_product_entry(Role::Owner, &product)
        .map_err(debug)?;
    fixture
        .register_customer(Role::Owner, &customer_id())
        .map_err(debug)?;
    Ok(fixture)
}

#[derive(serde::Deserialize)]
pub struct Terms {
    pub date: String,
    pub paid: String,
}

pub struct Posting {
    pub review: Review,
    serial: u64,
}

impl Posting {
    pub fn prepare(
        fixture: &mut ConfirmationFixture,
        role: Role,
        draft: &ProvisionalDraft,
        date: &str,
        paid: &str,
        serial: u64,
    ) -> Result<Self, String> {
        if role != Role::Owner {
            return Err(debug(ConfirmationError::AccessDenied));
        }
        if views::customer(draft) != CUSTOMER {
            return Err(format!("Use the named synthetic customer: {CUSTOMER}."));
        }
        let date: BillDate = date.parse().map_err(debug)?;
        let paid = Money::from_minor_units(paid.parse().map_err(debug)?);
        let selected = draft
            .clone()
            .with_customer(DraftCustomer::Selected {
                id: customer_id(),
                name: CUSTOMER.into(),
            })
            .with_bill_date(Some(date));
        fixture.reconnect_review(role, selected).map_err(debug)?;
        let review = fixture
            .review_named(role, draft.id(), paid)
            .map_err(debug)?;
        Ok(Self { review, serial })
    }

    pub const fn serial(&self) -> u64 {
        self.serial
    }

    pub fn confirm(
        &self,
        fixture: &mut ConfirmationFixture,
        role: Role,
        serial: u64,
    ) -> Result<Receipt, ConfirmationError> {
        if serial != self.serial {
            return Err(ConfirmationError::StaleReview);
        }
        fixture.confirm(
            role,
            RequestId::new(self.review.draft().id().value()),
            &self.review,
        )
    }
}

#[tauri::command(async)]
pub fn sale_prepare(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "review", |trial| {
        let role = trial.require_owner()?;
        trial.posting = None;
        let Some((draft, true)) = &trial.current else {
            return Err("Save or reopen the Manager draft first.".into());
        };
        let serial = trial
            .review_serial
            .checked_add(1)
            .ok_or("Review limit reached.")?;
        trial.posting = Some(Posting::prepare(
            &mut trial.authority,
            role,
            draft,
            &terms.date,
            &terms.paid,
            serial,
        )?);
        trial.review_serial = serial;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_confirm(state: State<'_, AppState>, id: u64) -> Result<String, String> {
    with(&state, "sales", |trial| {
        let role = trial.require_owner()?;
        let posting = trial
            .posting
            .as_ref()
            .ok_or("Review the sale before confirming.")?;
        let receipt = posting
            .confirm(&mut trial.authority, role, id)
            .map_err(debug)?;
        trial.message = format!(
            "Bill #{} confirmed in the local simulation.",
            receipt.invoice
        );
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sales_filter(
    state: State<'_, AppState>,
    terms: views::SalesFilter,
) -> Result<String, String> {
    let filter = terms.checked().map_err(debug)?;
    with(&state, "sales", |trial| {
        trial.require_owner()?;
        trial.sales_filter = filter;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_cancel(state: State<'_, AppState>, id: u64) -> Result<String, String> {
    with(&state, "sales", |trial| {
        let role = trial.require_owner()?;
        trial
            .authority
            .cancel_sale(role, alm_core::DraftId::new(id))
            .map_err(debug)
    })
}

#[tauri::command(async)]
pub fn sale_restore(state: State<'_, AppState>, id: u64) -> Result<String, String> {
    with(&state, "sales", |trial| {
        let role = trial.require_owner()?;
        trial
            .authority
            .undo_cancel_sale(role, alm_core::DraftId::new(id))
            .map_err(debug)
    })
}
