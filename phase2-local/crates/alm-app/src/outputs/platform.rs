//! Platform handoff: reveal on desktop, share path on Android, WhatsApp links.
use tauri::State;

use crate::{AppState, ipc::IpcDraft};

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

/// Writes the bill image and returns its path; desktop reveals it, Android shares it.
#[tauri::command(async)]
pub fn bill_image_share(state: State<'_, AppState>, id: IpcDraft) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    let (name, png) = trial.bill_image_png(id.value())?;
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
