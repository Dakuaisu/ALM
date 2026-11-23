use super::*;
use crate::DraftCustomerId;

mod dated_register;
use dated_register::{RegisterRow, register};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const SALE: DraftId = DraftId::new(1);
const ITEM: &str = "Synthetic Orange Soda";
const KEY: &str = "synthetic orange soda";
const SELL: Money = Money::from_minor_units(500);

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn qty(quantity: u32) -> NonZeroU32 {
    NonZeroU32::new(quantity).unwrap()
}

fn billing_fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 0, SELL).unwrap();
    let mut product = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    product.item = ITEM.into();
    fixture.save_product_entry(OWNER, &product).unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: Money::from_minor_units(90),
        })
        .unwrap();
    let order = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Supplier".into(),
                }),
                date: Some(date(9)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: ITEM.into(),
                    product_id: Some(PRODUCT),
                    ordered: qty(9),
                    rate: Money::from_minor_units(75),
                }],
            },
        )
        .unwrap();
    for (day, quantity, rate) in [(10, 5, 80), (11, 4, 120)] {
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: OWNER,
                id: order,
                date: Some(date(day)),
                today: date(12),
                lines: &[ReceiveLine {
                    quantity,
                    rate: Money::from_minor_units(rate),
                }],
            }),
            Ok(ReceiveResult::Received)
        );
    }
    let draft = ProvisionalDraft::new(SALE, vec![DraftLine::new(PRODUCT, qty(2))])
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("l12-01-buyer".into()),
            name: "Synthetic Buyer".into(),
        })
        .with_bill_date(Some(date(11)));
    fixture.register_draft(draft).unwrap();
    let review = fixture.review_named(OWNER, SALE, SELL).unwrap();
    fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();
    fixture
        .submit_physical_count(
            OWNER,
            PhysicalCountInput {
                product: PRODUCT,
                counted: "7",
                fridge: Some("11"),
                date: date(12),
                reason: None,
                note: None,
            },
        )
        .unwrap();
    fixture
}

#[derive(Debug, Eq, PartialEq)]
struct BillingSnapshot {
    on_hand: u32,
    fridge: u32,
    register: [RegisterRow; 3],
    fifo: StockLots,
    sale_cost: Option<Option<i64>>,
}

fn billing(fixture: &ConfirmationFixture) -> BillingSnapshot {
    let count = fixture.open_physical_count(OWNER, PRODUCT).unwrap();
    BillingSnapshot {
        on_hand: count.on_hand,
        fridge: count.fridge,
        register: register(fixture),
        fifo: fixture.open_stock_lots(OWNER, PRODUCT).unwrap(),
        sale_cost: fixture.catalog.allocated_cost(SALE),
    }
}

fn form<'a>(item: &'a str, cases: &'a str, note: &'a str) -> ManualGodownInput<'a> {
    ManualGodownInput { item, cases, note }
}

fn row(item: &str, cases: u32, note: Option<&str>) -> ManualGodownRow {
    ManualGodownRow {
        item: item.into(),
        cases: GodownCases::new(cases),
        note: note.map(str::to_owned),
    }
}

#[test]
fn owner_updates_manual_count_without_changing_billing_or_register() {
    // Given a named billing product, two dated receipts, a posted sale, and a fridge count.
    let mut fixture = billing_fixture();
    let mut expected = fixture.clone();
    assert!(fixture.open_manual_godown(OWNER).unwrap().is_empty());
    let input = form("  Synthetic Orange Soda ", " 3 ", " back rack ");
    fixture.save_manual_godown(OWNER, input).unwrap();
    let prior = row(ITEM, 3, Some("back rack"));
    expected.manual_godown.insert(KEY.into(), prior.clone());
    assert_eq!(fixture, expected);
    let manual_before = fixture.open_manual_godown(OWNER).unwrap();
    assert_eq!(manual_before, [&prior]);
    let before = billing(&fixture);
    println!("L12-01 before: manual={manual_before:?}, billing={before:?}");
    assert_eq!(
        before.register.each_ref().map(|row| (
            row.date,
            row.opening,
            row.received,
            row.dispatched,
            row.closing
        )),
        [
            (date(10), 0, 5, 0, 5),
            (date(11), 5, 4, 2, 7),
            (date(12), 7, 0, 0, 7),
        ]
    );
    assert_eq!((before.on_hand, before.fridge), (7, 11));
    assert_eq!(before.sale_cost, Some(Some(160)));
    assert_eq!(before.fifo.on_hand, 7);
    assert_eq!(before.fifo.total_value.map(Money::minor_units), Some(720));
    let fifo_rows: Vec<_> = before
        .fifo
        .rows
        .iter()
        .map(|row| {
            (
                row.date,
                row.remaining.get(),
                row.unit_cost.map(Money::minor_units),
                row.value.map(Money::minor_units),
            )
        })
        .collect();
    assert_eq!(
        fifo_rows,
        [
            (Some(date(10)), 3, Some(80), Some(240)),
            (Some(date(11)), 4, Some(120), Some(480)),
        ]
    );

    // When the Owner edits by a differently cased name with surrounding whitespace.
    let input = form("\tSYNTHETIC orange soda  ", "8", " \t ");
    fixture.save_manual_godown(OWNER, input).unwrap();

    // Then only the original manual row changes, and reopening returns the same billing state.
    let changed = row(ITEM, 8, Some("back rack"));
    expected.manual_godown.insert(KEY.into(), changed.clone());
    let manual_after = fixture.open_manual_godown(OWNER).unwrap();
    let after = billing(&fixture);
    println!("L12-01 after: manual={manual_after:?}, billing={after:?}");
    assert_eq!(manual_after, [&changed]);
    assert_eq!(after, before);
    assert_eq!(fixture, expected);
}

#[test]
fn manager_is_denied_before_validation_without_any_fixture_change() {
    let mut fixture = billing_fixture();
    fixture
        .save_manual_godown(OWNER, form(ITEM, "3", "back rack"))
        .unwrap();
    let before = fixture.clone();
    let manager = Role::Manager;
    let denied = ManualGodownError::AccessDenied;
    assert_eq!(fixture.open_manual_godown(manager), Err(denied));
    assert_eq!(fixture, before);
    for input in [
        form(ITEM, "8", "changed"),
        form(" ", "bad", ""),
        form(ITEM, "bad", ""),
    ] {
        assert_eq!(fixture.save_manual_godown(manager, input), Err(denied));
        assert_eq!(fixture, before);
    }
    println!("L12-01 Manager: reopen and valid/invalid saves denied; entire fixture unchanged");
}

#[test]
fn owner_saves_manual_only_names_and_rejects_invalid_input_atomically() {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 0, SELL).unwrap();
    let mut expected = fixture.clone();
    use ManualGodownError::{InvalidCases, ItemNameRequired};
    for (item, cases, error) in [
        (" ", "3", ItemNameRequired),
        (ITEM, " ", InvalidCases),
        (ITEM, "-1", InvalidCases),
        (ITEM, "1.5", InvalidCases),
        (ITEM, "NaN", InvalidCases),
        (ITEM, "4294967296", InvalidCases),
    ] {
        let input = form(item, cases, "");
        assert_eq!(fixture.save_manual_godown(OWNER, input), Err(error));
        assert_eq!(fixture, expected);
    }

    for (input, saved) in [
        (form(" Loose boxes ", "0", ""), row("Loose boxes", 0, None)),
        (
            form("LOOSE BOXES", "2", " top rack "),
            row("Loose boxes", 2, Some("top rack")),
        ),
    ] {
        fixture.save_manual_godown(OWNER, input).unwrap();
        expected.manual_godown.insert("loose boxes".into(), saved);
        assert_eq!(fixture, expected);
    }
}
