use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use alm_store::confirmation::ConfirmationError;
use alm_store::{LocalSales, Role};

#[path = "account_switch_restart/authority.rs"]
mod authority;
#[path = "account_switch_restart/crate_review.rs"]
mod crate_review;
#[path = "account_switch_restart/fixtures.rs"]
mod fixtures;
#[path = "../crates/alm-store/tests/common/manager_sqlite.rs"]
mod manager_sqlite;
#[path = "account_switch_restart/sqlite_evidence.rs"]
mod sqlite_evidence;
#[path = "../crates/alm-store/tests/common/target_scratch.rs"]
mod target_scratch;

use alm_store::SqliteSales;
use fixtures::{A_EDITED, A_INITIAL, Account, B_SAVED, DRAFT, MISSING};
use manager_sqlite::{SqliteEvidence, databases};
use target_scratch::{Scratch, target_dir};

const ROOT_ENV: &str = "ALM_ACCOUNT_SWITCH_RESTART_ROOT";
const MODE_ENV: &str = "ALM_ACCOUNT_SWITCH_RESTART_MODE";
const WORKER: &str = "account_switch_restart_worker";
const DONE: &str = "ACCOUNT_SWITCH_RESTART_COMPLETED";
const PREFIX: &str = "account-switch-restart-";
const DEVICE: &str = "synthetic-device";

#[test]
fn manager_switches_accounts_and_reopens_both_drafts_after_process_restart() {
    let scratch = Scratch::new(PREFIX);
    let root = scratch.path().join(DEVICE);
    fs::create_dir(&root).expect("create synthetic device root");
    assert!(databases(&root).is_empty());

    run_worker(&root, "write-and-switch");
    let paths = databases(&root);
    let [first, second] = paths.as_slice() else {
        panic!("expected two account SQLite databases: {paths:?}");
    };
    let before = [first, second].map(|path| SqliteEvidence::read(path));
    run_worker(&root, "reopen-a");
    run_worker(&root, "reopen-b");
    assert_eq!(databases(&root), paths, "reopens must not add stores");
    assert_eq!(
        [first, second].map(|path| SqliteEvidence::read(path)),
        before,
        "fresh-process reads must not change either saved database"
    );
}

#[test]
#[ignore = "subprocess worker dispatched by the account-switch test"]
fn account_switch_restart_worker() {
    let Ok(mode) = env::var(MODE_ENV) else { return };
    let root = PathBuf::from(env::var_os(ROOT_ENV).expect("synthetic device root"));
    verify_root(&root);
    match mode.as_str() {
        "write-and-switch" => write_and_switch(&root),
        "reopen-a" => reopen_existing(&root, Account::A),
        "reopen-b" => reopen_existing(&root, Account::B),
        _ => panic!("unknown account-switch worker mode {mode}"),
    }
    println!("{DONE}:{mode}");
}

fn write_and_switch(root: &Path) {
    let mut authority = authority::seed();
    let before = authority::Effects::read(&authority);
    let open = |account: Account| {
        LocalSales::open_manager_account(root, account.key()).expect("select Manager account")
    };
    let mut a = open(Account::A);
    assert_eq!(a.reopen_draft(DRAFT).unwrap(), None);
    a.save_draft(A_INITIAL.draft()).unwrap();
    assert_eq!(a.reopen_draft(DRAFT).unwrap(), Some(A_INITIAL.draft()));
    let a_path = only_database(root);
    let a_initial = SqliteEvidence::read(&a_path);
    a_initial.assert_draft(&A_INITIAL);
    assert_eq!(authority::Effects::read(&authority), before);

    let mut b = open(Account::B);
    assert_eq!(
        b.reopen_draft(DRAFT).unwrap(),
        None,
        "B must not see A's saved ID"
    );
    let [b_path] = databases(root)
        .into_iter()
        .filter(|path| path != &a_path)
        .collect::<Vec<_>>()
        .try_into()
        .expect("B selection must create its own SQLite store");
    SqliteEvidence::read(&b_path).assert_empty(Account::B);
    b.save_draft(B_SAVED.draft()).unwrap();
    assert_eq!(b.reopen_draft(DRAFT).unwrap(), Some(B_SAVED.draft()));
    assert_eq!(a.reopen_draft(DRAFT).unwrap(), Some(A_INITIAL.draft()));
    assert_eq!(SqliteEvidence::read(&a_path), a_initial);
    let b_saved = SqliteEvidence::read(&b_path);
    b_saved.assert_draft(&B_SAVED);
    assert_eq!(authority::Effects::read(&authority), before);

    a = open(Account::A);
    assert_eq!(a.reopen_draft(DRAFT).unwrap(), Some(A_INITIAL.draft()));
    a.save_draft(A_EDITED.draft()).unwrap();
    let reopened_a = a.reopen_draft(DRAFT).unwrap().expect("A's edited draft");
    assert_eq!(reopened_a, A_EDITED.draft());
    assert_eq!(b.reopen_draft(DRAFT).unwrap(), Some(B_SAVED.draft()));
    b = open(Account::B);
    let reopened_b = b.reopen_draft(DRAFT).unwrap().expect("B's saved draft");
    assert_eq!(reopened_b, B_SAVED.draft());
    assert_eq!(SqliteEvidence::read(&b_path), b_saved);
    SqliteEvidence::read(&a_path).assert_draft(&A_EDITED);
    assert_eq!(authority::Effects::read(&authority), before);
    for draft in [reopened_a, reopened_b] {
        assert_eq!(
            authority.reconnect_review(Role::Manager, draft),
            Err(ConfirmationError::AccessDenied)
        );
        assert_eq!(authority::Effects::read(&authority), before);
    }
    println!("Owner control unchanged after Manager saves and switches: {before:?}");
}

fn reopen_existing(root: &Path, account: Account) {
    let expected = account.saved();
    assert_eq!(
        databases(root).len(),
        2,
        "writer databases must exist before restart"
    );
    let manager =
        LocalSales::open_manager_account(root, account.key()).expect("select saved account");
    let reopened = manager
        .reopen_draft(DRAFT)
        .unwrap()
        .expect("saved draft, not an empty store");
    assert_eq!(manager.reopen_draft(MISSING).unwrap(), None);
    assert_eq!(reopened, expected.draft());
    let projection = format!("{reopened:?}");
    assert!(projection.contains(account.marker()));
    assert!(!projection.contains(account.other().marker()));
    println!(
        "{} reopened Manager projection: {projection}",
        account.key()
    );

    let snapshots = databases(root)
        .iter()
        .map(|path| SqliteEvidence::read(path))
        .collect::<Vec<_>>();
    let [matched] = snapshots
        .iter()
        .filter(|row| row.dump().contains(account.marker()))
        .collect::<Vec<_>>()
        .try_into()
        .expect("exactly one on-disk database for the reopened payload");
    matched.assert_draft(expected);
    println!("{} reopened SQLite rows: {}", account.key(), matched.rows());
}

fn only_database(root: &Path) -> PathBuf {
    let [path] = databases(root)
        .try_into()
        .expect("only the first account has been opened");
    path
}

fn run_worker(root: &Path, mode: &str) {
    let scratch = root.parent().expect("device is inside scratch");
    let output = Command::new(env::current_exe().expect("locate integration test binary"))
        .args(["--ignored", "--exact", WORKER, "--nocapture"])
        .env_clear()
        .env("PATH", env::var_os("PATH").expect("PATH for sqlite3"))
        .env("HOME", scratch)
        .env("USERPROFILE", scratch)
        .env("TMPDIR", scratch)
        .env(ROOT_ENV, root)
        .env(MODE_ENV, mode)
        .current_dir(scratch)
        .output()
        .expect("run account-switch subprocess");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.lines().any(|line| line == format!("{DONE}:{mode}")),
        "{mode} worker did not complete ({})\nstdout:\n{stdout}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{stdout}");
}

fn verify_root(root: &Path) {
    let parent = root.parent().expect("scratch parent");
    assert_eq!(root, root.canonicalize().unwrap());
    assert_eq!(
        root.file_name().and_then(|name| name.to_str()),
        Some(DEVICE)
    );
    assert_eq!(parent.parent(), Some(target_dir().as_path()));
    assert!(
        parent
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(PREFIX))
    );
}
