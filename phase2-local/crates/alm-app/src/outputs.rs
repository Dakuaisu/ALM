//! Generated outputs: the bill image (PNG) and opening a WhatsApp link. Files go to the app's
//! output folder; desktop reveals them, Android hands the path to the share bridge.
use alm_core::DraftId;
use alm_core::confirmation::bill_image::BillImageParty;
use alm_core::confirmation::{EstimateDisplay, EstimateLineDisplay, EstimateRequest};
use alm_core::confirmation::{PurchaseBusiness, PurchaseOutputRequest, PurchaseProductDisplay};
use alm_core::synthetic_read_model::{ArchivedPurchaseOrder, BusinessProfile, PurchaseCapture};
use tauri::State;

use crate::{AppState, Trial, debug, ipc::IpcId};

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
        let party = party(Some(self.shop.business_profile(role).map_err(debug)?));
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

    fn purchase_displays(&self) -> Vec<PurchaseProductDisplay<'_>> {
        self.directory
            .products(&self.authority)
            .map(|(id, item)| PurchaseProductDisplay {
                id,
                item,
                pack_size: "",
                uom: "",
                mrp: None,
            })
            .collect()
    }

    /// Every live purchase order as the archive stores it (`ArchivedPurchaseOrder::capture`).
    pub(crate) fn archived_orders(&self) -> Result<Vec<ArchivedPurchaseOrder>, String> {
        let role = self.require_owner()?;
        let displays = self.purchase_displays();
        let orders = self.authority.purchase_orders(role).map_err(debug)?;
        let mut archived = Vec::new();
        for row in self
            .authority
            .purchase_order_list(role)
            .map_err(debug)?
            .rows
        {
            let order = orders
                .iter()
                .find(|order| order.id == row.id)
                .ok_or("Purchase order not found")?;
            let view = self
                .authority
                .view_purchase_order(row.selection, &displays)
                .map_err(debug)?;
            archived.push(
                ArchivedPurchaseOrder::capture(
                    role,
                    PurchaseCapture {
                        order,
                        number: row.selection.number,
                        view: &view,
                        business: business(),
                        products: &displays,
                    },
                )
                .map_err(debug)?,
            );
        }
        archived.sort_by_key(|order| order.number);
        Ok(archived)
    }

    /// The live supplier print of PO number `number`.
    pub(crate) fn po_print_html(&self, number: u64) -> Result<String, String> {
        let role = self.require_owner()?;
        let displays = self.purchase_displays();
        let row = self
            .authority
            .purchase_order_list(role)
            .map_err(debug)?
            .rows
            .into_iter()
            .find(|row| row.selection.number == number)
            .ok_or("Purchase order not found")?;
        let mut html = None;
        self.authority
            .print_purchase_order(
                PurchaseOutputRequest {
                    selection: row.selection,
                    business: business(),
                    products: &displays,
                },
                &mut |doc| html = Some(doc.html),
            )
            .map_err(|error| error.to_string())?;
        html.ok_or_else(|| "Purchase order unavailable.".into())
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

/// The structured exports (`exportSalesCSV`, `exportExcel`) over the live archive records.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Export {
    SalesCsv,
    Workbook,
}

impl Trial {
    pub(crate) fn export(&self, kind: Export) -> Result<(String, String), String> {
        let role = self.require_owner()?;
        let records = self
            .authority
            .archive_records(role, &self.directory)
            .map_err(debug)?;
        let model = alm_core::synthetic_read_model::ReadModel::create(role, records, |_| Ok(()))
            .map_err(debug)?;
        let today = self.today()?;
        Ok(match kind {
            Export::SalesCsv => (
                format!("ALM-sales-{today}.csv"),
                model.sales_csv(role).map_err(debug)?,
            ),
            Export::Workbook => (
                format!("ALM-{today}.xls"),
                model.workbook(role).map_err(debug)?,
            ),
        })
    }
}

/// Opens the estimate (`tab` "estimate", bill draft ID) or PO print (`tab` "poprint", PO number).
#[tauri::command(async)]
pub fn print_open(state: State<'_, AppState>, id: IpcId, q: String) -> Result<String, String> {
    if q != "estimate" && q != "poprint" {
        return Err("Unknown print view.".into());
    }
    crate::with(&state, &q, |trial| {
        trial.require_owner()?;
        trial.print_view = Some(id.value());
        Ok(())
    })
}

/// Writes an export to the output folder and returns its path (revealed on desktop).
#[tauri::command(async)]
pub fn export_file(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let kind = match q.as_str() {
        "csv" => Export::SalesCsv,
        "xls" => Export::Workbook,
        _ => return Err("Unknown export.".into()),
    };
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    let (name, text) = trial.export(kind)?;
    let path = trial.save_output(&name, text.as_bytes())?;
    #[cfg(desktop)]
    reveal(&path);
    Ok(path.display().to_string())
}

/// The shop header the bill image prints (trial seller; the profile adds FSSAI/GSTIN lines).
pub(crate) fn party(profile: Option<&BusinessProfile>) -> BillImageParty<'_> {
    BillImageParty {
        business: crate::SELLER.name,
        tagline: "",
        address: crate::SELLER.address,
        mobile: "",
        profile,
        customer_mobile: "",
        customer_address: "",
    }
}

pub(crate) const fn business() -> PurchaseBusiness<'static> {
    PurchaseBusiness {
        name: crate::SELLER.name,
        address: crate::SELLER.address,
        mobile: "",
    }
}

/// Regenerates every output from a reopened archive alone (no live fixture, no seed).
#[cfg(test)]
pub(crate) struct Regenerated {
    pub bill_png: Vec<u8>,
    pub estimate: String,
    pub gst: String,
    pub po: String,
    pub rate_list: String,
}

#[cfg(test)]
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

#[cfg(desktop)]
fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = path;
}

/// Writes the bill image and returns its path; desktop reveals it, Android shares it.
#[tauri::command(async)]
pub fn bill_image_share(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    let (name, png) = trial.bill_image_png(DraftId::new(id.value()))?;
    let path = trial.save_output(&name, &png)?;
    #[cfg(desktop)]
    reveal(&path);
    Ok(path.display().to_string())
}

/// Opens a WhatsApp link in the desktop browser; only wa.me links are accepted.
#[tauri::command(async)]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://wa.me/") {
        return Err("Only WhatsApp links can be opened.".into());
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
