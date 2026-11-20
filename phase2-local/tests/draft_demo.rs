use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs};

const DEMO: &str = env!("CARGO_BIN_EXE_alm-draft-demo");
const CREATE_DRAFT: &str = "new\nSynthetic Customer\n1\n3\nreview\nsave\nquit\n";
const REOPEN_DRAFT: &str = "open\n1\nreview\nquit\n";
const MANAGER: &str = "ALM synthetic Manager demo — local drafts only; not authenticated.";
const PROVISIONAL: &str = "PROVISIONAL — NOT POSTED";
const DRAFT_ID: &str = "Draft ID: 1";
const CUSTOMER: &str = "Customer: Synthetic Customer";
const ITEM: &str = "Item: Synthetic Lime | Quantity: 3";
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
    let mut child = scratch
        .command(DEMO)
        .arg("--db")
        .arg(scratch.database())
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
    assert!(output.status.success(), "{launch}: {}", output.status);
    script_written.expect("send synthetic draft script");
    assert!(output.stderr.is_empty(), "{launch}: unexpected stderr");
    (
        pid,
        String::from_utf8(output.stdout).expect("draft demo stdout is UTF-8"),
    )
}

fn assert_no_pending_confirmations(scratch: &ScratchDir) {
    let output = scratch
        .command("sqlite3")
        .args(["-batch", "-bail", "-list", "-noheader", "-readonly"])
        .arg(scratch.database())
        .arg("SELECT COUNT(*) FROM pending_owner_confirmations;")
        .output()
        .expect("run read-only SQLite pending-confirmation query");
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "read-only SQLite query failed: {}",
        output.status,
    );
    let stdout = String::from_utf8(output.stdout).expect("SQLite stdout is UTF-8");
    assert_eq!(stdout.trim().parse::<u64>().ok(), Some(0));
    println!("SQLite (read-only): 0 pending owner confirmations");
}

fn logical_snapshot(scratch: &ScratchDir) -> Vec<u8> {
    let output = scratch
        .command("sqlite3")
        .args(["-batch", "-bail", "-readonly"])
        .arg(scratch.database())
        .arg(".dump")
        .output()
        .expect("run read-only SQLite dump");
    assert!(output.status.success() && output.stderr.is_empty());
    output.stdout
}

fn data_version(input: &mut impl Write, output: &mut impl BufRead) -> u64 {
    writeln!(input, "PRAGMA data_version;").expect("request SQLite data version");
    input.flush().expect("flush SQLite data version query");
    let mut line = String::new();
    assert_ne!(
        output
            .read_line(&mut line)
            .expect("read SQLite data version"),
        0
    );
    line.trim()
        .parse()
        .expect("SQLite data version is an integer")
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
    let mut observer = scratch
        .command("sqlite3")
        .args(["-batch", "-bail", "-list", "-noheader", "-readonly"])
        .arg(scratch.database())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start read-only SQLite observer");
    let mut observer_input = observer.stdin.take().expect("SQLite observer stdin");
    let mut observer_output =
        BufReader::new(observer.stdout.take().expect("SQLite observer stdout"));
    let version_before = data_version(&mut observer_input, &mut observer_output);
    let snapshot_before = logical_snapshot(&scratch);
    let (after_pid, after) = run_demo(&scratch, REOPEN_DRAFT, "Launch 2: open and review");
    let version_after = data_version(&mut observer_input, &mut observer_output);
    let snapshot_after = logical_snapshot(&scratch);
    drop(observer_input);
    let observer_result = observer
        .wait_with_output()
        .expect("wait for SQLite observer");
    assert!(observer_result.status.success() && observer_result.stderr.is_empty());

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
            "Unknown command. Use new, save, open, review, or quit.",
            PROVISIONAL,
            "State: UNSAVED",
            "Saved draft 1: PROVISIONAL — NOT POSTED.",
        ],
    );
    assert_no_pending_confirmations(&scratch);
}
