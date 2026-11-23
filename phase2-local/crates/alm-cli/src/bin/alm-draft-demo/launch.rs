use std::ffi::OsString;
use std::io;
use std::path::Path;

use alm_core::LocalSales;
use alm_store::ManagerDraftStore;
use alm_store::SqliteSales;

const LEGACY: &str = "Storage: legacy single-account database (--db).";
const SCOPED: &str = "Storage: account-scoped local drafts.";

pub(super) fn open(args: &[OsString]) -> io::Result<(ManagerDraftStore, &'static str)> {
    match args {
        [flag, path] if flag == "--db" => {
            let store = LocalSales::open_legacy_manager_sqlite(Path::new(path));
            store
                .map(|drafts| (drafts, LEGACY))
                .map_err(io::Error::other)
        }
        [root_flag, root, account_flag, account]
            if root_flag == "--device-root" && account_flag == "--account" =>
        {
            let account = account.to_str().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Synthetic account ID must be ASCII",
                )
            })?;
            let store = LocalSales::open_manager_account(Path::new(root), account);
            store
                .map(|drafts| (drafts, SCOPED))
                .map_err(io::Error::other)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: alm-draft-demo --device-root /absolute/device/root --account SYNTHETIC_ID\n       alm-draft-demo --db /absolute/path/to/legacy.sqlite3 (legacy single-account demo)",
        )),
    }
}
