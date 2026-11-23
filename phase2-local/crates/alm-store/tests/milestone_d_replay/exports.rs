use std::ffi::OsString;
use std::fs;
use std::path::Path;

use alm_store::confirmation::{ConfirmationError, ConfirmationFixture};
use alm_store::{DraftId, ProductId, Role};

#[path = "exports/clipboard.rs"]
mod clipboard;
#[path = "exports/staff.rs"]
mod staff;

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const SALE: DraftId = DraftId::new(21);
const A: ProductId = ProductId::new(1);
const B: ProductId = ProductId::new(2);
const ITEM_A: &str = "Synthetic A";
const ITEM_B: &str = "Synthetic B";
// SpreadsheetML: unrun; the local Rust slice has no typed workbook exporter.
const SALES_CSV: &str = concat!(
    "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n",
    "2025-02-21,1,Synthetic Buyer,1,17700,2,41000,10000,CANCELLED\n",
    "2025-02-21,1,Synthetic Buyer,2,5600,1,,,CANCELLED",
);

pub(super) fn exercise(fixture: &ConfirmationFixture, dir: &Path) {
    let before = directory_entries(dir);
    assert_eq!(
        fixture.sales_csv(MANAGER, SALE),
        Err(ConfirmationError::AccessDenied)
    );
    staff::exercise(fixture);
    clipboard::exercise(fixture);
    assert_eq!(directory_entries(dir), before, "inert paths wrote a file");

    let csv = fixture
        .sales_csv(OWNER, SALE)
        .expect("cancelled C sale CSV");
    let path = dir.join("sales.csv");
    assert_eq!(csv.as_bytes(), SALES_CSV.as_bytes());
    fs::write(&path, csv.as_bytes()).expect("write Owner Sales CSV");
    assert_eq!(
        fs::read(path).expect("read Owner Sales CSV"),
        SALES_CSV.as_bytes()
    );
}

fn directory_entries(dir: &Path) -> Vec<OsString> {
    let mut names = fs::read_dir(dir)
        .expect("open replay output directory")
        .map(|entry| entry.expect("read replay directory entry").file_name())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names
}
