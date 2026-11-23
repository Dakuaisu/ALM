//! L06-02 G4: standalone GST bills in the device database and the backup archive.
//! Local save only (`persist()` → `saveDB()`); sync and shared numbering are out of scope.
use std::num::NonZeroU32;

use alm_store::confirmation::{
    ConfirmationFixture, GstBillDraft, GstBillError, GstBillId, GstBillInvoice, GstBillLedger,
    GstBillLineInput, GstBillNumber, GstBillStoreError, GstUnit,
};
use alm_store::{
    BillDate, DraftId, DraftStorageError, GstBillDatabase, LocalSales, Money, ProductId,
    ProvisionalDraft, Role, SqliteSales,
};

#[path = "gst_bill_store/archive.rs"]
mod archive;
#[path = "common/target_scratch.rs"]
mod target_scratch;

const OWNER: Role = Role::Owner;
const LINES: [GstBillLineInput<'static>; 2] = [
    GstBillLineInput {
        item: "Synthetic Cola",
        product: Some(ProductId::new(1)),
        hsn: "22029090",
        quantity: 2,
        inclusive_rate: Money::from_minor_units(14_000),
        gst_basis_points: 4_000,
        packet_qty: 24,
        ..GstBillLineInput::BLANK
    },
    GstBillLineInput {
        item: "Synthetic Water",
        unit: GstUnit::Piece,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_500),
        gst_basis_points: 500,
        ..GstBillLineInput::BLANK
    },
];

fn today() -> BillDate {
    BillDate::new(2025, 4, 2).unwrap()
}

fn fixture() -> ConfirmationFixture {
    ConfirmationFixture::new(ProductId::new(1), 5, Money::from_minor_units(14_000)).unwrap()
}

/// A draft with every `saveGstBill` field set, numbered by `newGstDraft`.
fn full_draft(fixture: &ConfirmationFixture, id: u64) -> GstBillDraft<'static> {
    GstBillDraft {
        customer_id: "cust-7",
        customer: "Synthetic Buyer",
        gstin: Some("27ABCDE1234F1Z5"),
        cust_addr: "12 Market Road",
        cust_phone: "9000000001",
        cust_email: "buyer@example.test",
        cust_state: "Maharashtra",
        cust_state_code: "27",
        pay_mode: "Credit",
        pay_terms: "15 days",
        reverse_charge: "Yes",
        transporter: "Synthetic Roadways",
        vehicle_no: "MH12AB1234",
        lr_no: "LR-9",
        eway_number: Some("1234-5678-9012"),
        terms_delivery: "Door delivery",
        dispatch: "Lucknow",
        now: 1_700_000_000_000 + id,
        lines: &LINES,
        ..fixture
            .new_gst_bill_draft(OWNER, GstBillId(id), today())
            .unwrap()
    }
}

fn invoice(fixture: &ConfirmationFixture, id: u64) -> Result<GstBillInvoice, GstBillError> {
    let mut emitted = None;
    fixture.emit_gst_bill(OWNER, GstBillId(id), &mut |invoice| {
        emitted = Some(invoice.clone())
    })?;
    Ok(emitted.unwrap())
}

#[test]
fn gst_bills_survive_reopen_with_numbers_cancel_flags_fields_and_markers() {
    let scratch = target_scratch::Scratch::new("gst-bill-store-");
    let mut fixture = fixture();
    let before = fixture.clone();
    let mut database = GstBillDatabase::open_scoped(scratch.path(), "A").unwrap();
    for id in 1..=3 {
        let draft = full_draft(&fixture, id);
        assert_eq!(draft.number, GstBillNumber(id));
        fixture
            .save_gst_bill_to(OWNER, &draft, &mut database, &mut |_| {})
            .unwrap();
    }
    fixture
        .cancel_gst_bill_to(OWNER, GstBillId(2), &mut database)
        .unwrap();
    fixture
        .delete_gst_bill_to(OWNER, GstBillId(3), &mut database)
        .unwrap();
    let saved = fixture.gst_bill_ledger(OWNER).unwrap();
    let created = saved.bills[0].created;
    let edit = GstBillDraft {
        number: GstBillNumber(1),
        customer: "Edited Buyer",
        created,
        now: 1_900_000_000_000,
        ..full_draft(&fixture, 1)
    };
    fixture
        .save_gst_bill_to(OWNER, &edit, &mut database, &mut |_| {})
        .unwrap();
    let expected = fixture.gst_bill_ledger(OWNER).unwrap();
    assert_eq!(expected.deleted, [GstBillId(3)]);
    assert_eq!(
        expected
            .bills
            .iter()
            .map(|bill| (bill.id.0, bill.number.0, bill.cancelled, bill.created))
            .collect::<Vec<_>>(),
        [
            (1, 1, false, 1_700_000_000_001),
            (2, 2, true, 1_700_000_000_002)
        ]
    );
    assert_eq!(expected.bills[0].customer, "Edited Buyer");
    let invoices = [invoice(&fixture, 1).unwrap(), invoice(&fixture, 2).unwrap()];
    drop(database);

    // A fresh connection and a fresh fixture see exactly the committed bills.
    let reopened = GstBillDatabase::open_scoped(scratch.path(), "A").unwrap();
    let mut restarted = self::fixture();
    let restarted_before = restarted.clone();
    assert!(matches!(
        restarted.load_gst_bills_from(Role::Manager, &reopened),
        Err(GstBillStoreError::Bill(GstBillError::AccessDenied))
    ));
    restarted.load_gst_bills_from(OWNER, &reopened).unwrap();
    assert_eq!(restarted.gst_bill_ledger(OWNER).unwrap(), expected);
    assert_eq!(restarted.next_gst_bill_number(OWNER), Ok(GstBillNumber(3)));
    assert_eq!(
        [
            invoice(&restarted, 1).unwrap(),
            invoice(&restarted, 2).unwrap()
        ],
        invoices
    );
    assert_eq!(invoice(&restarted, 3), Err(GstBillError::NotFound));
    let mut reopened = reopened;
    assert!(matches!(
        restarted.save_gst_bill_to(
            OWNER,
            &full_draft(&restarted, 3),
            &mut reopened,
            &mut |_| { panic!("deleted bill saved") }
        ),
        Err(GstBillStoreError::Bill(GstBillError::Deleted))
    ));
    assert!(matches!(
        restarted.save_gst_bill_to(
            OWNER,
            &full_draft(&restarted, 2),
            &mut reopened,
            &mut |_| { panic!("cancelled bill edited") }
        ),
        Err(GstBillStoreError::Bill(GstBillError::Cancelled))
    ));

    // The account's draft store still opens the same file.
    let mut drafts = LocalSales::open_manager_account(scratch.path(), "A").unwrap();
    let draft = ProvisionalDraft::new(DraftId::new(1), Vec::new());
    drafts.save_draft(draft.clone()).unwrap();
    assert_eq!(drafts.reopen_draft(draft.id()).unwrap(), Some(draft));
    assert!(matches!(
        GstBillDatabase::open_scoped(scratch.path(), "a b"),
        Err(DraftStorageError::InvalidAccountScope)
    ));

    assert_eq!(fixture.without_gst_bills(), before.without_gst_bills());
    assert_eq!(
        restarted.without_gst_bills(),
        restarted_before.without_gst_bills()
    );
}

#[test]
fn failed_gst_writes_roll_back_both_the_database_and_memory() {
    let scratch = target_scratch::Scratch::new("gst-bill-rollback-");
    let path = scratch.path().join("legacy.sqlite3");
    let mut fixture = fixture();
    let before = fixture.clone();
    let mut database = GstBillDatabase::open(&path).unwrap();
    let original = full_draft(&fixture, 1);
    fixture
        .save_gst_bill_to(OWNER, &original, &mut database, &mut |_| {})
        .unwrap();
    let committed = fixture.clone();
    let ledger = fixture.gst_bill_ledger(OWNER).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_gst_line BEFORE INSERT ON standalone_gst_bill_lines
             BEGIN SELECT RAISE(ABORT, 'synthetic GST line failure'); END;
             CREATE TRIGGER reject_gst_marker BEFORE INSERT ON standalone_gst_deleted
             BEGIN SELECT RAISE(ABORT, 'synthetic GST marker failure'); END;",
        )
        .unwrap();
    let edit = GstBillDraft {
        customer: "Rejected edit",
        lines: &LINES[..1],
        ..original
    };
    let rejected = fixture.save_gst_bill_to(OWNER, &edit, &mut database, &mut |_| {
        panic!("failed save emitted")
    });
    assert!(matches!(rejected, Err(GstBillStoreError::Store(_))));
    let rejected = fixture.delete_gst_bill_to(OWNER, GstBillId(1), &mut database);
    assert!(matches!(rejected, Err(GstBillStoreError::Store(_))));
    assert_eq!(fixture, committed);
    drop(database);
    let mut restarted = self::fixture();
    restarted
        .load_gst_bills_from(OWNER, &GstBillDatabase::open(&path).unwrap())
        .unwrap();
    assert_eq!(restarted.gst_bill_ledger(OWNER).unwrap(), ledger);
    assert_eq!(fixture.without_gst_bills(), before.without_gst_bills());
}

#[test]
fn stored_lines_keep_unit_pack_size_and_inert_product_reference() {
    let scratch = target_scratch::Scratch::new("gst-bill-lines-");
    let mut fixture = fixture();
    let before = fixture.clone();
    let stock = fixture.product_stock(ProductId::new(1));
    let mut database = GstBillDatabase::open(&scratch.path().join("lines.sqlite3")).unwrap();
    fixture
        .save_gst_bill_to(OWNER, &full_draft(&fixture, 1), &mut database, &mut |_| {})
        .unwrap();
    let stored = database_ledger(&database);
    let lines = &stored.bills[0].lines;
    assert_eq!(
        (
            lines[0].product,
            lines[0].unit,
            lines[0].packet_qty,
            lines[0].hsn.as_str()
        ),
        (
            Some(ProductId::new(1)),
            GstUnit::Case,
            NonZeroU32::new(24).unwrap(),
            "22029090"
        )
    );
    assert_eq!(
        (
            lines[1].product,
            lines[1].unit,
            lines[1].packet_qty,
            lines[1].hsn.as_str()
        ),
        (None, GstUnit::Piece, NonZeroU32::MIN, "")
    );
    assert_eq!(fixture.product_stock(ProductId::new(1)), stock);
    assert_eq!(fixture.without_gst_bills(), before.without_gst_bills());
}

fn database_ledger(database: &GstBillDatabase) -> GstBillLedger {
    use alm_store::confirmation::GstBillStore;
    database.load_gst_bills().unwrap()
}
