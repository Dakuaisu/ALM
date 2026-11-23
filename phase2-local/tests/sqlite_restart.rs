//! Synthetic local acceptance coverage for docs/PRD.md R-5 and R-6.

use std::ffi::OsStr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs, io};

use alm_phase2_local::SqliteSales;
use alm_phase2_local::{
    AccessDenied, DraftId, DraftLine, DraftStorageError, LocalSales, Money, ProductId,
    ProvisionalDraft, Role,
};

#[path = "sqlite_restart/customer.rs"]
mod customer;
#[path = "sqlite_restart/reconnect.rs"]
mod reconnect;
#[path = "common/sqlite_list.rs"]
mod sqlite_list;

const WORKER_MODE: &str = "ALM_SQLITE_RESTART_MODE";
const WORKER_DB: &str = "ALM_SQLITE_RESTART_DB";
const WORKER_DONE: &str = "ALM_SQLITE_RESTART_WORKER_DONE";
const SCRATCH_PREFIX: &str = "sqlite-restart-";
const DB_NAME: &str = "synthetic-drafts.sqlite3";
const PROFIT: Money = Money::from_minor_units(-54_321);
const EDITED_ID: DraftId = DraftId::new(17);
const EMPTY_ID: DraftId = DraftId::new(28);
const MISSING_ID: DraftId = DraftId::new(39);
const ORIGINAL_LINES: &[(u64, u32)] = &[(4, 1), (7, 6), (2, 4), (12, 3)];
const EDITED_LINES: &[(u64, u32)] = &[(12, 8), (4, 2), (12, 5)];

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = target_dir();
        fs::create_dir_all(&target).expect("create manifest target directory");
        assert_eq!(
            target.canonicalize().expect("resolve target directory"),
            target,
            "scratch must stay inside the manifest directory",
        );

        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = target.join(format!("{SCRATCH_PREFIX}{}-{seq}", process::id()));
            match fs::create_dir(&dir) {
                Ok(()) => return Self(dir),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create scratch directory {}: {err}", dir.display()),
            }
        }
    }

    fn db_path(&self) -> PathBuf {
        self.0.join(DB_NAME)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "remove SQLite scratch directory {}: {err}",
                self.0.display()
            );
        }
    }
}

fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("resolve Cargo manifest directory")
        .join("target")
}

fn draft(id: DraftId, lines: &[(u64, u32)]) -> ProvisionalDraft {
    let lines = lines
        .iter()
        .map(|&(product, quantity)| {
            DraftLine::new(
                ProductId::new(product),
                NonZeroU32::new(quantity).expect("synthetic quantity is positive"),
            )
        })
        .collect();
    ProvisionalDraft::new(id, lines)
}

fn open_sales(path: &Path) -> LocalSales {
    let opened: Result<LocalSales, DraftStorageError> = LocalSales::open_sqlite(path, PROFIT);
    opened.expect("open synthetic SQLite draft store")
}

fn run_worker(mode: &str, path: &Path) {
    let output = Command::new(env::current_exe().expect("locate integration test binary"))
        .args([
            "--ignored",
            "--exact",
            "sqlite_restart_worker",
            "--nocapture",
        ])
        .env_clear()
        .env("PATH", env::var_os("PATH").unwrap_or_default())
        .env(WORKER_MODE, mode)
        .env(WORKER_DB, path)
        .output()
        .expect("run SQLite restart worker");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Libtest also exits successfully when an exact filter matches no tests.
    assert!(
        output.status.success() && stdout.contains(WORKER_DONE),
        "{mode} worker did not finish ({})\nstdout:\n{stdout}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
}

fn run_sql(path: &Path, read_only: bool, sql: &str) -> String {
    let conn = if read_only {
        sqlite_list::open_read_only(path)
    } else {
        rusqlite::Connection::open(path)
    }
    .expect("open synthetic SQLite file");
    sqlite_list::sqlite_list(&conn, sql).expect("run synthetic SQL")
}

#[test]
fn edited_and_empty_drafts_survive_a_fresh_process_without_posting_tables() {
    // Given: a fresh file path and a writer process that exits cleanly.
    let scratch = ScratchDir::new();
    let path = scratch.db_path();
    run_worker("write", &path);

    // When: a different process reopens the file and checks the exact saved drafts.
    run_worker("read", &path);

    // Then: both drafts survived, and SQLite has no posting tables.
    let tables = run_sql(
        &path,
        true,
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name;",
    );
    assert_eq!(
        tables.lines().collect::<Vec<_>>(),
        [
            "pending_owner_confirmations",
            "provisional_draft_crates",
            "provisional_draft_customers",
            "provisional_draft_dates",
            "provisional_draft_lines",
            "provisional_drafts"
        ],
    );
}

#[test]
#[ignore = "subprocess worker dispatched by the SQLite restart test"]
fn sqlite_restart_worker() {
    let mode = match env::var(WORKER_MODE) {
        Ok(mode) => mode,
        Err(env::VarError::NotPresent) => return,
        Err(err) => panic!("invalid SQLite worker mode: {err}"),
    };
    let path = PathBuf::from(env::var_os(WORKER_DB).expect("worker database path"));
    let dir = path.parent().expect("worker scratch directory");
    let target = target_dir();
    assert!(path.is_absolute());
    assert_eq!(path.file_name(), Some(OsStr::new(DB_NAME)));
    assert_eq!(dir.parent(), Some(target.as_path()));
    assert!(
        dir.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(SCRATCH_PREFIX))
    );
    assert_eq!(
        dir.canonicalize()
            .expect("resolve scratch directory")
            .as_path(),
        dir,
    );

    match mode.as_str() {
        "write" => {
            assert!(matches!(
                fs::symlink_metadata(&path),
                Err(err) if err.kind() == io::ErrorKind::NotFound
            ));
            let mut sales = open_sales(&path);
            let mut manager = sales.session(Role::Manager);
            for to_save in [
                draft(EDITED_ID, ORIGINAL_LINES),
                draft(EMPTY_ID, &[(9, 2)]),
                draft(EDITED_ID, EDITED_LINES),
                draft(EMPTY_ID, &[]),
            ] {
                manager.save_draft(to_save).expect("save synthetic draft");
            }
        }
        "read" => {
            assert!(
                fs::symlink_metadata(&path)
                    .expect("writer created database")
                    .is_file()
            );
            let mut sales = open_sales(&path);
            let manager = sales.session(Role::Manager);
            for (id, expected) in [
                (EDITED_ID, Some(draft(EDITED_ID, EDITED_LINES))),
                (EMPTY_ID, Some(draft(EMPTY_ID, &[]))),
                (MISSING_ID, None),
            ] {
                let reopened: Result<Option<ProvisionalDraft>, DraftStorageError> =
                    manager.reopen_draft(id);
                assert_eq!(reopened.expect("reopen draft after restart"), expected);
            }
        }
        "write_reconnect" => reconnect::write_saved_draft(&path),
        "read_reconnect" => reconnect::reopen_and_confirm(&path),
        "customer_write" | "customer_edit" | "customer_read" => {
            customer::worker(mode.as_str(), &path)
        }
        _ => panic!("unknown SQLite worker mode: {mode}"),
    }
    println!("{WORKER_DONE}");
}

#[test]
fn sqlite_profit_is_visible_to_owner_but_denied_to_manager() {
    // Given: a SQLite-backed service with a synthetic profit total.
    let scratch = ScratchDir::new();
    let mut sales = open_sales(&scratch.db_path());

    // When: Owner reads the total, followed by Manager on the same service.
    let owner_profit = sales.session(Role::Owner).total_profit();
    let manager_profit = sales.session(Role::Manager).total_profit();

    // Then: only Owner receives the exact total.
    assert_eq!(owner_profit, Ok(PROFIT));
    assert_eq!(manager_profit, Err(AccessDenied));
}

#[test]
fn failed_line_insert_rolls_back_draft_replacement() {
    // Given: a committed draft and a trigger that rejects replacement lines.
    let scratch = ScratchDir::new();
    let path = scratch.db_path();
    let original =
        draft(EDITED_ID, ORIGINAL_LINES).with_bill_date(Some("2024-02-29".parse().unwrap()));
    open_sales(&path)
        .session(Role::Manager)
        .save_draft(original.clone())
        .expect("commit original draft");
    run_sql(
        &path,
        false,
        "CREATE TRIGGER reject_synthetic_line BEFORE INSERT ON provisional_draft_lines \
         BEGIN SELECT RAISE(ABORT, 'synthetic line insert failure'); END;",
    );

    // When: a line insert aborts while saving a replacement for the same draft ID.
    let rejected: Result<(), DraftStorageError> =
        open_sales(&path).session(Role::Manager).save_draft(
            draft(EDITED_ID, EDITED_LINES).with_bill_date(Some("2024-03-01".parse().unwrap())),
        );

    // Then: the save reports failure and a new connection sees the whole old draft.
    assert!(rejected.is_err(), "trigger must reject the replacement");
    let reopened = open_sales(&path)
        .session(Role::Manager)
        .reopen_draft(EDITED_ID)
        .expect("reopen draft after failed save");
    assert_eq!(reopened, Some(original));
}
