use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs};

use alm_core::{DraftCustomer, DraftId, LocalSales, ProductId};
use alm_store::SqliteSales;

#[path = "draft_demo/scoped_edit.rs"]
mod scoped_edit;
#[path = "../../alm-store/tests/common/sqlite_dump.rs"]
mod sqlite_dump;
#[path = "../../alm-store/tests/common/sqlite_list.rs"]
mod sqlite_list;

const DEMO: &str = env!("CARGO_BIN_EXE_alm-draft-demo");
const CREATE_DRAFT: &str = "new\nSynthetic Customer\n1\n3\nreview\nsave\nquit\n";
const REOPEN_DRAFT: &str = "open\n1\nreview\nquit\n";
const MANAGER: &str = "ALM synthetic Manager demo — local drafts only; not authenticated.";
const LEGACY: &str = "Storage: legacy single-account database (--db).";
const PROVISIONAL: &str = "PROVISIONAL — NOT POSTED";
const DRAFT_ID: &str = "Draft ID: 1";
const CUSTOMER: &str = "Customer: Synthetic Customer";
const ITEM: &str = "Line 1: Item: Synthetic Lime | Quantity: 3";
const LINE_AMOUNTS: &str = "Unit price (inclusive): ₹118.00 | Line total (inclusive): ₹354.00";
const TAXABLE: &str = "Taxable: ₹300.00";
const CGST: &str = "CGST: ₹27.00";
const SGST: &str = "SGST: ₹27.00";
const TOTAL: &str = "Inclusive total: ₹354.00";

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .expect("resolve manifest directory")
            .join("target");
        fs::create_dir_all(&target).expect("ensure crate target directory exists");
        assert!(
            target.canonicalize().expect("resolve target directory") == target,
            "scratch must stay inside the crate target directory",
        );

        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = target.join(format!("draft-demo-{}-{seq}", process::id()));
            match fs::create_dir(&dir) {
                Ok(()) => return Self(dir),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create draft demo scratch directory: {err}"),
            }
        }
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("PATH", env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.0)
            .env("USERPROFILE", &self.0)
            .current_dir(&self.0);
        command
    }

    fn database(&self) -> PathBuf {
        self.0.join("synthetic-drafts.sqlite3")
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.0) {
            eprintln!("remove draft demo scratch directory: {err}");
        }
    }
}

fn run_demo(scratch: &ScratchDir, script: &str, launch: &str) -> (u32, String) {
    let mut command = scratch.command(DEMO);
    command.arg("--db").arg(scratch.database());
    let (pid, output) = run_script(command, script, launch);
    assert!(output.status.success(), "{launch}: {output:?}");
    assert!(output.stderr.is_empty(), "{launch}: unexpected stderr");
    (
        pid,
        String::from_utf8(output.stdout).expect("draft demo stdout is UTF-8"),
    )
}

fn run_script(mut command: Command, script: &str, launch: &str) -> (u32, Output) {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{launch}: launch draft demo: {err}"));
    let pid = child.id();
    let script_written = child
        .stdin
        .take()
        .expect("piped draft demo stdin")
        .write_all(script.as_bytes());
    let output = child.wait_with_output().expect("wait for draft demo");
    script_written.expect("send synthetic draft script");
    (pid, output)
}

fn assert_no_pending_confirmations(scratch: &ScratchDir) {
    let conn = sqlite_list::open_read_only(&scratch.database())
        .expect("open read-only SQLite pending-confirmation query");
    let stdout =
        sqlite_list::sqlite_list(&conn, "SELECT COUNT(*) FROM pending_owner_confirmations;")
            .expect("run read-only SQLite pending-confirmation query");
    assert_eq!(stdout.trim().parse::<u64>().ok(), Some(0));
    println!("SQLite (read-only): 0 pending owner confirmations");
}

fn logical_snapshot(scratch: &ScratchDir) -> String {
    let conn =
        sqlite_list::open_read_only(&scratch.database()).expect("open read-only SQLite dump");
    sqlite_dump::sqlite_dump(&conn).expect("run read-only SQLite dump")
}

fn data_version(observer: &rusqlite::Connection) -> i64 {
    observer
        .pragma_query_value(None, "data_version", |row| row.get(0))
        .expect("read SQLite data version")
}

fn assert_transcript(launch: &str, stdout: &str, expected: &[&str]) {
    println!("{launch} (selected CLI output):");
    let mut lines = stdout.lines().map(str::trim);
    for &expected_line in expected {
        let line = lines
            .find(|line| *line == expected_line)
            .unwrap_or_else(|| panic!("{launch}: missing expected line: {expected_line}"));
        println!("  {line}");
    }
}

#[test]
fn saved_provisional_draft_reopens_in_a_fresh_cli_process() {
    // Given: a new database path that is shared only by these launches.
    let scratch = ScratchDir::new();

    // When: one process saves and exits, then a second process opens draft 1.
    let (before_pid, before) = run_demo(&scratch, CREATE_DRAFT, "Launch 1: create and save");
    let observer =
        sqlite_list::open_read_only(&scratch.database()).expect("start read-only SQLite observer");
    let version_before = data_version(&observer);
    let snapshot_before = logical_snapshot(&scratch);
    let (after_pid, after) = run_demo(&scratch, REOPEN_DRAFT, "Launch 2: open and review");
    let version_after = data_version(&observer);
    let snapshot_after = logical_snapshot(&scratch);
    drop(observer);

    // Then: review shows the same synthetic draft, still provisional, across distinct launches.
    assert_ne!(
        before_pid, after_pid,
        "the CLI must run in distinct processes"
    );
    assert_transcript(
        "Launch 1: create and save",
        &before,
        &[
            MANAGER,
            LEGACY,
            PROVISIONAL,
            DRAFT_ID,
            "State: UNSAVED",
            CUSTOMER,
            ITEM,
            LINE_AMOUNTS,
            TAXABLE,
            CGST,
            SGST,
            TOTAL,
            "Saved draft 1: PROVISIONAL — NOT POSTED.",
        ],
    );
    assert_transcript(
        "Launch 2: open and review",
        &after,
        &[
            MANAGER,
            LEGACY,
            "Opened draft 1: SAVED.",
            PROVISIONAL,
            DRAFT_ID,
            "State: SAVED",
            CUSTOMER,
            ITEM,
            LINE_AMOUNTS,
            TAXABLE,
            CGST,
            SGST,
            TOTAL,
        ],
    );
    assert_eq!(
        version_before, version_after,
        "review must not commit SQLite writes"
    );
    assert_eq!(
        snapshot_before, snapshot_after,
        "review must not change SQLite contents"
    );
    for transcript in [&before, &after] {
        let lower = transcript.to_ascii_lowercase();
        assert!(!lower.contains("profit") && !lower.contains("cost"));
    }
    println!("Process check: distinct CLI launches");
    println!("SQLite check: no committed writes or logical changes during reopen/review");
    assert_no_pending_confirmations(&scratch);
    let store = LocalSales::open_legacy_manager_sqlite(&scratch.database()).unwrap();
    let saved = store.reopen_draft(DraftId::new(1)).unwrap().unwrap();
    assert_eq!(
        saved.customer(),
        &DraftCustomer::Typed("Synthetic Customer".into())
    );
    assert_eq!(saved.lines()[0].product_id(), ProductId::new(1));
    assert_eq!(saved.lines()[0].quantity().get(), 3);
}

#[test]
fn post_is_rejected_when_a_synthetic_draft_is_active() {
    // Given: a fresh database and a script that enters a synthetic draft.
    let scratch = ScratchDir::new();
    let script = "new\nSynthetic Customer\n1\n3\npost\nreview\nsave\nquit\n";

    // When: the Manager tries the unsupported post command with the draft active.
    let (_, stdout) = run_demo(&scratch, script, "Posting probe");

    // Then: post is rejected, and ordinary draft review and save remain available.
    assert_transcript(
        "Posting probe",
        &stdout,
        &[
            "Unknown command. Use new, save, open, edit, review, or quit.",
            PROVISIONAL,
            "State: UNSAVED",
            "Saved draft 1: PROVISIONAL — NOT POSTED.",
        ],
    );
    assert_no_pending_confirmations(&scratch);
}
