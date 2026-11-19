use super::{ConfirmationError, ConfirmationFixture, Role};
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

impl ConfirmationFixture {
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
        if let DraftCustomer::Selected { id, .. } = review.draft.customer()
            && let Some((saved_id, mobile)) = request.saved_customer_mobile
            && id.value() == saved_id
            && let Some(phone) = clean_phone(mobile)
        {
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

fn clean_phone(mobile: &str) -> Option<String> {
    let digits: String = mobile.chars().filter(char::is_ascii_digit).collect();
    (digits.len() >= 10).then(|| format!("91{}", &digits[digits.len() - 10..]))
}
