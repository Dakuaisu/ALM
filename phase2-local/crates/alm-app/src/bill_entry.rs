use std::num::{NonZeroU32, NonZeroUsize};

use alm_core::confirmation::owner_billing::BillingDirectory;
use alm_core::{DraftId, ProductId, ProvisionalDraft};

use crate::{Trial, refusal, text, views};

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Intent {
    Select,
    Save,
}

#[derive(serde::Deserialize)]
pub struct Header {
    customer: String,
    date: String,
    /// "Loading/Delivery ₹ per packet"; absent leaves the draft's rate.
    #[serde(default)]
    loading: Option<String>,
    intent: Intent,
}

#[cfg(test)]
pub(crate) use Intent as HeaderIntent;

#[cfg(test)]
impl Header {
    pub(crate) fn new(customer: &str, date: &str, loading: Option<&str>, intent: Intent) -> Self {
        Self {
            customer: customer.into(),
            date: date.into(),
            loading: loading.map(str::to_owned),
            intent,
        }
    }
}

/// The item row's form: the picked product's ID (blank until an item is picked) and quantity.
#[derive(serde::Deserialize)]
pub struct Line {
    #[serde(default)]
    product: String,
    #[serde(default)]
    quantity: String,
}

impl Line {
    /// The picked product and its quantity; an empty row is refused in the HTML's words.
    pub(crate) fn picked(&self) -> Result<(ProductId, NonZeroU32), String> {
        let product = self
            .product
            .trim()
            .parse()
            .map(ProductId::new)
            .map_err(|_| crate::refusals::NO_ITEM.to_owned())?;
        Ok((product, quantity(&self.quantity)?))
    }
}

impl Trial {
    pub(super) fn render_bill(&mut self) -> Result<String, String> {
        let saved_ids = self.saved_ids()?;
        if self.current.is_none() {
            self.fresh_bill(&saved_ids)?;
        }
        let crates = self.bill_crates();
        let catalog = self.manager_catalog();
        let (draft, saved) = self.current.as_ref().ok_or("No draft is open.")?;
        Ok(views::bill_screen(&views::BillView {
            authority: self.drafting(),
            directory: &self.directory,
            draft,
            saved: *saved,
            saved_ids: &saved_ids,
            role: self.role(),
            crates: &crates,
            catalog: catalog.as_deref(),
        }))
    }

    /// Live quantity edit: applies the quantity like `bill_quantity` and returns `#bill-body`.
    pub(crate) fn bill_live(&mut self, line: &str, qty: &str) -> Result<String, String> {
        let line = line
            .parse::<usize>()
            .ok()
            .and_then(NonZeroUsize::new)
            .ok_or("Unknown line.")?;
        let qty = quantity(qty)?;
        self.edit_bill(|draft| {
            let previous = draft
                .replace_line_quantity(line, qty)
                .ok_or("Unknown line.")?;
            Ok(previous != qty)
        })?;
        let crates = self.bill_crates();
        let catalog = self.manager_catalog();
        let (draft, saved) = self.current.as_ref().ok_or("No draft is open.")?;
        Ok(views::bill_body(&views::BillView {
            authority: self.drafting(),
            directory: &self.directory,
            draft,
            saved: *saved,
            saved_ids: &[],
            role: self.role(),
            crates: &crates,
            catalog: catalog.as_deref(),
        }))
    }
    pub(super) fn fresh_bill(&mut self, saved_ids: &[DraftId]) -> Result<(), String> {
        let next = saved_ids
            .iter()
            .map(|id| id.value())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Draft limit reached.")?;
        let id = DraftId::scoped(&self.draft_origin(), next).map_err(refusal)?;
        let date = self.today()?;
        // `newDraft`: loadingRate starts at the business default (Settings, Owner only).
        let loading = match self.role() {
            alm_core::Role::Owner => self
                .shop
                .business_profile(alm_core::Role::Owner)
                .map_err(refusal)?
                .loading_charge(),
            alm_core::Role::Manager => alm_core::Money::from_minor_units(0),
        };
        let draft = ProvisionalDraft::new(id, Vec::new())
            .with_bill_date(Some(date))
            .with_loading_rate(loading);
        self.current = Some((draft, false));
        self.posting.invalidate();
        Ok(())
    }

    pub(crate) fn edit_bill(
        &mut self,
        edit: impl FnOnce(&mut ProvisionalDraft) -> Result<bool, String>,
    ) -> Result<(), String> {
        if self.revoked {
            return Err(crate::trial::REVOKED.into());
        }
        let (draft, saved) = self.current.as_mut().ok_or("No draft is open.")?;
        if edit(draft)? {
            *saved = false;
            self.posting.invalidate();
        }
        Ok(())
    }

    pub(crate) fn bill_header(&mut self, header: Header) -> Result<(), String> {
        if self.revoked {
            return Err(crate::trial::REVOKED.into());
        }
        let customer = match self.manager_customers() {
            Some(names) => BillingDirectory::select_listed_customer(&names, &header.customer),
            None => self
                .directory
                .select_customer(self.drafting(), &header.customer),
        }
        .map_err(|_| "Select a connected customer.".to_owned())?;
        let date = header
            .date
            .parse()
            .map_err(|_| "Enter a valid bill date.".to_owned())?;
        let (current, saved) = self.current.as_ref().ok_or("No draft is open.")?;
        let loading = match header.loading.as_deref().map(str::trim) {
            None => current.loading_rate(),
            Some("") => alm_core::Money::from_minor_units(0),
            Some(text) => alm_core::Money::parse_rupees(text)
                .map_err(|_| "Enter the loading charge in rupees per packet.".to_owned())?,
        };
        let draft = current
            .clone()
            .with_customer(customer)
            .with_bill_date(Some(date))
            .with_loading_rate(loading);
        let changed = &draft != current;
        let saved = match header.intent {
            Intent::Select => *saved && !changed,
            Intent::Save => {
                if draft.lines().is_empty() && draft.crate_rows().is_empty() {
                    return Err("Add at least one item before saving.".into());
                }
                if !draft.lines().is_empty() {
                    let catalog = self.manager_catalog();
                    self.directory
                        .new_bill_pricing(self.drafting(), catalog.as_deref(), &draft)
                        .map_err(|error| crate::refusals::pricing(&error))?;
                }
                self.drafts.save_draft(draft.clone()).map_err(text)?;
                self.message = format!(
                    "Saved draft {}: PROVISIONAL, NOT POSTED.",
                    draft.id().value()
                );
                true
            }
        };
        self.current = Some((draft, saved));
        if changed {
            self.posting.invalidate();
        }
        Ok(())
    }

    pub(crate) fn bill_add(
        &mut self,
        product: ProductId,
        quantity: NonZeroU32,
    ) -> Result<(), String> {
        let line = match self.manager_catalog() {
            Some(catalog) => {
                alm_core::confirmation::owner_billing::BillingDirectory::select_catalog_line(
                    &catalog, product, quantity,
                )
            }
            None => self
                .directory
                .select_line(self.drafting(), product, quantity),
        }
        .map_err(|_| "Select a connected product.".to_owned())?;
        self.edit_bill(|draft| {
            draft.add_line(line);
            Ok(true)
        })
    }

    pub(crate) fn bill_open(&mut self, id: DraftId) -> Result<(), String> {
        let draft = self
            .drafts
            .reopen_draft(id)
            .map_err(text)?
            .ok_or("Draft not found.")?;
        self.current = Some((draft, true));
        self.posting.invalidate();
        Ok(())
    }
}

pub(super) fn quantity(raw: &str) -> Result<NonZeroU32, String> {
    raw.trim()
        .parse()
        .map_err(|_| "Quantity must be a positive whole number.".to_owned())
}

#[cfg(test)]
mod batch1;
#[cfg(test)]
mod batch1_edit;
pub mod commands;
pub mod crate_row;
mod search;
#[cfg(test)]
mod tests;
