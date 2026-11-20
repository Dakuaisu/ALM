use std::num::NonZeroU32;

use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, EditChanges, PostedLineItem, ProductEntryError,
    PurchaseDraft, PurchaseLine, PurchaseProduct, PurchaseReceipt, PurchaseSupplier, Receipt,
    ReceiveLine, ReceiveRequest, ReceiveResult, RequestId, Review,
};
use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(9);
const SALE: DraftId = DraftId::new(21);
const REQUEST: RequestId = RequestId::new(51);
const OLD: &str = "Synthetic Peach Soda";
const NEW: &str = "Synthetic Peach Fizz";
const SELL: Money = Money::from_minor_units(500);
const BUY: Money = Money::from_minor_units(300);
const RECEIVED: Money = Money::from_minor_units(200);

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn named_line(product_id: ProductId, item: &str) -> PostedLineItem {
    PostedLineItem {
        product_id,
        item: Some(item.into()),
    }
}

fn name_product(fixture: &mut ConfirmationFixture, product: ProductId, item: &str) {
    let mut form = fixture.open_product_entry(OWNER, product).unwrap();
    form.item = item.into();
    fixture.save_product_entry(OWNER, &form).unwrap();
}

fn posted_fixture() -> (ConfirmationFixture, Review, Receipt) {
    let mut fixture = ConfirmationFixture::with_ordered_lots(PRODUCT, &[(1, BUY)], SELL).unwrap();
    name_product(&mut fixture, PRODUCT, OLD);
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: "Supplier's Peach Soda".into(),
            buy_rate: BUY,
        })
        .unwrap();
    let order = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Orchard Supply".into(),
                }),
                date: Some(date(1)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: OLD.into(),
                    product_id: Some(PRODUCT),
                    ordered: qty(5),
                    rate: Money::from_minor_units(275),
                }],
            },
        )
        .unwrap();
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: OWNER,
            id: order,
            date: Some(date(2)),
            today: date(3),
            lines: &[ReceiveLine {
                quantity: 4,
                rate: BUY,
            }],
        }),
        Ok(ReceiveResult::Received)
    );

    let draft = ProvisionalDraft::new(SALE, vec![DraftLine::new(PRODUCT, qty(2))])
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("l09-04-buyer".into()),
            name: "Synthetic Buyer".into(),
        })
        .with_bill_date(Some(date(4)));
    fixture.register_draft(draft).unwrap();
    let review = fixture.review_named(OWNER, SALE, RECEIVED).unwrap();
    let receipt = fixture.confirm(OWNER, REQUEST, &review).unwrap();
    (fixture, review, receipt)
}

#[test]
fn owner_rename_changes_exact_lookup_but_preserves_sale_purchase_and_lots() {
    let (mut fixture, review, receipt) = posted_fixture();
    let expected_line = named_line(PRODUCT, OLD);
    assert_eq!(
        fixture.find_product_by_exact_name(OWNER, OLD),
        Ok(Some(PRODUCT))
    );
    assert_eq!(fixture.find_product_by_exact_name(OWNER, NEW), Ok(None));
    assert_eq!(
        fixture.posted_sale_items(OWNER, SALE).unwrap(),
        std::slice::from_ref(&expected_line)
    );
    let orders = fixture.purchase_orders(OWNER).unwrap().to_vec();
    let stock = fixture.product_stock(PRODUCT);
    let lots = fixture.catalog.lot_snapshot(PRODUCT);
    let dated_lots = fixture.catalog.receipt_lots(PRODUCT);
    let allocations = fixture.catalog.allocation_snapshot();

    name_product(&mut fixture, PRODUCT, NEW);

    assert_eq!(
        fixture.find_product_by_exact_name(OWNER, NEW),
        Ok(Some(PRODUCT))
    );
    assert_eq!(fixture.find_product_by_exact_name(OWNER, OLD), Ok(None));
    assert_eq!(
        fixture.find_product_by_exact_name(OWNER, "synthetic peach fizz"),
        Ok(None)
    );
    assert_eq!(
        fixture.posted_sale_items(OWNER, SALE).unwrap(),
        [expected_line]
    );
    assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &receipt)));
    assert_eq!(receipt.invoice, 1);
    assert_eq!(receipt.quantity, 2);
    assert_eq!(receipt.total, Money::from_minor_units(1_000));
    assert_eq!(receipt.due, Some(Money::from_minor_units(800)));

    let current_orders = fixture.purchase_orders(OWNER).unwrap();
    assert_eq!(current_orders, orders);
    let po_line = &current_orders[0].items[0];
    assert_eq!(po_line.line.item, OLD);
    assert_eq!(po_line.line.product_id, Some(PRODUCT));
    assert_eq!(
        po_line.receipts(),
        [PurchaseReceipt {
            date: date(2),
            quantity: qty(4),
            rate: BUY,
        }]
    );
    assert_eq!(stock, Some(3));
    assert_eq!(fixture.product_stock(PRODUCT), stock);
    assert_eq!(lots, [0, 3]);
    assert_eq!(dated_lots, [(date(2), 3, BUY.minor_units())]);
    assert_eq!(fixture.catalog.lot_snapshot(PRODUCT), lots);
    assert_eq!(fixture.catalog.receipt_lots(PRODUCT), dated_lots);
    assert_eq!(fixture.catalog.allocation_snapshot(), allocations);

    let before_replay = fixture.clone();
    assert_eq!(fixture.confirm(OWNER, REQUEST, &review), Ok(receipt));
    assert_eq!(fixture, before_replay);
}

#[test]
fn manager_rename_denial_leaves_the_entire_operational_fixture_unchanged() {
    let (mut fixture, _, _) = posted_fixture();
    let mut form = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    form.item = NEW.into();
    let before = fixture.clone();

    assert_eq!(
        fixture.save_product_entry(Role::Manager, &form),
        Err(ProductEntryError::AccessDenied)
    );

    assert_eq!(fixture, before);
    assert_eq!(
        fixture.find_product_by_exact_name(Role::Manager, OLD),
        Err(ProductEntryError::AccessDenied)
    );
    assert_eq!(
        fixture.posted_sale_items(Role::Manager, SALE),
        Err(ConfirmationError::AccessDenied)
    );
}

#[test]
fn sale_edit_retains_the_posted_name_and_freezes_the_appended_name_on_save() {
    let (mut fixture, review, receipt) = posted_fixture();
    let appended = ProductId::new(10);
    fixture.add_product(appended, 1, SELL).unwrap();
    name_product(&mut fixture, PRODUCT, NEW);
    let edit = fixture
        .begin_edit(
            OWNER,
            SALE,
            EditChanges {
                retained_price: SELL,
                appended_line: DraftLine::new(appended, qty(1)),
                appended_price: SELL,
            },
        )
        .unwrap();
    name_product(&mut fixture, appended, "Synthetic Berry Soda");

    let edited = fixture.save_edit(OWNER, &edit).unwrap();
    name_product(&mut fixture, appended, "Synthetic Berry Fizz");

    assert_eq!(
        fixture.posted_sale_items(OWNER, SALE).unwrap(),
        [
            named_line(PRODUCT, OLD),
            named_line(appended, "Synthetic Berry Soda")
        ]
    );
    assert_eq!(edited.invoice, receipt.invoice);
    assert_eq!(edited.quantity, 3);
    assert_eq!(edited.total, Money::from_minor_units(1_500));
    let before_replay = fixture.clone();
    assert_eq!(fixture.confirm(OWNER, REQUEST, &review), Ok(receipt));
    assert_eq!(fixture, before_replay);
}

#[test]
fn new_sale_uses_the_renamed_item_and_keeps_anonymous_lines_unnamed() {
    let (mut fixture, _, _) = posted_fixture();
    let anonymous = ProductId::new(10);
    let sale = DraftId::new(22);
    fixture.add_product(anonymous, 1, SELL).unwrap();
    name_product(&mut fixture, PRODUCT, NEW);
    let draft = ProvisionalDraft::new(
        sale,
        vec![
            DraftLine::new(anonymous, qty(1)),
            DraftLine::new(PRODUCT, qty(1)),
        ],
    )
    .with_customer(DraftCustomer::Typed("Synthetic Buyer".into()))
    .with_bill_date(Some(date(5)));
    fixture.register_draft(draft).unwrap();
    let review = fixture.review(OWNER, sale).unwrap();
    fixture.confirm(OWNER, RequestId::new(52), &review).unwrap();
    name_product(&mut fixture, anonymous, "Synthetic Berry Soda");

    assert_eq!(
        fixture.posted_sale_items(OWNER, sale).unwrap(),
        [
            PostedLineItem {
                product_id: anonymous,
                item: None
            },
            named_line(PRODUCT, NEW)
        ]
    );
}
