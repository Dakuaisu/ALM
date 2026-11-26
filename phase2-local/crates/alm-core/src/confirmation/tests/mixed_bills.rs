//! OPEN-013 mixed bills: New Bill's crate rows beside ordinary lines on one bill (HTML L3640
//! `draft.items` as `crateRow` or `lineRow`; `saveBill` L4022 with `linePackets` and
//! `lineUnitCost`'s crate branches).
use super::super::demo_fixture::{customer_id, directory, fixture};
use super::super::manual_cashbook::ManualCashBook;
use super::super::owner_billing::{OwnerPosting, SaleTerms, priced_draft};
use super::super::{
    BalanceMode, ConfirmationError, CrateTemplate, ProductTerms, SalesPeriod, pending,
};
use super::*;
use crate::synthetic_read_model::LineKind;
use crate::{CrateComponent, DraftCustomer, SaleUnit};

const LIME: ProductId = ProductId::new(1);
const SODA: ProductId = ProductId::new(2);
const BILL: DraftId = DraftId::new(41);

fn day() -> BillDate {
    BillDate::new(2025, 2, 7).unwrap()
}

fn rs(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

fn n(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

/// The demo shop with a Lime piece at half a packet and a "Lime Twins" crate (one Lime packet
/// and two Lime pieces, ₹300): the template repeats the product also sold as an ordinary line.
fn shop() -> (ConfirmationFixture, crate::confirmation::CrateTemplateId) {
    let mut shop = fixture().unwrap();
    shop.set_checkout_product(
        LIME,
        ProductTerms {
            pack: n(2),
            piece_price: rs(6_100),
            case_price: rs(12_050),
        },
    )
    .unwrap();
    let template = shop
        .save_crate_template(
            Role::Owner,
            CrateTemplate {
                name: "Lime Twins".into(),
                components: vec![
                    CrateComponent::new(LIME, n(1), SaleUnit::Case),
                    CrateComponent::new(LIME, n(2), SaleUnit::Piece),
                ],
                price: rs(30_000),
            },
        )
        .unwrap();
    (shop, template)
}

/// Two Lime and one Soda packet, then two Lime Twins crates, ₹1.50 loading a packet.
fn mixed_draft(
    shop: &ConfirmationFixture,
    template: crate::confirmation::CrateTemplateId,
) -> ProvisionalDraft {
    let mut draft = ProvisionalDraft::new(
        BILL,
        vec![DraftLine::new(LIME, n(2)), DraftLine::new(SODA, n(1))],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: super::super::demo_fixture::CUSTOMER.into(),
    })
    .with_bill_date(Some(day()))
    .with_loading_rate(rs(150));
    shop.load_crate_template(Role::Owner, template, n(2), &mut draft)
        .unwrap();
    draft
}

fn post(shop: &mut ConfirmationFixture, draft: &ProvisionalDraft, paid: &str) -> Receipt {
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            shop,
            &directory(),
            Role::Owner,
            Some(draft),
            SaleTerms {
                date: "2025-02-07",
                paid,
            },
        )
        .unwrap();
    let serial = posting.pending(Role::Owner).unwrap().serial();
    posting.confirm(shop, Role::Owner, serial).unwrap()
}

#[test]
fn a_mixed_bill_posts_lines_then_each_crate_component_from_fifo() {
    let (mut shop, template) = shop();
    let lime = shop.product_stock(LIME).unwrap();
    let soda = shop.product_stock(SODA).unwrap();
    let draft = mixed_draft(&shop, template);
    assert_eq!((draft.lines().len(), draft.crate_rows().len()), (2, 1));

    let priced = priced_draft(&shop, Role::Owner, &draft).unwrap();
    let (lime_rate, soda_rate) = (priced.lines[0].unit_price, priced.lines[1].unit_price);
    assert_eq!(priced.crates.len(), 1);
    assert_eq!(
        (priced.crates[0].name.as_str(), priced.crates[0].amount),
        ("Lime Twins", rs(60_000))
    );
    // Packets: 2 + 1 lines, then 2 crates x (1 packet + 2 pieces at 2 a packet) = 4.
    assert_eq!(priced.loading, rs(150 * 7));
    let items = 2 * lime_rate.minor_units() + soda_rate.minor_units() + 60_000;
    assert_eq!(priced.total, rs(items + 150 * 7));
    assert_eq!(
        priced_draft(&shop, Role::Manager, &draft),
        Err(ConfirmationError::AccessDenied)
    );

    let receipt = post(&mut shop, &draft, "200");
    assert_eq!((receipt.quantity, receipt.total), (7, priced.total));
    assert_eq!(shop.product_stock(LIME), Some(lime - 6));
    assert_eq!(shop.product_stock(SODA), Some(soda - 1));

    // FIFO per component: the ordinary lines first (positions 0, 1), then each crate component
    // as its own demand (2, 3), a repeated product taking from its lots per component (OPEN-010).
    let book = shop
        .owner_book(Role::Owner, &ManualCashBook::default())
        .unwrap();
    let sale = book.sales.iter().find(|s| s.draft == BILL).unwrap();
    let taken: Vec<_> = sale
        .allocations
        .iter()
        .map(|a| (a.line, a.product, a.quantity))
        .collect();
    assert_eq!(taken, [(0, 1, 2), (1, 2, 1), (2, 1, 2), (3, 1, 2)]);
    assert_eq!(sale.crate_rows.len(), 1);
    assert_eq!(sale.prices.len(), 3);
    let items: Vec<_> = sale.items.iter().map(|(p, _)| *p).collect();
    assert_eq!(items, [1, 2, 1, 1]);
}

#[test]
fn a_mixed_bill_is_listed_imaged_and_archived_with_one_line_per_item_and_crate() {
    let (mut shop, template) = shop();
    let draft = mixed_draft(&shop, template);
    let receipt = post(&mut shop, &draft, "200");

    let preview = shop
        .bill_preview(Role::Owner, BILL, BalanceMode::AsOnBillDate)
        .unwrap();
    let labels: Vec<_> = preview.lines.iter().map(|l| l.label.clone()).collect();
    assert_eq!(labels, [None, None, Some("Crate: Lime Twins".into())]);
    // `totQty`: 2 + 1 line quantities and 2 crates.
    assert_eq!(preview.items_quantity, 5);
    assert_eq!(
        (preview.total, preview.loading),
        (receipt.total, rs(150 * 7))
    );
    assert_eq!(preview.lines[2].amount, rs(60_000));

    for search in ["", "lime twins", "synthetic soda"] {
        let history = shop
            .sales_history(Role::Owner, day(), SalesPeriod::All, search)
            .unwrap();
        assert_eq!(history.rows.len(), 1, "{search}");
        assert_eq!(history.active_packets, 7);
    }
    let snapshot = shop
        .projection(Role::Owner, &directory(), &ManualCashBook::default())
        .unwrap();
    let listed = snapshot.sales.iter().find(|s| s.draft_id == BILL).unwrap();
    assert_eq!(
        (listed.lines, listed.items.as_slice()),
        (
            3,
            &[
                "Synthetic Lime".to_owned(),
                "Synthetic Soda".to_owned(),
                "Lime Twins".to_owned()
            ][..]
        )
    );
    assert_eq!(snapshot.sales_unsupported, None);

    let records = shop.archive_records(Role::Owner, &directory()).unwrap();
    let sale = &records.sales[0];
    assert!(matches!(sale.lines[0].kind, LineKind::Item));
    assert!(matches!(sale.lines[1].kind, LineKind::Item));
    match &sale.lines[2].kind {
        LineKind::Crate { components } => assert_eq!(components.len(), 2),
        LineKind::Item => panic!("the crate line"),
    }
    assert_eq!(sale.lines[2].amount_minor, 60_000);
    assert_eq!(sale.total_minor, receipt.total.minor_units());
}

#[test]
fn a_mixed_bill_is_edited_cancelled_and_restored_in_place() {
    let (mut shop, template) = shop();
    let lime = shop.product_stock(LIME).unwrap();
    let draft = mixed_draft(&shop, template);
    let receipt = post(&mut shop, &draft, "200");
    let (review, _) = shop.current_sale(Role::Owner, BILL).unwrap();
    let rates = review.prices.clone();
    let edit = |quantity: u32, price: Money| LineEdit {
        quantity: n(quantity),
        price,
    };
    // One entry per ordinary line, then one per crate row: 3 Lime, 1 Soda, 1 crate at ₹310.
    let after = shop
        .edit_posted_lines(
            Role::Owner,
            BILL,
            &[edit(3, rates[0]), edit(1, rates[1]), edit(1, rs(31_000))],
        )
        .unwrap();
    assert_eq!(after.quantity, 3 + 1 + 2);
    assert_eq!(shop.product_stock(LIME), Some(lime - 5));
    let expected = 3 * rates[0].minor_units() + rates[1].minor_units() + 31_000 + 150 * 6;
    assert_eq!(after.total, rs(expected));
    assert_eq!(after.invoice, receipt.invoice);
    let (review, _) = shop.current_sale(Role::Owner, BILL).unwrap();
    assert_eq!(review.draft().crate_rows()[0].rate(), Some(rs(31_000)));
    assert_eq!(review.draft().crate_rows()[0].name(), Some("Lime Twins"));
    // A count change and a total below the amount paid are refused before any effect.
    assert_eq!(
        shop.edit_posted_lines(Role::Owner, BILL, &[edit(3, rates[0])]),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        shop.edit_posted_lines(
            Role::Owner,
            BILL,
            &[edit(1, rs(0)), edit(1, rs(0)), edit(1, rs(0))]
        ),
        Err(ConfirmationError::InvalidAmount)
    );
    assert_eq!(
        shop.edit_crate_bill(Role::Owner, BILL, edit(1, rs(1))),
        Err(ConfirmationError::InvalidDraft),
        "not a bill of one crate"
    );
    assert_eq!(shop.product_stock(LIME), Some(lime - 5));

    shop.cancel_sale(Role::Owner, BILL).unwrap();
    assert_eq!(shop.product_stock(LIME), Some(lime));
    shop.undo_cancel_sale(Role::Owner, BILL).unwrap();
    assert_eq!(shop.product_stock(LIME), Some(lime - 5));
    assert_eq!(
        shop.edit_posted_lines(Role::Manager, BILL, &[]),
        Err(ConfirmationError::AccessDenied)
    );
}

#[test]
fn a_mixed_review_survives_the_pending_snapshot_round_trip() {
    let (mut shop, template) = shop();
    let draft = mixed_draft(&shop, template);
    shop.register_draft(draft).unwrap();
    let review = shop.review_named(Role::Owner, BILL, rs(20_000)).unwrap();
    assert!(review.is_named_crate_sale());
    let text = pending::encode(&review);
    assert!(text.contains("10|"), "{text}");
    assert_eq!(pending::decode(&text), Ok(review.clone()));
    let receipt = shop
        .confirm(Role::Owner, RequestId::new(BILL.value()), &review)
        .unwrap();
    assert_eq!(receipt.quantity, 7);
}

/// Several crate rows and no item line (HTML `saveBill` accepts a bill whose lines are all
/// crates): priced, reviewed and posted like a mixed bill, each component from FIFO.
#[test]
fn a_bill_of_several_crate_rows_and_no_item_line_posts() {
    let (mut shop, template) = shop();
    let lime = shop.product_stock(LIME).unwrap();
    let mut draft = ProvisionalDraft::new(BILL, Vec::new())
        .with_customer(DraftCustomer::Selected {
            id: customer_id(),
            name: super::super::demo_fixture::CUSTOMER.into(),
        })
        .with_bill_date(Some(day()))
        .with_loading_rate(rs(100));
    shop.load_crate_template(Role::Owner, template, n(1), &mut draft)
        .unwrap();
    shop.load_crate_template(Role::Owner, template, n(2), &mut draft)
        .unwrap();
    let priced = priced_draft(&shop, Role::Owner, &draft).unwrap();
    assert!(priced.lines.is_empty());
    let amounts: Vec<_> = priced.crates.iter().map(|c| c.amount).collect();
    assert_eq!(amounts, [rs(30_000), rs(60_000)]);
    // 3 crates x (1 packet + 2 pieces at 2 a packet) = 6 packets.
    assert_eq!((priced.loading, priced.total), (rs(600), rs(90_600)));
    let receipt = post(&mut shop, &draft, "0");
    assert_eq!((receipt.quantity, receipt.total), (6, rs(90_600)));
    assert_eq!(shop.product_stock(LIME), Some(lime - 6));
    let empty = ProvisionalDraft::new(DraftId::new(42), Vec::new())
        .with_customer(draft.customer().clone())
        .with_bill_date(Some(day()));
    assert_eq!(
        priced_draft(&shop, Role::Owner, &empty),
        Err(ConfirmationError::InvalidDraft)
    );
}
