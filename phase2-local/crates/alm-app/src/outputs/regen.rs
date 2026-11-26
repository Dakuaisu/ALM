//! Regenerates every output from a reopened archive alone (test support).
use super::party;
use crate::refusal;

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
    mode: alm_core::confirmation::BalanceMode,
) -> Result<Regenerated, String> {
    use alm_core::confirmation::rate_list::rate_list_html_from_records;
    use alm_core::synthetic_read_model::SaleId;
    let id = SaleId::new(sale.to_owned());
    let bill_png = bill_png(store, role, sale, today, mode)?;
    let estimate = store
        .archived_estimate_html(role, &id, today, mode)
        .map_err(refusal)?;
    let mut gst_html = Err("GST bill missing".to_owned());
    store
        .emit_standalone_gst(role, gst, &mut |invoice| {
            gst_html = crate::gst_print::invoice_html(invoice, &crate::SELLER);
        })
        .map_err(refusal)?;
    let po = store
        .print_purchase_order(role, po)
        .map_err(refusal)?
        .ok_or("Purchase order not archived")?
        .html;
    let rate_list = rate_list_html_from_records(
        store.records(role).map_err(refusal)?,
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

/// The bill image of archived sale `sale` in balance `mode`, from the archive alone.
pub(crate) fn bill_png(
    store: &alm_store::synthetic_read_model::Store,
    role: alm_core::Role,
    sale: &str,
    today: alm_core::BillDate,
    mode: alm_core::confirmation::BalanceMode,
) -> Result<Vec<u8>, String> {
    use alm_core::confirmation::invoice_image::invoice_image_layout;
    let id = alm_core::synthetic_read_model::SaleId::new(sale.to_owned());
    let profile = store.business_profile(role).map_err(refusal)?;
    let document = store
        .archived_estimate(role, &id, today, mode)
        .map_err(refusal)?;
    let details = store
        .records(role)
        .map_err(refusal)?
        .bill_image_details(sale)
        .ok_or("Sale not archived")?;
    let image = invoice_image_layout(
        &document,
        &party(Some(profile), &details),
        details.loading_rate,
        true,
        &crate::image::measure,
    );
    crate::image::render_png(&image)
}
