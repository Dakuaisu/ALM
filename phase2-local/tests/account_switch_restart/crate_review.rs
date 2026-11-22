use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

use super::{Account, B_SAVED, DRAFT, LocalSales, Scratch, SqliteEvidence, only_database};

#[test]
fn fresh_manager_cli_reads_saved_crate_without_prices_or_database_changes() {
    let scratch = Scratch::new();
    let root = &scratch.0;
    let account = Account::B.key();
    let expected = B_SAVED.draft();
    LocalSales::open_manager_account(root, account)
        .unwrap()
        .save_draft(expected.clone())
        .unwrap();
    let db = only_database(root);
    let before = SqliteEvidence::read(&db);
    before.assert_draft(&B_SAVED);

    let mut child = Command::new(env!("CARGO_BIN_EXE_alm-draft-demo"))
        .arg("--device-root")
        .arg(root)
        .args(["--account", account])
        .env_clear()
        .env("PATH", env::var_os("PATH").expect("PATH for sqlite3"))
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("TMPDIR", root)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch fresh Manager CLI");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"open\n21\nreview\nquit\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{output:?}"
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.lines().any(|line| line == "Opened draft 21: SAVED."));
    let review = stdout
        .lines()
        .skip_while(|line| *line != "PROVISIONAL — NOT POSTED")
        .take_while(|line| !line.starts_with("Command ("))
        .collect::<Vec<_>>();
    assert_eq!(
        review,
        [
            "PROVISIONAL — NOT POSTED",
            "Draft ID: 21",
            "State: SAVED",
            "Customer: SYNTHETIC-B Second's Shop",
            "Item: Unrecognized item | Quantity: 5",
            "Item: Unrecognized item | Quantity: 2",
            "Crate quantity: 3",
            "Crate component: Product ID 9 | Quantity per crate: 6 | Unit: Piece",
            "Crate component: Product ID 7 | Quantity per crate: 2 | Unit: Case",
            "Bill preview unavailable: UnsupportedCrateRow",
        ]
    );
    assert!(!stdout.contains('₹'));
    assert_eq!(SqliteEvidence::read(&db), before);
    assert_eq!(
        LocalSales::open_manager_account(root, account)
            .unwrap()
            .reopen_draft(DRAFT)
            .unwrap(),
        Some(expected),
    );
    println!("{}", review.join("\n"));
}
