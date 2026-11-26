//! This device's own `sync` entries of the HTML's activity log (`logAct("sync", …)`): events
//! the authority cannot record because they happen to this device's copy (a rebuild from the
//! authority, a revoked account). One line per entry, `<ms>\t<kind>\t<message>`, in the app
//! data folder; shown with the authority's log on Settings.
use std::io::Write as _;
use std::path::Path;

use alm_core::projection::book::ActivityRow;

const FILE: &str = "device-activity.log";

/// Appends a `kind` entry; a failed write loses the entry but never the action it records.
pub(crate) fn record(device_root: &Path, kind: &str, message: &str) {
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |e| u64::try_from(e.as_millis()).unwrap_or(u64::MAX));
    let line = format!(
        "{at}\t{kind}\t{}\n",
        message.replace(['\t', '\n', '\r'], " ")
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(device_root.join(FILE))
        .and_then(|mut file| file.write_all(line.as_bytes()));
}

/// The entries as activity rows from `device`, oldest first; unreadable lines are skipped.
pub(crate) fn rows(device_root: &Path, device: &str) -> Vec<ActivityRow> {
    let text = std::fs::read_to_string(device_root.join(FILE)).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let at_ms = parts.next()?.parse().ok()?;
            let kind = parts.next()?.to_owned();
            let subject = parts.next()?.to_owned();
            Some(ActivityRow {
                at_ms,
                device: device.to_owned(),
                account: "this device".to_owned(),
                kind,
                subject,
                amount: None,
                detail: String::new(),
            })
        })
        .collect()
}
