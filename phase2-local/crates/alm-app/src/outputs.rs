//! Generated outputs: the bill image (PNG) and opening a WhatsApp link. Files go to the app's
//! output folder; desktop reveals them, Android hands the path to the share bridge.
use alm_core::DraftId;
use alm_core::confirmation::bill_image::BillImageParty;
use alm_core::confirmation::{
    BillImageDetails, EstimateDisplay, EstimateLineDisplay, EstimateRequest,
};
use alm_core::synthetic_read_model::BusinessProfile;

use crate::{Trial, debug};

pub mod export;
pub mod platform;
mod purchase;
#[cfg(test)]
mod regen;

#[cfg(test)]
pub(crate) use export::Export;
#[cfg(test)]
pub(crate) use regen::{bill_png, regenerate};

impl Trial {
    /// The bill image PNG for an Owner-visible bill, with its file name.
    pub(crate) fn bill_image_png(&self, id: DraftId) -> Result<(String, Vec<u8>), String> {
        let role = self.require_owner()?;
        let mode = self.prefs.balance_mode();
        let bill = self.authority.bill_preview(role, id, mode).map_err(debug)?;
        let labels: Vec<String> = bill
            .lines
            .iter()
            .map(|line| {
                self.directory
                    .products(&self.authority)
                    .find(|(product, _)| *product == line.product)
                    .map_or_else(|| line.product.to_string(), |(_, name)| name.to_owned())
            })
            .collect();
        let lines: Vec<_> = bill
            .lines
            .iter()
            .zip(&labels)
            .map(|(line, label)| EstimateLineDisplay {
                product: line.product,
                label,
            })
            .collect();
        let request = EstimateRequest {
            role,
            draft_id: id,
            balance_mode: mode,
            display: EstimateDisplay {
                today: self.today()?,
                lines: &lines,
            },
        };
        let details = self
            .directory
            .bill_image_details(&bill.customer, bill.lines.iter().map(|line| line.product));
        let party = party(
            Some(self.shop.business_profile(role).map_err(debug)?),
            &details,
        );
        let image = self
            .authority
            .bill_image(request, &party, &crate::image::measure)
            .map_err(debug)?;
        let png = crate::image::render_png(&image)?;
        Ok((format!("Bill-{}.png", bill.invoice), png))
    }

    /// The live estimate (print document) at the device's balance preference.
    pub(crate) fn estimate_html(&self, id: DraftId) -> Result<String, String> {
        let role = self.require_owner()?;
        let labels = self.line_labels(id)?;
        let lines: Vec<_> = labels
            .iter()
            .map(|(product, label)| EstimateLineDisplay {
                product: *product,
                label,
            })
            .collect();
        let request = EstimateRequest {
            role,
            draft_id: id,
            balance_mode: self.prefs.balance_mode(),
            display: EstimateDisplay {
                today: self.today()?,
                lines: &lines,
            },
        };
        let profile = self.shop.business_profile(role).map_err(debug)?;
        let mut html = None;
        self.authority
            .print_estimate_with_business(request, profile, &mut |doc| html = Some(doc.html))
            .map_err(debug)?;
        html.ok_or_else(|| "Estimate unavailable.".into())
    }

    fn line_labels(&self, id: DraftId) -> Result<Vec<(alm_core::ProductId, String)>, String> {
        let role = self.require_owner()?;
        let bill = self
            .authority
            .bill_preview(role, id, alm_core::confirmation::BalanceMode::AsOnBillDate)
            .map_err(debug)?;
        Ok(bill
            .lines
            .iter()
            .map(|line| {
                let name = self
                    .directory
                    .products(&self.authority)
                    .find(|(product, _)| *product == line.product)
                    .map_or_else(|| line.product.to_string(), |(_, name)| name.to_owned());
                (line.product, name)
            })
            .collect())
    }

    /// The live standalone GST invoice HTML.
    #[cfg(test)]
    pub(crate) fn gst_invoice_html(
        &self,
        id: alm_core::confirmation::GstBillId,
    ) -> Result<String, String> {
        let role = self.require_owner()?;
        let mut page = Err("GST bill missing".to_owned());
        self.authority
            .emit_gst_bill(role, id, &mut |invoice| {
                page = crate::gst_print::invoice_html(invoice, &crate::SELLER);
            })
            .map_err(debug)?;
        page
    }

    pub(crate) fn save_output(
        &self,
        name: &str,
        bytes: &[u8],
    ) -> Result<std::path::PathBuf, String> {
        std::fs::create_dir_all(&self.outputs).map_err(|e| e.to_string())?;
        let path = self.outputs.join(name);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

/// The shop header the bill image prints (trial seller; the profile adds FSSAI/GSTIN lines).
pub(crate) fn party<'a>(
    profile: Option<&'a BusinessProfile>,
    details: &'a BillImageDetails,
) -> BillImageParty<'a> {
    let fallback = [crate::SELLER.name, "", crate::SELLER.address, ""];
    let [business, tagline, address, mobile] =
        profile.map_or(fallback, |profile| profile.header(fallback));
    BillImageParty {
        business,
        tagline,
        address,
        mobile,
        profile,
        customer_mobile: &details.customer_mobile,
        customer_address: &details.customer_address,
        lines: &details.lines,
    }
}
