use super::estimate::display_date;
use super::{
    BalanceMode, BillPreview, ConfirmationError, ConfirmationFixture, EstimateDisplay,
    EstimateError, EstimateRequest, Money, Role,
};
use crate::{DraftCustomer, DraftId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageHandoffRequest<'a> {
    pub role: Role,
    pub draft_id: DraftId,
    pub business_name: &'a str,
    /// Synthetic customer ID and mobile, looked up against the posted bill.
    pub saved_customer_mobile: Option<(&'a str, &'a str)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageFile {
    pub name: String,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShareRejected;

/// Inputs for the image-to-text fallback. Balance mode reflects the current preview setting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvoiceHandoffRequest<'a> {
    pub image: ImageHandoffRequest<'a>,
    pub tagline: &'a str,
    pub balance_mode: BalanceMode,
    pub display: EstimateDisplay<'a>,
}

/// Lazy synthetic render results; `None` represents a failed render.
pub struct InvoiceImageAttempts<Html, Canvas> {
    pub html: Html,
    pub canvas: Canvas,
}

/// Captured text URL and window target, without navigation or sending.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextHandoffIntent {
    pub url: String,
    pub target: &'static str,
}

impl ConfirmationFixture {
    /// Tries HTML, then canvas, then captures a text intent for a regular packet bill.
    ///
    /// # Errors
    /// Denies Managers and rejects missing sales before any callback. Invalid preview or
    /// display data prevents text emission but does not affect the image path.
    pub fn handoff_invoice<'a>(
        &self,
        request: InvoiceHandoffRequest<'_>,
        attempts: InvoiceImageAttempts<
            impl FnOnce() -> Option<&'a [u8]>,
            impl FnOnce() -> Option<&'a [u8]>,
        >,
        sinks: &mut impl InvoiceHandoff,
    ) -> Result<(), EstimateError> {
        let image = request.image;
        if image.role != Role::Owner {
            return Err(EstimateError::Preview(ConfirmationError::AccessDenied));
        }
        let (review, _, _) = self
            .current_sales
            .get(&image.draft_id)
            .ok_or(EstimateError::Preview(ConfirmationError::DraftNotFound))?;

        if let Some(png) = (attempts.html)().or_else(attempts.canvas) {
            return self
                .handoff_invoice_image(image, png, sinks)
                .map_err(EstimateError::Preview);
        }

        let preview = self.preview_estimate(EstimateRequest {
            role: image.role,
            draft_id: image.draft_id,
            balance_mode: request.balance_mode,
            display: request.display,
        })?;
        let phone = saved_phone(review.draft.customer(), image.saved_customer_mobile);
        let mut url = format!("https://wa.me/{}?text=", phone.as_deref().unwrap_or(""));
        append_uri_component(&mut url, &invoice_text(&preview, request));
        sinks.text(TextHandoffIntent {
            url,
            target: "_blank",
        });
        Ok(())
    }

    /// Routes ready PNG bytes to inert sinks; rendering is outside this boundary.
    pub fn handoff_invoice_image(
        &self,
        request: ImageHandoffRequest<'_>,
        png: &[u8],
        sinks: &mut impl ImageHandoff,
    ) -> Result<(), ConfirmationError> {
        if request.role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (review, receipt, _) = self
            .current_sales
            .get(&request.draft_id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        let invoice = receipt.invoice;
        let file = ImageFile {
            name: format!("Estimate-{invoice}.png"),
            mime: "image/png",
            bytes: png.to_vec(),
        };
        if sinks.can_share(&file) {
            let title = format!("Estimate #{invoice}");
            let text = format!("{} — {title}", request.business_name);
            // Like waInvoice, neither native result enters the download fallback.
            return match sinks.share(&file, &title, &text) {
                Ok(()) | Err(ShareRejected) => Ok(()),
            };
        }
        sinks.download(&file);
        if let Some(phone) = saved_phone(review.draft.customer(), request.saved_customer_mobile) {
            sinks.chat(&phone);
        }
        Ok(())
    }
}

pub trait ImageHandoff {
    fn can_share(&mut self, file: &ImageFile) -> bool;
    fn share(&mut self, file: &ImageFile, title: &str, text: &str) -> Result<(), ShareRejected>;
    fn download(&mut self, file: &ImageFile);
    /// Records a chat target without opening a URL or attaching the file.
    fn chat(&mut self, phone: &str);
}

pub trait InvoiceHandoff: ImageHandoff {
    /// Records only a text navigation intent when neither image render succeeds.
    fn text(&mut self, intent: TextHandoffIntent);
}

fn saved_phone(customer: &DraftCustomer, saved_mobile: Option<(&str, &str)>) -> Option<String> {
    let (saved_id, mobile) = saved_mobile?;
    match customer {
        DraftCustomer::Selected { id, .. } if id.value() == saved_id => clean_phone(mobile),
        DraftCustomer::Selected { .. } | DraftCustomer::Typed(_) | DraftCustomer::Unassigned => {
            None
        }
    }
}

pub(super) fn clean_phone(mobile: &str) -> Option<String> {
    let digits: String = mobile.chars().filter(char::is_ascii_digit).collect();
    (digits.len() >= 10).then(|| format!("91{}", &digits[digits.len() - 10..]))
}

fn invoice_text(preview: &BillPreview, request: InvoiceHandoffRequest<'_>) -> String {
    const DIVIDER: &str = "----------------------";
    let mut text = format!(
        "*{}*\n{}\n{DIVIDER}\n*Estimate #{}*  ({})\nParty: {}\n\n",
        request.image.business_name,
        request.tagline,
        preview.invoice,
        display_date(preview.date),
        preview.customer,
    );
    for (line, display) in preview.lines.iter().zip(request.display.lines) {
        text.push_str(&format!(
            "{}  ×{} = ₹{}\n",
            display.label,
            group_indian_digits(u64::from(line.quantity.get())),
            format_amount(line.amount),
        ));
    }
    let (paid_label, balance_label) = match preview.balance_mode {
        BalanceMode::AsOnBillDate => ("Paid", String::from("Balance Due")),
        BalanceMode::Latest => (
            "Paid on this bill",
            format!("Total Due (as on {})", display_date(request.display.today)),
        ),
    };
    text.push_str(&format!(
        "Loading/Delivery: ₹{}\n{DIVIDER}\n*Bill Amount: ₹{}*\n{paid_label}: ₹{}\n",
        format_amount(preview.loading),
        format_amount(preview.total),
        format_amount(preview.paid),
    ));
    if let Some(previous) = preview.previous_balance {
        text.push_str(&format!(
            "Old Balance: ₹{}\n",
            format_amount(previous.amount)
        ));
    }
    text.push_str(&format!(
        "*{balance_label}: ₹{}*\n\nThank you! 🙏",
        format_amount(preview.balance),
    ));
    text
}

pub(super) fn group_indian_digits(value: u64) -> String {
    let digits = value.to_string();
    let prefix = digits.len().saturating_sub(3);
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index != 0 && index <= prefix && (prefix - index).is_multiple_of(2) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub(super) fn format_amount(amount: Money) -> String {
    let value = amount.minor_units();
    let magnitude = value.unsigned_abs();
    let sign = if value < 0 { "-" } else { "" };
    let whole = group_indian_digits(magnitude / 100);
    match magnitude % 100 {
        0 => format!("{sign}{whole}"),
        cents if cents.is_multiple_of(10) => format!("{sign}{whole}.{}", cents / 10),
        cents => format!("{sign}{whole}.{cents:02}"),
    }
}

pub(super) fn append_uri_component(url: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            url.push(char::from(byte));
        } else {
            url.push('%');
            url.push(char::from(HEX[usize::from(byte >> 4)]));
            url.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
}
