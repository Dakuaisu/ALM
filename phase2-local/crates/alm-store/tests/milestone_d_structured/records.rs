use alm_core::confirmation::{ConfirmationFixture, CustomerLedgerKind, ReceiptMode};
use alm_core::synthetic_read_model::{
    Component, Customer, HistoricalGst, Line, LineKind, Product, Receipt, Records, Sale,
};
use alm_core::{BillDate, DraftCustomerId, Role};

use super::fixture::{BUYER_ID, ITEM_A, ITEM_B, SALE};
use super::{gst, rate_list};

pub(super) fn capture(fixture: &ConfirmationFixture) -> (Records, HistoricalGst) {
    let (review, receipt) = fixture.current_sale(Role::Owner, SALE).unwrap();
    let lines = review
        .draft()
        .lines()
        .iter()
        .zip(review.unit_prices())
        .enumerate()
        .map(|(i, (line, price))| Line {
            item: if i == 0 { ITEM_A } else { ITEM_B }.into(),
            pack: "12".into(),
            uom: "pkt".into(),
            packet_quantity: 12,
            kind: LineKind::Item,
            rate_minor: price.minor_units(),
            quantity: line.quantity().get(),
            amount_minor: price.minor_units() * i64::from(line.quantity().get()),
        })
        .collect();
    let cancelled = Sale {
        id: "sale-cancelled-id".into(),
        date: review.draft().bill_date().unwrap(),
        number: receipt.invoice,
        customer: "Synthetic Buyer".into(),
        mode: "Cash".into(),
        lines,
        total_minor: receipt.total.minor_units(),
        paid_minor: receipt.received.unwrap().minor_units(),
        cancelled: true,
    };
    let active = Sale {
        id: "sale-active-id".into(),
        date: BillDate::new(2025, 1, 1).unwrap(),
        number: 99,
        customer: "Control".into(),
        mode: "Online".into(),
        lines: vec![
            Line {
                item: "Control item".into(),
                pack: "12".into(),
                uom: "pc".into(),
                packet_quantity: 12,
                kind: LineKind::Item,
                rate_minor: 100,
                quantity: 6,
                amount_minor: 600,
            },
            Line {
                item: "Control bundle".into(),
                pack: "".into(),
                uom: "".into(),
                packet_quantity: 1,
                kind: LineKind::Crate {
                    components: vec![Component {
                        item: "hidden-component-id".into(),
                        quantity: 3,
                    }],
                },
                rate_minor: 250,
                quantity: 2,
                amount_minor: 500,
            },
        ],
        total_minor: 1100,
        paid_minor: 300,
        cancelled: false,
    };
    let ledger = fixture
        .customer_ledger(Role::Owner, &DraftCustomerId::new(BUYER_ID.into()))
        .unwrap();
    let [receipt_entry] = ledger.entries.as_slice() else {
        panic!("expected C receipt")
    };
    let CustomerLedgerKind::Receipt { mode } = receipt_entry.kind else {
        panic!("expected receipt")
    };
    let mode = match mode {
        ReceiptMode::Cash => "Cash",
        ReceiptMode::Online => "Online",
    };
    let tax = gst::snapshot(&cancelled, review.draft().lines());
    let mut products = vec![Product {
        item: ITEM_A.into(),
        company: "Synthetic".into(),
        pack: "12".into(),
        uom: "pkt".into(),
        packet_quantity: 12,
        mrp_minor: 20_000,
        buy_minor: 10_000,
        sell_minor: 17_700,
        purchased: 5,
        sold: 0,
        on_hand: 5,
        reorder: 5,
    }];
    products.extend(rate_list::catalog());
    let records = Records {
        reorder_default: 5,
        products,
        customers: vec![Customer {
            name: "Synthetic Buyer".into(),
            mobile: "".into(),
            address: "".into(),
            balance_minor: 0,
        }],
        sales: vec![cancelled, active],
        receipts: vec![Receipt {
            id: "receipt-standalone-id".into(),
            date: receipt_entry.date,
            mode: mode.into(),
            customer: "Synthetic Buyer".into(),
            amount_minor: receipt_entry.credit.minor_units(),
        }],
    };
    (records, tax)
}
