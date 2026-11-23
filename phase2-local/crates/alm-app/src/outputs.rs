//! Generated outputs: the bill image (PNG) and opening a WhatsApp link. Files go to the app's
//! output folder; desktop reveals them, Android hands the path to the share bridge.
use alm_core::DraftId;
use alm_core::confirmation::bill_image::BillImageParty;
use alm_core::confirmation::{EstimateDisplay, EstimateLineDisplay, EstimateRequest};
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
        let party = BillImageParty {
            business: crate::SELLER.name,
            tagline: "",
            address: crate::SELLER.address,
            mobile: "",
            profile: None,
            customer_mobile: "",
            customer_address: "",
        };
        let image = self
            .authority
            .bill_image(request, &party, &crate::image::measure)
            .map_err(debug)?;
        let png = crate::image::render_png(&image)?;
        Ok((format!("Bill-{}.png", bill.invoice), png))
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
