//! Regenerates every output from a reopened archive alone (test support).
use super::party;
use crate::debug;

/// Regenerates every output from a reopened archive alone (no live fixture, no seed).
pub(crate) struct Regenerated {
    pub bill_png: Vec<u8>,
    pub estimate: String,
    pub gst: String,
    pub po: String,
    pub rate_list: String,
}

pub(crate) fn regenerate(
    store: &alm_store::synthetic_read_model::Store,
    role: alm_core::Role,
    sale: &str,
    gst: alm_core::confirmation::GstBillId,
    po: u64,
    today: alm_core::BillDate,
) -> Result<Regenerated, String> {
    use alm_core::confirmation::bill_image::bill_image_layout;
    use alm_core::confirmation::rate_list::rate_list_html_from_records;
    use alm_core::synthetic_read_model::SaleId;
    let id = SaleId::new(sale.to_owned());
    let profile = store.business_profile(role).map_err(debug)?;
    let document = store.latest_estimate(role, &id, today).map_err(debug)?;
    let image = bill_image_layout(&document, &party(Some(profile)), &crate::image::measure);
    let bill_png = crate::image::render_png(&image)?;
    let estimate = store
        .latest_estimate_html(role, &id, today)
        .map_err(debug)?;
    let mut gst_html = Err("GST bill missing".to_owned());
    store
        .emit_standalone_gst(role, gst, &mut |invoice| {
            gst_html = crate::gst_print::invoice_html(invoice, &crate::SELLER);
        })
        .map_err(debug)?;
    let po = store
        .print_purchase_order(role, po)
        .map_err(debug)?
        .ok_or("Purchase order not archived")?
        .html;
    let rate_list = rate_list_html_from_records(
        store.records(role).map_err(debug)?,
        (crate::SELLER.name, crate::SELLER.address, ""),
        today,
    )
    .ok_or("No products to list")?;
    Ok(Regenerated {
        bill_png,
        estimate,
        gst: gst_html?,
        po,
        rate_list,
    })
}
