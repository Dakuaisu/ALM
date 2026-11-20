use crate::confirmation::estimate::display_date;
use crate::confirmation::image_handoff::{append_uri_component, clean_phone, group_indian_digits};
use crate::confirmation::{ImageFile, ImageHandoff, ShareRejected, TextHandoffIntent};

use super::super::PurchaseSupplierId;
use super::{
    ConfirmationFixture, PurchaseError, PurchaseOrder, PurchaseOutputRequest, supplier_html,
};

/// Entry in the synthetic supplier directory used for recipient selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseSupplierContact<'a> {
    pub id: PurchaseSupplierId,
    pub name: &'a str,
    pub mobile: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseWhatsAppRequest<'a> {
    pub output: PurchaseOutputRequest<'a>,
    pub contacts: &'a [PurchaseSupplierContact<'a>],
}

/// Inert capture seam. The local fixture never renders, opens a URL, or writes a file.
pub trait PurchaseOrderHandoff: ImageHandoff {
    fn render_png(&mut self, html: &str, width: u32) -> Option<Vec<u8>>;
    fn text(&mut self, intent: TextHandoffIntent);
}

impl ConfirmationFixture {
    /// Captures the legacy WhatsApp branches against synthetic recipients and sinks.
    ///
    /// # Errors
    /// Denies Managers and rejects a missing order before invoking any sink.
    pub fn handoff_purchase_order(
        &self,
        request: PurchaseWhatsAppRequest<'_>,
        sinks: &mut impl PurchaseOrderHandoff,
    ) -> Result<(), PurchaseError> {
        let output = request.output;
        let order = self.selected_purchase_order(output.selection)?;
        let phone = supplier_phone(order, request.contacts);
        let html = supplier_html::order_html(order, output);
        let Some(bytes) = sinks.render_png(&html, 760) else {
            let mut text = format!(
                "*Order — {}* ({})\n\n",
                output.business.name,
                display_date(order.date)
            );
            for saved in &order.items {
                text.push_str(&format!(
                    "{} — {} cases\n",
                    saved.line.item,
                    group_indian_digits(u64::from(saved.line.ordered.get())),
                ));
            }
            let mut url = format!("https://wa.me/{}?text=", phone.as_deref().unwrap_or(""));
            append_uri_component(&mut url, &text);
            sinks.text(TextHandoffIntent {
                url,
                target: "_blank",
            });
            return Ok(());
        };

        let mut file = ImageFile {
            name: "o.png".into(),
            mime: "image/png",
            bytes,
        };
        let supported = sinks.can_share(&file);
        let number = output.selection.number();
        file.name = format!("Order-{number}.png");
        if supported {
            let title = format!("Order {number}");
            let text = format!("{} — order {number}", output.business.name);
            return match sinks.share(&file, &title, &text) {
                Ok(()) | Err(ShareRejected) => Ok(()),
            };
        }
        sinks.download(&file);
        if let Some(phone) = phone {
            sinks.chat(&phone);
        }
        Ok(())
    }
}

fn supplier_phone(
    order: &PurchaseOrder,
    contacts: &[PurchaseSupplierContact<'_>],
) -> Option<String> {
    let by_id = contacts
        .iter()
        .find(|contact| contact.id == order.supplier_id);
    let selected = by_id
        .filter(|contact| contact.name == order.supplier_name)
        .or_else(|| {
            contacts
                .iter()
                .find(|contact| contact.name == order.supplier_name)
        })
        .or(by_id)?;
    clean_phone(selected.mobile)
}
