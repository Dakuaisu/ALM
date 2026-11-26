//! Choosing a file (Settings > Backup & Restore and the legacy import): the platform's file
//! dialog (tauri-plugin-dialog) picks it, the file is copied into the app's own folder first
//! (`backups` or `legacy-import`, under a fresh safe name), and only then the existing validated
//! paths read it: a backup is listed for Restore, a legacy file gets its dry run. On Android the
//! dialog returns a `content://` URI, which the app's platform bridge copies to the path Rust
//! names (inside the app's data folder) before `file_picked` continues.
use std::path::{Path, PathBuf};

use crate::ipc::State;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::{AppState, Trial, text, with};

#[derive(Clone, Copy)]
enum Kind {
    Backup,
    Legacy,
}

impl Kind {
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "backup" => Ok(Self::Backup),
            "legacy" => Ok(Self::Legacy),
            _ => Err("Unknown file kind.".into()),
        }
    }

    const fn stem(self) -> &'static str {
        match self {
            Self::Backup => "picked-backup",
            Self::Legacy => "picked-legacy",
        }
    }
}

/// A name the folder listings accept: the chosen file's own name when it is safe, else a fresh
/// `picked-…-<n>.json`; never one already there.
fn target(dir: &Path, chosen: Option<&str>, kind: Kind) -> PathBuf {
    let safe = chosen.filter(|name| {
        name.ends_with(".json")
            && !name.starts_with('.')
            && name.len() <= 80
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    });
    if let Some(name) = safe
        && !dir.join(name).exists()
    {
        return dir.join(name);
    }
    (1..)
        .map(|n| dir.join(format!("{}-{n}.json", kind.stem())))
        .find(|path| !path.exists())
        .unwrap_or_else(|| dir.join(format!("{}.json", kind.stem())))
}

impl Trial {
    fn pick_dir(&self, kind: Kind) -> PathBuf {
        match kind {
            Kind::Backup => self.backup_dir(),
            Kind::Legacy => self.legacy_dir(),
        }
    }

    /// Copies a file chosen on this device into the app's folder for `kind`.
    pub(crate) fn file_copy_in(&mut self, kind: &str, source: &Path) -> Result<String, String> {
        self.require_owner()?;
        let kind = Kind::parse(kind)?;
        let dir = self.pick_dir(kind);
        std::fs::create_dir_all(&dir).map_err(text)?;
        let chosen = source.file_name().and_then(|n| n.to_str());
        let path = target(&dir, chosen, kind);
        std::fs::copy(source, &path).map_err(text)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        self.file_picked(&format!("{}/{name}", text_of(kind)))?;
        Ok(name)
    }

    /// The path the platform bridge copies a picked `content://` file to.
    fn copy_target(&self, kind: &str) -> Result<PathBuf, String> {
        self.require_owner()?;
        let kind = Kind::parse(kind)?;
        let dir = self.pick_dir(kind);
        std::fs::create_dir_all(&dir).map_err(text)?;
        Ok(target(&dir, None, kind))
    }

    /// Continues after the copy with the existing validated path for the file's kind.
    pub(crate) fn file_picked(&mut self, q: &str) -> Result<(), String> {
        self.require_owner()?;
        let (kind, name) = q.split_once('/').ok_or("Unknown file.")?;
        let kind = Kind::parse(kind)?;
        if !self.pick_dir(kind).join(name).is_file() || name.contains(['/', '\\']) {
            return Err("The chosen file was not copied.".into());
        }
        match kind {
            Kind::Legacy => self.legacy_dry_run(name),
            Kind::Backup => {
                if !self.backups().iter().any(|(listed, _)| listed == name) {
                    return Err("Not a backup file name.".into());
                }
                self.message = format!(
                    "Copied {name} into this app's backups: press ⬆ Restore on it to check and restore it."
                );
                Ok(())
            }
        }
    }
}

const fn text_of(kind: Kind) -> &'static str {
    match kind {
        Kind::Backup => "backup",
        Kind::Legacy => "legacy",
    }
}

/// Opens the platform's file dialog. Desktop: the chosen file is copied in and the Settings
/// screen returned. Android: `COPY\t<content URI>\t<target path>\t<kind>/<name>` for the bridge;
/// an empty answer when the dialog was closed.
#[tauri::command(async)]
pub fn file_pick(app: AppHandle, state: State<'_, AppState>, q: String) -> Result<String, String> {
    Kind::parse(&q)?;
    state.lock()?.require_owner()?;
    let picked = app
        .dialog()
        .file()
        .add_filter("ALM backup (.json)", &["json"])
        .blocking_pick_file();
    match picked {
        None => Ok(String::new()),
        Some(FilePath::Path(path)) => with(&state, "settings", |trial| {
            trial.file_copy_in(&q, &path).map(|_| ())
        }),
        Some(FilePath::Url(url)) if url.scheme() == "file" => {
            let path = url
                .to_file_path()
                .map_err(|()| "Not a local file.".to_owned())?;
            with(&state, "settings", |trial| {
                trial.file_copy_in(&q, &path).map(|_| ())
            })
        }
        Some(FilePath::Url(url)) => {
            let trial = state.lock()?;
            let dest = trial.copy_target(&q)?;
            let name = dest
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Bad target.")?;
            Ok(format!(
                "COPY\t{url}\t{}\t{q}/{name}",
                dest.to_str().ok_or("Bad target.")?
            ))
        }
    }
}

#[tauri::command(async)]
pub fn file_picked(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "settings", |trial| trial.file_picked(&q))
}
