use std::time::Duration;

use super::{
    BalanceMode, BillPreview, ConfirmationError, ConfirmationFixture, DraftId, Money, Role,
};
use crate::{BillDate, ProductId};

/// Inputs to the synthetic, read-only estimate actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstimateRequest<'a> {
    pub role: Role,
    pub draft_id: DraftId,
    pub balance_mode: BalanceMode,
    pub display: EstimateDisplay<'a>,
}

/// Explicit synthetic display context, not persisted product metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstimateDisplay<'a> {
    pub today: BillDate,
    pub lines: &'a [EstimateLineDisplay<'a>],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EstimateLineDisplay<'a> {
    pub product: ProductId,
    pub label: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstimateError {
    Preview(ConfirmationError),
    InvalidLineDisplay,
}

/// Describes the legacy print trigger without running it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintIntent {
    OnLoad,
    AfterLoad(Duration),
}

/// Serialized content and intent handed to an inert caller-supplied sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopupDocument {
    pub target: &'static str,
    pub intent: PrintIntent,
    pub html: String,
}

impl ConfirmationFixture {
    pub fn print_estimate_with_business(
        &self,
        request: EstimateRequest<'_>,
        profile: &crate::synthetic_read_model::BusinessProfile,
        sink: &mut impl FnMut(PopupDocument),
    ) -> Result<(), EstimateError> {
        self.emit_estimate(request, (PrintIntent::OnLoad, Some(profile)), sink)
    }
    /// Captures an estimate for the legacy Print action, without opening a window.
    ///
    /// # Errors
    /// Rejects invalid display data or a preview error without invoking the sink.
    pub fn print_estimate(
        &self,
        request: EstimateRequest<'_>,
        sink: &mut impl FnMut(PopupDocument),
    ) -> Result<(), EstimateError> {
        self.emit_estimate(request, (PrintIntent::OnLoad, None), sink)
    }

    /// Captures an estimate for Save as PDF, without creating a file or opening a window.
    ///
    /// # Errors
    /// Rejects invalid display data or a preview error without invoking the sink.
    pub fn save_estimate_as_pdf(
        &self,
        request: EstimateRequest<'_>,
        sink: &mut impl FnMut(PopupDocument),
    ) -> Result<(), EstimateError> {
        self.emit_estimate(
            request,
            (PrintIntent::AfterLoad(Duration::from_millis(350)), None),
            sink,
        )
    }

    fn emit_estimate(
        &self,
        request: EstimateRequest<'_>,
        output: (
            PrintIntent,
            Option<&crate::synthetic_read_model::BusinessProfile>,
        ),
        sink: &mut impl FnMut(PopupDocument),
    ) -> Result<(), EstimateError> {
        let (intent, profile) = output;
        let preview = self.preview_estimate(request)?;
        let target = match intent {
            PrintIntent::OnLoad => "_print",
            PrintIntent::AfterLoad(_) => "_pdf",
        };
        sink(PopupDocument {
            target,
            intent,
            html: estimate_html(&preview, request.display, profile),
        });
        Ok(())
    }

    pub(super) fn preview_estimate(
        &self,
        request: EstimateRequest<'_>,
    ) -> Result<BillPreview, EstimateError> {
        let preview = self
            .bill_preview(request.role, request.draft_id, request.balance_mode)
            .map_err(EstimateError::Preview)?;
        if preview.lines.len() != request.display.lines.len()
            || preview
                .lines
                .iter()
                .zip(request.display.lines)
                .any(|(line, display)| {
                    line.product != display.product || display.label.trim().is_empty()
                })
        {
            return Err(EstimateError::InvalidLineDisplay);
        }
        Ok(preview)
    }
}

fn estimate_html(
    preview: &BillPreview,
    display: EstimateDisplay<'_>,
    profile: Option<&crate::synthetic_read_model::BusinessProfile>,
) -> String {
    let customer = escape_text(&preview.customer);
    let mut html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Estimate-{}-{customer}</title></head><body><main>",
        preview.invoice,
    );
    if let Some(profile) = profile {
        for line in profile.estimate_lines() {
            html.push_str(&format!(
                "<div style=\"font-size:12px;color:#000\">{}</div>",
                escape_text(&line)
            ));
        }
    }
    html.push_str(&format!(
        "<h1>Estimate #{}</h1><p>Date: {}</p><p>Bill To: {customer}</p>",
        preview.invoice,
        display_date(preview.date)
    ));
    if preview.cancelled {
        html.push_str("<p>CANCELLED</p>");
    }
    html.push_str("<table><thead><tr><th>Item</th><th>Quantity</th><th>Rate</th><th>Amount</th></tr></thead><tbody>");
    for (line, display_line) in preview.lines.iter().zip(display.lines) {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_text(display_line.label),
            line.quantity,
            format_money(line.price),
            format_money(line.amount)
        ));
    }
    html.push_str(&format!(
        "</tbody><tfoot><tr><th colspan=\"2\">Items Total</th><td>{}</td><td>{}</td></tr></tfoot></table><table><tbody><tr><th>Loading/Delivery</th><td>{}</td></tr>",
        preview.items_quantity,
        format_money(preview.items_total),
        format_money(preview.loading)
    ));
    html.push_str(&format!(
        "<tr><th>Bill Amount</th><td>{}</td></tr>",
        format_money(preview.total)
    ));
    if let Some(previous) = preview.previous_balance {
        let date = previous.date.map_or_else(String::new, |date| {
            format!(" (as on {})", display_date(date))
        });
        html.push_str(&format!(
            "<tr><th>Prev Balance{date}</th><td>{}</td></tr>",
            format_money(previous.amount)
        ));
    }
    let (paid_label, balance_label) = match preview.balance_mode {
        BalanceMode::AsOnBillDate => ("Paid", String::from("Balance Due")),
        BalanceMode::Latest => (
            "Paid on this bill",
            format!("Total Due (as on {})", display_date(display.today)),
        ),
    };
    html.push_str(&format!(
        "<tr><th>{paid_label}</th><td>{}</td></tr><tr><th>{balance_label}</th><td>{}</td></tr></tbody></table></main></body></html>",
        format_money(preview.paid),
        format_money(preview.balance)
    ));
    html
}

pub(super) fn display_date(date: BillDate) -> String {
    let (year, month, day) = date.parts();
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{day:02}-{}-{year:04}", months[usize::from(month - 1)])
}

pub(super) fn escape_text(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            '\0' => escaped.push('\u{fffd}'),
            other => escaped.push(other),
        }
    }
    escaped
}

fn format_money(amount: Money) -> String {
    let value = amount.minor_units();
    let magnitude = value.unsigned_abs();
    let sign = if value < 0 { "-" } else { "" };
    format!("₹{sign}{}.{:02}", magnitude / 100, magnitude % 100)
}
