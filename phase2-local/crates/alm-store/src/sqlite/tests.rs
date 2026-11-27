use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, DurableDraftStore, LocalSales,
    Money, ProductId, ProvisionalDraft, Role, decode_crate_row,
};

use super::SqliteDraftStore;
use crate::SqliteSales;

// Frozen copy of the DDL the sqlite3 CLI store fed to `sqlite3` before the driver switch.
const CLI_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS provisional_drafts (
    id TEXT PRIMARY KEY NOT NULL CHECK (
        length(id) BETWEEN 1 AND 20 AND instr(id, char(0)) = 0
        AND id NOT GLOB '*[^0-9]*' AND (id = '0' OR id NOT GLOB '0*')
        AND (length(id) < 20 OR id <= '18446744073709551615')
    )
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_lines (
    draft_id TEXT NOT NULL REFERENCES provisional_drafts(id),
    position INTEGER NOT NULL CHECK (position >= 0),
    product_id TEXT NOT NULL CHECK (
        length(product_id) BETWEEN 1 AND 20 AND instr(product_id, char(0)) = 0
        AND product_id NOT GLOB '*[^0-9]*'
        AND (product_id = '0' OR product_id NOT GLOB '0*')
        AND (length(product_id) < 20 OR product_id <= '18446744073709551615')
    ),
    quantity INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 4294967295),
    PRIMARY KEY (draft_id, position)
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_customers (
    draft_id TEXT PRIMARY KEY NOT NULL REFERENCES provisional_drafts(id),
    kind INTEGER NOT NULL CHECK (kind IN (1, 2)),
    customer_id TEXT,
    name TEXT NOT NULL,
    CHECK ((kind = 1 AND customer_id IS NULL) OR (kind = 2 AND customer_id IS NOT NULL))
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_crates (
    draft_id TEXT PRIMARY KEY NOT NULL REFERENCES provisional_drafts(id),
    definition TEXT NOT NULL CHECK (
        length(definition) > 0 AND instr(definition, char(0)) = 0
        AND definition NOT GLOB '*[^0-9,;]*'
    )
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_dates (
    draft_id TEXT PRIMARY KEY NOT NULL REFERENCES provisional_drafts(id),
    bill_date TEXT NOT NULL CHECK (
        length(bill_date) = 10 AND instr(bill_date, char(0)) = 0
        AND bill_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    )
) STRICT;
";

const CLI_PENDING: &str = "
CREATE TABLE IF NOT EXISTS pending_owner_confirmations (
    request_id TEXT PRIMARY KEY NOT NULL,
    snapshot TEXT NOT NULL
) STRICT;
";

// Rows exactly as the CLI `save` path wrote them, with hex-cast text literals.
const CLI_ROWS: &str = "
PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
INSERT INTO provisional_drafts (id) VALUES ('43') ON CONFLICT(id) DO NOTHING;
INSERT INTO provisional_draft_dates VALUES ('43', '2024-02-29');
INSERT INTO provisional_draft_customers VALUES ('43', 2, CAST(X'63726174652D6275796572' AS TEXT), CAST(X'53796E74686574696320E282B9' AS TEXT));
INSERT INTO provisional_draft_lines (draft_id, position, product_id, quantity) VALUES ('43', 0, '1', 2);
INSERT INTO provisional_draft_lines (draft_id, position, product_id, quantity) VALUES ('43', 1, '12', 3);
INSERT INTO provisional_draft_crates VALUES ('43', CAST(X'323B322C392C303B312C322C31' AS TEXT));
INSERT INTO provisional_drafts (id) VALUES ('44') ON CONFLICT(id) DO NOTHING;
INSERT INTO provisional_draft_customers VALUES ('44', 1, NULL, CAST(X'27576F726B27' AS TEXT));
COMMIT;
";

struct Scratch(PathBuf);

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

impl Scratch {
    fn new() -> Self {
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        fs::create_dir_all(&target).unwrap();
        Self(scratch_retry::create(&target, "cli-schema-").expect("create scratch"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("remove scratch: {error}");
        }
    }
}

fn line(product: u64, quantity: u32) -> DraftLine {
    DraftLine::new(ProductId::new(product), NonZeroU32::new(quantity).unwrap())
}

fn expected_drafts() -> [ProvisionalDraft; 2] {
    let mut crated = ProvisionalDraft::new(DraftId::new(43), vec![line(1, 2), line(12, 3)])
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("crate-buyer".into()),
            name: "Synthetic ₹".into(),
        })
        .with_bill_date(Some(BillDate::new(2024, 2, 29).unwrap()));
    crated.add_crate_row(decode_crate_row("2;2,9,0;1,2,1").unwrap());
    let typed = ProvisionalDraft::new(DraftId::new(44), vec![])
        .with_customer(DraftCustomer::Typed("'Work'".into()));
    [crated, typed]
}

fn cli_database(path: &Path, extra: &str) {
    Connection::open(path)
        .unwrap()
        .execute_batch(&format!(
            "BEGIN IMMEDIATE;\n{CLI_SCHEMA}\n{extra}\nCOMMIT;\n{CLI_ROWS}"
        ))
        .unwrap();
}

#[test]
fn legacy_cli_database_opens_and_round_trips_with_the_embedded_driver() {
    // Given: a legacy file built by the CLI code path, including a pending approval.
    let scratch = Scratch::new();
    let path = scratch.0.join("legacy.sqlite3");
    cli_database(&path, CLI_PENDING);
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "INSERT INTO pending_owner_confirmations VALUES ('987', CAST(X'367C327C61' AS TEXT));",
        )
        .unwrap();

    // When: the embedded driver opens the file.
    let mut sales = LocalSales::open_sqlite(&path, Money::from_minor_units(0)).unwrap();
    let store = SqliteDraftStore::open(&path).unwrap();

    // Then: every stored draft and approval decodes unchanged, and saves still work.
    let [crated, typed] = expected_drafts();
    let mut manager = sales.session(Role::Manager);
    assert_eq!(
        manager.reopen_draft(DraftId::new(43)).unwrap(),
        Some(crated)
    );
    assert_eq!(
        manager.reopen_draft(DraftId::new(44)).unwrap(),
        Some(typed.clone())
    );
    assert_eq!(store.load_pending(987).unwrap().as_deref(), Some("6|2|a"));
    let replaced = typed.with_bill_date(Some(BillDate::new(2024, 3, 1).unwrap()));
    manager.save_draft(replaced.clone()).unwrap();
    assert_eq!(
        manager.reopen_draft(DraftId::new(44)).unwrap(),
        Some(replaced)
    );
}

#[test]
fn scoped_cli_database_opens_for_its_bound_account_only() {
    // Given: a scoped file initialized by the CLI code path for account "A".
    let scratch = Scratch::new();
    let directory = scratch.0.join("manager-accounts");
    fs::create_dir(&directory).unwrap();
    cli_database(
        &directory.join("account-41.sqlite3"),
        "CREATE TABLE manager_account_scope (account_hex TEXT PRIMARY KEY NOT NULL) STRICT;\n\
         INSERT INTO manager_account_scope VALUES ('41');",
    );

    // When: the embedded driver opens the account slot.
    let store = LocalSales::open_manager_account(&scratch.0, "A").unwrap();

    // Then: the old drafts reopen unchanged under the same binding.
    for expected in expected_drafts() {
        assert_eq!(store.reopen_draft(expected.id()).unwrap(), Some(expected));
    }
}
