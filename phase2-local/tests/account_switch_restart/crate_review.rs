use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use super::fixtures::B_LINE_EDITED;
use super::{Account, B_SAVED, DRAFT, LocalSales, PREFIX, Scratch, SqliteEvidence, only_database};

fn run_manager_cli(root: &Path, script: &str) -> String {
    let account = Account::B.key();
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
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{output:?}"
    );

    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn fresh_manager_cli_reads_saved_crate_without_prices_or_database_changes() {
    let scratch = Scratch::new(PREFIX);
    let root = scratch.path();
    let account = Account::B.key();
    let expected = B_SAVED.draft();
    LocalSales::open_manager_account(root, account)
        .unwrap()
        .save_draft(expected.clone())
        .unwrap();
    let db = only_database(root);
    let before = SqliteEvidence::read(&db);
    before.assert_draft(&B_SAVED);

    let stdout = run_manager_cli(root, "open\n21\nreview\nquit\n");
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
            "Line 1: Product ID 7 | Quantity: 5",
            "Line 2: Product ID 9 | Quantity: 2",
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

#[test]
fn displayed_line_number_selects_the_saved_product_for_quantity_edit() {
    let scratch = Scratch::new(PREFIX);
    let root = scratch.path();
    let account = Account::B.key();
    LocalSales::open_manager_account(root, account)
        .unwrap()
        .save_draft(B_SAVED.draft())
        .unwrap();

    let review = run_manager_cli(root, "open\n21\nreview\nquit\n");
    let line_number = review
        .lines()
        .filter_map(|line| line.strip_prefix("Line "))
        .find_map(|line| line.strip_suffix(": Product ID 9 | Quantity: 2"))
        .expect("review must identify the edit number for saved product 9");
    let edited = run_manager_cli(
        root,
        &format!("open\n21\nedit\n{line_number}\n8\nreview\nsave\nquit\n"),
    );

    assert_eq!(
        LocalSales::open_manager_account(root, account)
            .unwrap()
            .reopen_draft(DRAFT)
            .unwrap(),
        Some(B_LINE_EDITED.draft()),
    );
    assert!(
        edited
            .lines()
            .any(|line| line == "Line 2: Product ID 9 | Quantity: 8")
    );
    assert!(!edited.contains('₹'));
    println!(
        "Selected displayed line {line_number}; saved quantities: product 7 = 5, product 9 = 8"
    );
}
