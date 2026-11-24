use std::num::{NonZeroU32, NonZeroUsize};

use alm_core::{DraftId, ProductId, ProvisionalDraft};

use crate::{Trial, debug, ipc::IpcId, text, views};

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Intent {
    Select,
    Save,
}

#[derive(serde::Deserialize)]
pub struct Header {
    customer: String,
    date: String,
    intent: Intent,
}

#[derive(serde::Deserialize)]
pub struct Line {
    product: IpcId,
    quantity: String,
}

impl Trial {
    pub(super) fn render_bill(&mut self) -> Result<String, String> {
        let saved_ids = self.saved_ids()?;
        if self.current.is_none() {
            self.fresh_bill(&saved_ids)?;
        }
        let (draft, saved) = self.current.as_ref().ok_or("No draft is open.")?;
        Ok(views::bill_screen(&views::BillView {
            authority: self.drafting(),
            directory: &self.directory,
            draft,
            saved: *saved,
            saved_ids: &saved_ids,
            role: self.role(),
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
        let (draft, saved) = self.current.as_ref().ok_or("No draft is open.")?;
        Ok(views::bill_body(&views::BillView {
            authority: self.drafting(),
            directory: &self.directory,
            draft,
            saved: *saved,
            saved_ids: &[],
            role: self.role(),
        }))
    }

    pub(crate) fn bill_customer_search(&self, q: &str) -> String {
        let found = self.directory.search_customers(self.drafting(), q);
        let options: Vec<_> = found
            .iter()
            .map(|c| [("customer", c.id), ("customer_q", c.name)])
            .collect();
        views::search_options(
            &found
                .iter()
                .zip(&options)
                .map(|(c, fill)| views::SearchOption {
                    fill,
                    label: c.name,
                    sub: "",
                    submit: Some("select"),
                })
                .collect::<Vec<_>>(),
        )
    }

    pub(crate) fn bill_item_search(&self, q: &str) -> String {
        let found = self.directory.search_products(self.drafting(), q);
        let ids: Vec<String> = found.iter().map(|(id, _)| id.to_string()).collect();
        let fills: Vec<_> = found
            .iter()
            .zip(&ids)
            .map(|((_, name), id)| [("product", id.as_str()), ("product_q", *name)])
            .collect();
        views::search_options(
            &found
                .iter()
                .zip(&fills)
                .map(|((_, name), fill)| views::SearchOption {
                    fill,
                    label: name,
                    sub: "",
                    submit: None,
                })
                .collect::<Vec<_>>(),
        )
    }

    pub(super) fn fresh_bill(&mut self, saved_ids: &[u64]) -> Result<(), String> {
        let id = saved_ids
            .last()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Draft limit reached.")?;
        let date = self.today()?;
        let draft = ProvisionalDraft::new(DraftId::new(id), Vec::new()).with_bill_date(Some(date));
        self.current = Some((draft, false));
        self.posting.invalidate();
        Ok(())
    }

    pub(super) fn edit_bill(
        &mut self,
        edit: impl FnOnce(&mut ProvisionalDraft) -> Result<bool, String>,
    ) -> Result<(), String> {
        let (draft, saved) = self.current.as_mut().ok_or("No draft is open.")?;
        if edit(draft)? {
            *saved = false;
            self.posting.invalidate();
        }
        Ok(())
    }

    pub(super) fn bill_header(&mut self, header: Header) -> Result<(), String> {
        let customer = self
            .directory
            .select_customer(self.drafting(), &header.customer)
            .map_err(|_| "Select a connected customer.".to_owned())?;
        let date = header
            .date
            .parse()
            .map_err(|_| "Enter a valid bill date.".to_owned())?;
        let (current, saved) = self.current.as_ref().ok_or("No draft is open.")?;
        let draft = current
            .clone()
            .with_customer(customer)
            .with_bill_date(Some(date));
        let changed = &draft != current;
        let saved = match header.intent {
            Intent::Select => *saved && !changed,
            Intent::Save => {
                if draft.lines().is_empty() {
                    return Err("Add at least one item before saving.".into());
                }
                self.directory
                    .preview(self.drafting(), &draft)
                    .map_err(debug)?;
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

    pub(super) fn bill_add(
        &mut self,
        product: ProductId,
        quantity: NonZeroU32,
    ) -> Result<(), String> {
        let line = self
            .directory
            .select_line(self.drafting(), product, quantity)
            .map_err(|_| "Select a connected product.".to_owned())?;
        self.edit_bill(|draft| {
            draft.add_line(line);
            Ok(true)
        })
    }

    pub(super) fn bill_open(&mut self, id: DraftId) -> Result<(), String> {
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
#[cfg(test)]
mod tests;
