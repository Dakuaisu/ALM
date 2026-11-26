//! Platform handoff: generated files, desktop reveal, and WhatsApp links.
use crate::ipc::State;

use crate::{AppState, Trial, ipc::IpcDraft};

#[cfg(test)]
mod tests;

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum ShareOutput {
    Saved(String),
    File(ShareFile),
}

#[derive(Debug, serde::Serialize)]
pub struct ShareFile {
    pub name: String,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

pub(super) fn handoff(
    trial: &Trial,
    file: ShareFile,
    web_share: Option<bool>,
) -> Result<ShareOutput, String> {
    if web_share.unwrap_or(false) {
        return Ok(ShareOutput::File(file));
    }
    let path = trial.save_output(&file.name, &file.bytes)?;
    #[cfg(desktop)]
    reveal(&path);
    Ok(ShareOutput::Saved(path.display().to_string()))
}

#[cfg(desktop)]
pub(super) fn reveal(path: &std::path::Path) {
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

/// Generates the bill image for sharing, or saves and reveals it on desktop.
#[tauri::command(async)]
pub fn bill_image_share(
    state: State<'_, AppState>,
    id: IpcDraft,
    web_share: Option<bool>,
) -> Result<ShareOutput, String> {
    let trial = state.lock()?;
    let (name, bytes) = trial.bill_image_png(id.value())?;
    handoff(
        &trial,
        ShareFile {
            name,
            mime: "image/png",
            bytes,
        },
        web_share,
    )
}

/// Opens a WhatsApp link in the desktop browser; only wa.me links are accepted.
#[tauri::command(async)]
pub fn open_url(state: State<'_, AppState>, url: String) -> Result<(), String> {
    let _trial = state.lock()?;
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
