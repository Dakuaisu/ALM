use std::num::NonZeroU32;
use std::path::Path;

use alm_core::{
    DraftCustomer, DraftCustomerId, DraftId, DraftLine, LocalSales, ProductId, ProvisionalDraft,
};

use super::{DEMO, DRAFT_ID, PROVISIONAL, REOPEN_DRAFT, ScratchDir, assert_transcript, run_script};
use alm_store::SqliteSales;

const A: &str = "synthetic-edit-a";
const B: &str = "synthetic-edit-b";
const ID: DraftId = DraftId::new(1);

fn draft(account: &str, quantities: [u32; 2]) -> ProvisionalDraft {
    let lines = quantities
        .map(|quantity| DraftLine::new(ProductId::new(1), NonZeroU32::new(quantity).unwrap()));
    ProvisionalDraft::new(ID, lines.into())
        .with_bill_date(Some("2025-02-12".parse().unwrap()))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new(account.into()),
            name: format!("Synthetic buyer {account}"),
        })
}

#[test]
fn scoped_quantity_edit_survives_only_explicit_save_across_processes() {
    let scratch = ScratchDir::new();
    let initial_a = draft(A, [2, 3]);
    let initial_b = draft(B, [4, 5]);
    for (account, initial) in [(A, &initial_a), (B, &initial_b)] {
        LocalSales::open_manager_account(&scratch.0, account)
            .unwrap()
            .save_draft(initial.clone())
            .unwrap();
    }
    let launch = |account: &str, script: &str, label: &str| {
        let mut command = scratch.command(DEMO);
        command
            .arg("--device-root")
            .arg(&scratch.0)
            .arg("--account")
            .arg(account);
        let (_, output) = run_script(command, script, label);
        (
            output.status,
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };

    let (status, stdout, stderr) = launch(A, REOPEN_DRAFT, "A review before edit");
    assert!(status.success() && stderr.is_empty(), "{status}: {stderr}");
    let initial_review = [
        "Opened draft 1: SAVED.",
        PROVISIONAL,
        DRAFT_ID,
        "State: SAVED",
        "Customer: Synthetic buyer synthetic-edit-a",
        "Line 1: Item: Synthetic Lime | Quantity: 2",
        "Unit price (inclusive): ₹118.00 | Line total (inclusive): ₹236.00",
        "Line 2: Item: Synthetic Lime | Quantity: 3",
        "Unit price (inclusive): ₹118.00 | Line total (inclusive): ₹354.00",
        "Taxable: ₹500.00",
        "CGST: ₹45.00",
        "SGST: ₹45.00",
        "Inclusive total: ₹590.00",
    ];
    assert_transcript("A review before edit", &stdout, &initial_review);
    let line_number = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("Line "))
        .find_map(|line| line.strip_suffix(": Item: Synthetic Lime | Quantity: 3"))
        .expect("review identifies the second line's edit number");
    let edit_script = format!("open\n1\nedit\n{line_number}\n7\nreview\n");
    let (status, stdout, stderr) = launch(A, &edit_script, "A edit then EOF");
    assert!(!status.success(), "unsaved EOF must fail");
    assert!(
        stderr.contains("Input ended; draft 1 was NOT SAVED."),
        "{stderr}"
    );
    assert!(!stdout.contains("Saved draft 1:"));
    let edited_review = [
        "Opened draft 1: SAVED.",
        "Line number (one-based):",
        "Quantity (positive whole number):",
        "Edited draft 1: UNSAVED.",
        PROVISIONAL,
        DRAFT_ID,
        "State: UNSAVED",
        "Customer: Synthetic buyer synthetic-edit-a",
        "Line 1: Item: Synthetic Lime | Quantity: 2",
        "Unit price (inclusive): ₹118.00 | Line total (inclusive): ₹236.00",
        "Line 2: Item: Synthetic Lime | Quantity: 7",
        "Unit price (inclusive): ₹118.00 | Line total (inclusive): ₹826.00",
        "Taxable: ₹900.00",
        "CGST: ₹81.00",
        "SGST: ₹81.00",
        "Inclusive total: ₹1062.00",
    ];
    assert_transcript("A edit then EOF", &stdout, &edited_review);

    let (status, stdout, stderr) = launch(A, REOPEN_DRAFT, "A fresh reopen after EOF");
    assert!(status.success() && stderr.is_empty(), "{status}: {stderr}");
    assert_transcript("A fresh reopen after EOF", &stdout, &initial_review);
    assert_eq!(read_draft(&scratch.0, A), initial_a);
    assert_eq!(read_draft(&scratch.0, B), initial_b);

    let save_script = format!("{edit_script}save\nquit\n");
    let (status, stdout, stderr) = launch(A, &save_script, "A edit and Save");
    assert!(status.success() && stderr.is_empty(), "{status}: {stderr}");
    assert_transcript(
        "A edit and Save",
        &stdout,
        &[
            edited_review.as_slice(),
            &["Saved draft 1: PROVISIONAL — NOT POSTED."],
        ]
        .concat(),
    );

    for (account, expected, customer, items) in [
        (
            A,
            draft(A, [2, 7]),
            "Customer: Synthetic buyer synthetic-edit-a",
            [
                "Line 1: Item: Synthetic Lime | Quantity: 2",
                "Line 2: Item: Synthetic Lime | Quantity: 7",
            ],
        ),
        (
            B,
            initial_b,
            "Customer: Synthetic buyer synthetic-edit-b",
            [
                "Line 1: Item: Synthetic Lime | Quantity: 4",
                "Line 2: Item: Synthetic Lime | Quantity: 5",
            ],
        ),
    ] {
        let label = format!("{account} fresh reopen after Save");
        let (status, stdout, stderr) = launch(account, REOPEN_DRAFT, &label);
        assert!(status.success() && stderr.is_empty(), "{status}: {stderr}");
        assert_transcript(
            &label,
            &stdout,
            &[
                "Opened draft 1: SAVED.",
                PROVISIONAL,
                DRAFT_ID,
                "State: SAVED",
                customer,
                items[0],
                items[1],
            ],
        );
        assert_eq!(read_draft(&scratch.0, account), expected);
    }
}

fn read_draft(root: &Path, account: &str) -> ProvisionalDraft {
    LocalSales::open_manager_account(root, account)
        .expect("select Manager account for readback")
        .reopen_draft(ID)
        .expect("read draft from SQLite")
        .expect("saved draft must exist")
}
