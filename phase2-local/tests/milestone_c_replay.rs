use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, CreationKey, CustomerReceiptInput,
    CustomerReceiptMetadata, EditChanges, GstInvoice, Receipt, ReceiptMode,
};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId, Role,
};

#[path = "milestone_c_replay/accounts.rs"]
mod accounts;
#[path = "milestone_c_replay/books.rs"]
mod books;
#[path = "milestone_c_replay/documents.rs"]
mod documents;
#[path = "milestone_c_replay/inventory.rs"]
mod inventory;
#[path = "milestone_c_replay/start.rs"]
mod start;

use accounts::{AccountCheckpoint, AccountReadback, assert_accounts, march_receipt_date};
use documents::{BillVersion, EstimateCheckpoint, assert_estimate, assert_gst};
use inventory::{StockCheckpoint, assert_stock, receive_dated_stock};

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const DENIED: ConfirmationError = ConfirmationError::AccessDenied;
const A: ProductId = ProductId::new(1);
const B: ProductId = ProductId::new(2);
const ITEM_A: &str = "Synthetic A";
const ITEM_B: &str = "Synthetic B";
const SALE: DraftId = DraftId::new(21);
const BUYER: &str = "Synthetic Buyer";
const BUYER_ID: &str = "milestone-c-buyer";
const PAID: Money = money(10_000);
const RECEIPT_CREATION_KEY: CreationKey = CreationKey::new(2);
const POSTED_CSV_ROWS: &[&str] = &["2025-02-21,1,Synthetic Buyer,1,11800,2,23600,10000,"];

const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}

const fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("positive synthetic packet count")
}

fn feb(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).expect("valid February fixture date")
}

#[test]
fn milestone_c_replays_part_paid_sale_edit_and_cancellation() {
    // Given: zero opening stock. Two dated PO receipts arrive in reverse date order (L11-01).
    let mut fixture = ConfirmationFixture::new(A, 0, start::INITIAL_RATE).unwrap();
    fixture.add_product(B, 0, money(5_600)).unwrap();
    receive_dated_stock(&mut fixture);
    assert_stock(&fixture, StockCheckpoint::Received);

    // When: an explicit Owner confirmation posts the part-paid named bill (L03-02).
    let buyer = DraftCustomerId::new(BUYER_ID.into());
    let (review, posted) = start::post_sale(&mut fixture, &buyer);
    assert_eq!(posted, saved_receipt(2, 23_600, (13_600, 13_600)));
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &posted)));
    let posted_accounts = assert_accounts(&fixture, &buyer, AccountCheckpoint::Posted);
    books::assert_books(&fixture, &buyer, AccountCheckpoint::Posted);
    assert_stock(&fixture, StockCheckpoint::Posted);
    assert_history(&fixture, &posted, POSTED_CSV_ROWS);
    let posted_gst = assert_gst(&fixture, BillVersion::Original, false);
    assert_estimate(
        &fixture,
        BillVersion::Original,
        EstimateCheckpoint {
            today: feb(21),
            latest_label: "Total Due (as on 21-Feb-2025)",
            latest_due: "₹136.00",
            cancelled: false,
        },
    );

    // When: Owner records a later Online receipt without allocating it to the bill (L14-01).
    let receipt_date = march_receipt_date();
    let input = CustomerReceiptInput {
        customer_id: buyer.clone(),
        date: receipt_date,
        amount: money(5_000),
        mode: ReceiptMode::Online,
    };
    let metadata = CustomerReceiptMetadata {
        selected_name: BUYER.into(),
        creation_key: RECEIPT_CREATION_KEY,
    };
    let denied =
        fixture.record_customer_receipt_with_metadata(MANAGER, input.clone(), metadata.clone());
    assert_eq!(denied, Err(DENIED));
    assert_eq!(AccountReadback::open(&fixture, &buyer), posted_accounts);
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &posted)));
    println!("Mar 1: Manager receipt={denied:?}; Owner readbacks unchanged");
    fixture
        .record_customer_receipt_with_metadata(OWNER, input, metadata)
        .unwrap();
    let receipt_accounts = assert_accounts(&fixture, &buyer, AccountCheckpoint::OnlineReceipt);
    books::assert_books(&fixture, &buyer, AccountCheckpoint::OnlineReceipt);
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &posted)));
    assert_history(&fixture, &posted, POSTED_CSV_ROWS);
    assert_stock(&fixture, StockCheckpoint::Posted);
    assert_eq!(
        assert_gst(&fixture, BillVersion::Original, false),
        posted_gst
    );
    let march_estimate = |latest_due, cancelled| EstimateCheckpoint {
        today: receipt_date,
        latest_label: "Total Due (as on 01-Mar-2025)",
        latest_due,
        cancelled,
    };
    assert_estimate(
        &fixture,
        BillVersion::Original,
        march_estimate("₹86.00", false),
    );

    // When: Owner saves repricing and an appended line on the same invoice (L04-01).
    let changes = EditChanges {
        retained_price: money(17_700),
        appended_line: DraftLine::new(B, qty(1)),
        appended_price: money(5_600),
    };
    assert_eq!(fixture.begin_edit(MANAGER, SALE, changes), Err(DENIED));
    let edit = fixture.begin_edit(OWNER, SALE, changes).unwrap();
    assert_eq!(fixture.save_edit(MANAGER, &edit), Err(DENIED));
    assert_eq!(AccountReadback::open(&fixture, &buyer), receipt_accounts);
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &posted)));
    let edited = fixture.save_edit(OWNER, &edit).unwrap();
    assert_eq!(edited, saved_receipt(3, 41_000, (31_000, 26_000)));
    let edited_accounts = assert_accounts(&fixture, &buyer, AccountCheckpoint::Edited);
    books::assert_books(&fixture, &buyer, AccountCheckpoint::Edited);
    let (saved, _) = fixture.current_sale(OWNER, SALE).unwrap();
    assert_eq!(
        saved.draft().lines(),
        [DraftLine::new(A, qty(2)), DraftLine::new(B, qty(1))]
    );
    assert_eq!(saved.unit_prices(), [money(17_700), money(5_600)]);
    let saved = saved.clone();
    assert_stock(&fixture, StockCheckpoint::Edited);
    assert_history(
        &fixture,
        &edited,
        &[
            "2025-02-21,1,Synthetic Buyer,1,17700,2,41000,10000,",
            "2025-02-21,1,Synthetic Buyer,2,5600,1,,,",
        ],
    );
    let edited_gst = assert_gst(&fixture, BillVersion::Edited, false);
    assert_estimate(
        &fixture,
        BillVersion::Edited,
        march_estimate("₹260.00", false),
    );
    let saved_close = books::SavedCashBookClose::save(&fixture);

    // Then: reversal restores the lots but leaves the saved document in history (L04-02).
    assert_eq!(fixture.cancel_sale(MANAGER, SALE), Err(DENIED));
    assert_eq!(AccountReadback::open(&fixture, &buyer), edited_accounts);
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&saved, &edited)));
    assert_stock(&fixture, StockCheckpoint::Edited);
    fixture.cancel_sale(OWNER, SALE).unwrap();
    assert_accounts(&fixture, &buyer, AccountCheckpoint::Cancelled);
    books::assert_books(&fixture, &buyer, AccountCheckpoint::Cancelled);
    saved_close.assert_after_cancel(&fixture);
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&saved, &edited)));
    assert_stock(&fixture, StockCheckpoint::Cancelled);
    assert_history(
        &fixture,
        &edited,
        &[
            "2025-02-21,1,Synthetic Buyer,1,17700,2,41000,10000,CANCELLED",
            "2025-02-21,1,Synthetic Buyer,2,5600,1,,,CANCELLED",
        ],
    );
    let cancelled_gst = assert_gst(&fixture, BillVersion::Edited, true);
    assert_eq!(
        cancelled_gst,
        GstInvoice {
            cancelled: true,
            ..edited_gst
        }
    );
    assert_estimate(
        &fixture,
        BillVersion::Edited,
        march_estimate("₹-50.00", true),
    );
}

const fn saved_receipt(quantity: u32, total: i64, (due, customer_balance): (i64, i64)) -> Receipt {
    Receipt {
        draft_id: SALE,
        invoice: 1,
        quantity,
        total: money(total),
        received: Some(money(10_000)),
        due: Some(money(due)),
        customer_balance: Some(money(customer_balance)),
    }
}

fn assert_history(fixture: &ConfirmationFixture, expected: &Receipt, rows: &[&str]) {
    assert_eq!(fixture.current_sale(MANAGER, SALE), Err(DENIED));
    assert_eq!(fixture.sales_csv(MANAGER, SALE), Err(DENIED));
    let (_, current) = fixture.current_sale(OWNER, SALE).unwrap();
    assert_eq!(current, expected);
    assert_eq!(fixture.posted_count(), 1);
    let csv = fixture.sales_csv(OWNER, SALE).unwrap();
    let mut actual = csv.lines();
    assert_eq!(
        actual.next(),
        Some("Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status")
    );
    assert_eq!(actual.collect::<Vec<_>>(), rows);
    println!("current sale: {current:?}; Sales CSV:\n{csv}");
}
