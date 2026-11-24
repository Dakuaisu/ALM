//! The authority conformance suite: one generic check per `SalesAuthority` command, run against
//! every implementation. Each command must succeed once, return the original outcome to an
//! identical retry, refuse reuse of its request ID with another intent, refuse stale reviewed
//! terms where it has any, deny a Manager, and leave no effect (and no request record) on any
//! rejection. Everything the suite needs it reads through `pull`, so it is seed-agnostic.
#![allow(dead_code)]

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::confirmation::{GstBillDraft, GstBillId, GstBillLineInput, GstBillNumber, GstUnit};
use alm_core::projection::ProjectionSnapshot;
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};
use alm_sync::{
    AuthorityError, CrateTerms, Intent, Outcome, ProductTerms, PurchaseLineTerms,
    PurchaseOrderTerms, RequestId, SaleIntent, SalesAuthority, gst_bill_key,
};

pub fn day() -> BillDate {
    BillDate::new(2025, 4, 30).unwrap()
}

fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

fn req(case: &str, tag: &str) -> RequestId {
    RequestId::new(&format!("conf-{case}-{tag}")).unwrap()
}

/// What the suite works with, read from the Owner projection.
pub struct World {
    pub product: u64,
    pub item: String,
    pub brand: String,
    pub sell: Money,
    pub buy: Money,
    pub on_hand: u32,
    pub customer: (String, String),
    pub supplier: u64,
}

pub fn state<A: SalesAuthority>(a: &A) -> (u64, ProjectionSnapshot) {
    let batch = a.pull(Role::Owner, None).expect("owner pull");
    (batch.checkpoint, batch.snapshot.expect("a full snapshot"))
}

pub fn world<A: SalesAuthority>(a: &A) -> World {
    let (_, s) = state(a);
    let stock = s
        .stock
        .iter()
        .max_by_key(|row| (row.on_hand, std::cmp::Reverse(row.product)))
        .expect("a product with stock");
    assert!(stock.on_hand >= 10, "the seed needs stock");
    let terms = s.products.iter().find(|p| p.id == stock.product).unwrap();
    let buy = s
        .owner
        .as_ref()
        .and_then(|o| o.buy_rates.iter().find(|(id, _)| *id == stock.product))
        .map_or(money(0), |(_, buy)| *buy);
    let customer = s.customers.first().expect("a customer");
    World {
        product: stock.product,
        item: terms.item.clone(),
        brand: terms.brand.clone(),
        sell: terms.sell,
        buy,
        on_hand: stock.on_hand,
        customer: (customer.id.clone(), customer.name.clone()),
        supplier: s.suppliers.first().expect("a supplier").id,
    }
}

pub fn sale(w: &World, draft: u64, quantity: u32, price: Money) -> Intent {
    Intent::PostSale(SaleIntent {
        draft: ProvisionalDraft::new(
            DraftId::new(draft),
            vec![DraftLine::new(
                ProductId::new(w.product),
                std::num::NonZeroU32::new(quantity).unwrap(),
            )],
        )
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new(w.customer.0.clone()),
            name: w.customer.1.clone(),
        })
        .with_bill_date(Some(day())),
        date: day(),
        // The development core cancels only bills with a part payment.
        paid: money(2_000),
        prices: vec![price],
    })
}

fn setup<A: SalesAuthority>(a: &mut A, case: &str, tag: &str, intent: &Intent) -> Outcome {
    a.submit(Role::Owner, &req(case, tag), intent)
        .unwrap_or_else(|e| panic!("{case} setup {tag}: {e:?}"))
}

fn created(outcome: Outcome) -> u64 {
    match outcome {
        Outcome::Created(id) => id,
        other => panic!("expected a created ID, got {other:?}"),
    }
}

/// The common contract of one command. `effect` must be false before and true after.
#[allow(clippy::too_many_arguments)]
fn exercise<A: SalesAuthority>(
    a: &mut A,
    case: &str,
    good: &Intent,
    other: &Intent,
    stale: Option<&Intent>,
    invalid: Option<&Intent>,
    effect: impl Fn(&ProjectionSnapshot) -> bool,
) -> Outcome {
    let before = state(a);
    assert!(!effect(&before.1), "{case}: effect already present");

    assert_eq!(
        a.submit(Role::Manager, &req(case, "manager"), good),
        Err(AuthorityError::Denied),
        "{case}: Manager denied"
    );
    assert_eq!(state(a), before, "{case}: Manager denial left an effect");

    if let Some(stale) = stale {
        assert_eq!(
            a.submit(Role::Owner, &req(case, "stale"), stale),
            Err(AuthorityError::StaleReview),
            "{case}: stale terms"
        );
        assert_eq!(state(a), before, "{case}: stale rejection left an effect");
        assert_eq!(a.status(Role::Owner, &req(case, "stale")), Ok(None));
    }
    if let Some(invalid) = invalid {
        let error = a
            .submit(Role::Owner, &req(case, "invalid"), invalid)
            .expect_err("invalid intent rejected");
        assert!(
            error.is_definite() && error != AuthorityError::Denied,
            "{case}: definite rejection, got {error:?}"
        );
        assert_eq!(state(a), before, "{case}: rejection left a partial effect");
        assert_eq!(a.status(Role::Owner, &req(case, "invalid")), Ok(None));
    }

    let first = a
        .submit(Role::Owner, &req(case, "1"), good)
        .unwrap_or_else(|e| panic!("{case}: success, got {e:?}"));
    let after = state(a);
    assert!(after.0 > before.0, "{case}: checkpoint advances");
    assert!(effect(&after.1), "{case}: effect visible");

    assert_eq!(
        a.submit(Role::Owner, &req(case, "1"), good),
        Ok(first),
        "{case}: identical retry returns the original outcome"
    );
    assert_eq!(state(a), after, "{case}: retry had a second effect");
    assert_eq!(a.status(Role::Owner, &req(case, "1")), Ok(Some(first)));
    assert_eq!(
        a.status(Role::Manager, &req(case, "1")),
        Err(AuthorityError::Denied)
    );

    assert_eq!(
        a.submit(Role::Owner, &req(case, "1"), other),
        Err(AuthorityError::Conflict),
        "{case}: conflicting reuse"
    );
    assert_eq!(state(a), after, "{case}: conflict left an effect");
    first
}

fn sale_of(s: &ProjectionSnapshot, draft: u64) -> Option<&alm_core::projection::ProjectedSale> {
    s.sales.iter().find(|x| x.draft_id == draft)
}

fn stock_of(s: &ProjectionSnapshot, product: u64) -> Option<u32> {
    s.stock
        .iter()
        .find(|x| x.product == product)
        .map(|x| x.on_hand)
}

pub fn post_sale<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let (product, on_hand) = (w.product, w.on_hand);
    let outcome = exercise(
        a,
        "post",
        &sale(&w, 101, 2, w.sell),
        &sale(&w, 101, 3, w.sell),
        Some(&sale(&w, 102, 1, money(w.sell.minor_units() + 1))),
        Some(&sale(&w, 103, on_hand + 1_000, w.sell)),
        |s| sale_of(s, 101).is_some() && stock_of(s, product) == Some(on_hand - 2),
    );
    let Outcome::Sale(receipt) = outcome else {
        panic!("a sale receipt")
    };
    assert_eq!(
        (receipt.draft, receipt.quantity, receipt.total),
        (101, 2, money(w.sell.minor_units() * 2))
    );
    assert_eq!(
        a.submit(
            Role::Owner,
            &req("post", "again"),
            &sale(&w, 101, 2, w.sell)
        ),
        Err(AuthorityError::AlreadyPosted)
    );
}

pub fn cancel_sale<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    setup(a, "cancel", "setup", &sale(&w, 111, 2, w.sell));
    exercise(
        a,
        "cancel",
        &Intent::CancelSale { draft: 111 },
        &Intent::RestoreSale { draft: 111 },
        None,
        Some(&Intent::CancelSale { draft: 999 }),
        |s| sale_of(s, 111).is_some_and(|x| x.cancelled),
    );
}

pub fn restore_sale<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    setup(a, "restore", "setup", &sale(&w, 111, 2, w.sell));
    setup(a, "restore", "cancel", &Intent::CancelSale { draft: 111 });
    exercise(
        a,
        "restore",
        &Intent::RestoreSale { draft: 111 },
        &Intent::CancelSale { draft: 111 },
        None,
        Some(&Intent::RestoreSale { draft: 998 }),
        |s| sale_of(s, 111).is_some_and(|x| !x.cancelled),
    );
}

pub fn edit_sale<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    setup(a, "edit", "setup", &sale(&w, 121, 2, w.sell));
    let rate = w.sell.minor_units();
    let outcome = exercise(
        a,
        "edit",
        &Intent::EditSale {
            draft: 121,
            lines: vec![(3, rate)],
        },
        &Intent::EditSale {
            draft: 121,
            lines: vec![(4, rate)],
        },
        None,
        Some(&Intent::EditSale {
            draft: 121,
            lines: vec![(3, rate), (1, rate)],
        }),
        |s| sale_of(s, 121).is_some_and(|x| x.total == money(rate * 3)),
    );
    assert!(matches!(outcome, Outcome::Sale(r) if r.quantity == 3));
}

pub fn record_receipt<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let n = state(a).1.receipts.len();
    let receipt = |customer: &str, amount: i64| Intent::RecordReceipt {
        customer: customer.into(),
        date: day(),
        amount: money(amount),
        online: true,
    };
    exercise(
        a,
        "receipt",
        &receipt(&w.customer.0, 1_000),
        &receipt(&w.customer.0, 2_000),
        None,
        Some(&receipt("conformance-nobody", 1_000)),
        |s| s.receipts.len() == n + 1,
    );
}

fn cash(amount: i64) -> CashBookAmounts {
    CashBookAmounts {
        cash: money(amount),
        ..CashBookAmounts::ZERO
    }
}

pub fn save_cash_day<A: SalesAuthority>(a: &mut A) {
    let n = state(a).1.cash_days.len();
    exercise(
        a,
        "cashday",
        &Intent::SaveCashDay {
            id: None,
            date: day(),
            amounts: cash(2_500),
        },
        &Intent::SaveCashDay {
            id: None,
            date: day(),
            amounts: cash(2_600),
        },
        None,
        Some(&Intent::SaveCashDay {
            id: Some(999_999),
            date: day(),
            amounts: cash(2_500),
        }),
        |s| s.cash_days.len() == n + 1,
    );
}

pub fn delete_cash_day<A: SalesAuthority>(a: &mut A) {
    setup(
        a,
        "cashdel",
        "setup",
        &Intent::SaveCashDay {
            id: None,
            date: day(),
            amounts: cash(2_500),
        },
    );
    let (_, s) = state(a);
    let id = s.cash_days.last().unwrap().id;
    let n = s.cash_days.len();
    exercise(
        a,
        "cashdel",
        &Intent::DeleteCashDay { id },
        &Intent::DeleteCashDay { id: id + 1 },
        None,
        Some(&Intent::DeleteCashDay { id: 999_999 }),
        |s| s.cash_days.len() == n - 1,
    );
}

fn product(id: u64, new: bool, item: &str, sell: i64) -> ProductTerms {
    ProductTerms {
        id,
        new,
        item: item.into(),
        brand: "Conformance Co".into(),
        buy: money(3_000),
        sell: money(sell),
        reorder: Some(6),
        reviewed: None,
    }
}

pub fn add_product<A: SalesAuthority>(a: &mut A) {
    exercise(
        a,
        "product-new",
        &Intent::SaveProduct(product(777, true, "Conformance Orange", 4_500)),
        &Intent::SaveProduct(product(777, true, "Conformance Orange", 4_600)),
        None,
        Some(&Intent::SaveProduct(product(777, true, "  ", 4_500))),
        |s| {
            s.products
                .iter()
                .any(|p| p.id == 777 && p.sell == money(4_500) && p.reorder_level == 6)
        },
    );
}

pub fn edit_product<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let edit = |sell: i64, reviewed: (Money, Money)| {
        Intent::SaveProduct(ProductTerms {
            id: w.product,
            new: false,
            item: w.item.clone(),
            brand: w.brand.clone(),
            buy: w.buy,
            sell: money(sell),
            reorder: None,
            reviewed: Some(reviewed),
        })
    };
    let current = (w.buy, w.sell);
    let target = w.sell.minor_units() + 100;
    let mut blank = product(w.product, false, "", target);
    blank.reviewed = Some(current);
    exercise(
        a,
        "product-edit",
        &edit(target, current),
        &edit(target + 100, current),
        Some(&edit(target, (w.buy, money(w.sell.minor_units() + 1)))),
        Some(&Intent::SaveProduct(blank)),
        |s| {
            s.products
                .iter()
                .any(|p| p.id == w.product && p.sell == money(target))
        },
    );
}

pub fn delete_product<A: SalesAuthority>(a: &mut A) {
    setup(
        a,
        "product-del",
        "setup",
        &Intent::SaveProduct(product(778, true, "Conformance Grape", 3_000)),
    );
    exercise(
        a,
        "product-del",
        &Intent::DeleteProduct { id: 778 },
        &Intent::DeleteProduct { id: 779 },
        None,
        Some(&Intent::DeleteProduct { id: 99_999 }),
        |s| !s.products.iter().any(|p| p.id == 778),
    );
    // A product on a posted bill is refused, with no effect.
    let w = world(a);
    setup(a, "product-del", "sale", &sale(&w, 131, 1, w.sell));
    let before = state(a);
    assert!(
        a.submit(
            Role::Owner,
            &req("product-del", "sold"),
            &Intent::DeleteProduct { id: w.product }
        )
        .is_err()
    );
    assert_eq!(state(a), before);
}

pub fn add_supplier<A: SalesAuthority>(a: &mut A) {
    let outcome = exercise(
        a,
        "supplier",
        &Intent::AddSupplier {
            name: "Conformance Traders".into(),
        },
        &Intent::AddSupplier {
            name: "Other Traders".into(),
        },
        None,
        Some(&Intent::AddSupplier { name: " ".into() }),
        |s| s.suppliers.iter().any(|x| x.name == "Conformance Traders"),
    );
    let id = created(outcome);
    assert!(state(a).1.suppliers.iter().any(|x| x.id == id));
}

pub fn set_supplier_prices<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let (product, supplier) = (w.product, w.supplier);
    let quotes = |rates: Vec<(u64, Option<Money>)>| Intent::SetSupplierPrices { product, rates };
    exercise(
        a,
        "quotes",
        &quotes(vec![(supplier, Some(money(2_951)))]),
        &quotes(vec![(supplier, Some(money(2_961)))]),
        None,
        // Atomic: the valid first quote must not survive the unknown second supplier.
        Some(&quotes(vec![
            (supplier, Some(money(2_951))),
            (99_999, Some(money(1))),
        ])),
        |s| {
            s.owner.as_ref().is_some_and(|o| {
                o.supplier_prices
                    .contains(&(product, supplier, money(2_951)))
            })
        },
    );
}

fn purchase(w: &World, supplier: Option<&str>, ordered: u32) -> Intent {
    Intent::SavePurchaseOrder(PurchaseOrderTerms {
        supplier: supplier.map(|name| (None, name.into())),
        date: Some(day()),
        note: String::new(),
        lines: vec![PurchaseLineTerms {
            item: w.item.clone(),
            product: Some(w.product),
            ordered,
            rate: money(3_600),
        }],
    })
}

pub fn save_purchase_order<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let n = state(a).1.owner.unwrap().purchase_orders.len();
    let outcome = exercise(
        a,
        "po",
        &purchase(&w, Some("Conformance Depot"), 10),
        &purchase(&w, Some("Conformance Depot"), 11),
        None,
        Some(&purchase(&w, None, 10)),
        |s| {
            s.owner
                .as_ref()
                .is_some_and(|o| o.purchase_orders.len() == n + 1)
        },
    );
    let id = created(outcome);
    let (_, s) = state(a);
    let order = s
        .owner
        .unwrap()
        .purchase_orders
        .into_iter()
        .find(|o| o.id == id)
        .unwrap();
    assert_eq!((order.lines[0].ordered, order.lines[0].remaining), (10, 10));
}

fn receive(po: u64, quantity: u32, remaining: u32) -> Intent {
    Intent::ReceivePurchaseOrder {
        po,
        date: Some(day()),
        today: day(),
        lines: vec![(quantity, money(3_500))],
        remaining: vec![remaining],
    }
}

fn order_line(s: &ProjectionSnapshot, po: u64) -> Option<(u32, u32)> {
    let owner = s.owner.as_ref()?;
    let order = owner.purchase_orders.iter().find(|o| o.id == po)?;
    order.lines.first().map(|l| (l.received, l.remaining))
}

pub fn receive_purchase_order<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let po = created(setup(
        a,
        "receive",
        "setup",
        &purchase(&w, Some("Conformance Depot"), 10),
    ));
    let (product, on_hand) = (w.product, w.on_hand);
    exercise(
        a,
        "receive",
        &receive(po, 4, 10),
        &receive(po, 5, 10),
        Some(&receive(po, 4, 9)),
        Some(&receive(po, 0, 10)),
        |s| order_line(s, po) == Some((4, 6)) && stock_of(s, product) == Some(on_hand + 4),
    );
}

pub fn close_purchase_order<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let po = created(setup(
        a,
        "close",
        "setup",
        &purchase(&w, Some("Conformance Depot"), 10),
    ));
    setup(a, "close", "receive", &receive(po, 4, 10));
    exercise(
        a,
        "close",
        &Intent::ClosePurchaseOrder {
            po,
            remaining: vec![6],
        },
        &Intent::ClosePurchaseOrder {
            po,
            remaining: vec![5],
        },
        Some(&Intent::ClosePurchaseOrder {
            po,
            remaining: vec![10],
        }),
        Some(&Intent::ClosePurchaseOrder {
            po: 999,
            remaining: vec![6],
        }),
        |s| order_line(s, po) == Some((4, 0)),
    );
}

fn crate_terms(w: &World, id: Option<u64>, price: i64, components: bool) -> Intent {
    Intent::SaveCrate(CrateTerms {
        id,
        name: "Conformance Crate".into(),
        price: money(price),
        components: if components {
            vec![(w.product, 2, false)]
        } else {
            Vec::new()
        },
    })
}

pub fn save_crate<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let id = created(exercise(
        a,
        "crate",
        &crate_terms(&w, None, 40_000, true),
        &crate_terms(&w, None, 41_000, true),
        None,
        Some(&crate_terms(&w, None, 40_000, false)),
        |s| s.crates.iter().any(|c| c.name == "Conformance Crate"),
    ));
    let (_, s) = state(a);
    assert!(
        s.crates
            .iter()
            .any(|c| c.id == id && c.price == money(40_000))
    );
}

pub fn delete_crate<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let id = created(setup(
        a,
        "cratedel",
        "setup",
        &crate_terms(&w, None, 40_000, true),
    ));
    exercise(
        a,
        "cratedel",
        &Intent::DeleteCrate { id },
        &Intent::DeleteCrate { id: id + 1 },
        None,
        Some(&Intent::DeleteCrate { id: 999 }),
        |s| !s.crates.iter().any(|c| c.id == id),
    );
}

pub fn save_opening<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let id = w.customer.0.clone();
    let before = state(a)
        .1
        .customers
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .balance
        .minor_units();
    let opening = |customer: &str, amount: i64| Intent::SaveOpening {
        customer: customer.into(),
        amount: Some(money(amount)),
        date: Some(BillDate::new(2025, 4, 1).unwrap()),
    };
    exercise(
        a,
        "opening",
        &opening(&id, 15_000),
        &opening(&id, 16_000),
        None,
        Some(&opening("conformance-nobody", 15_000)),
        |s| {
            s.customers
                .iter()
                .any(|c| c.id == id && c.balance.minor_units() == before + 15_000)
        },
    );
}

pub fn save_godown<A: SalesAuthority>(a: &mut A) {
    let row = |cases: &str| Intent::SaveGodown {
        item: "Conformance Soda".into(),
        cases: cases.into(),
        note: "back room".into(),
    };
    exercise(
        a,
        "godown",
        &row("9"),
        &row("10"),
        None,
        Some(&row("1.5")),
        |s| {
            s.godown
                .iter()
                .any(|g| g.item == "Conformance Soda" && g.cases == 9)
        },
    );
}

pub fn count_stock<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let count = |product: u64, counted: u32, on_hand: u32| Intent::CountStock {
        product,
        counted,
        fridge: Some(2),
        date: day(),
        reason: Some("damage".into()),
        note: Some("conformance".into()),
        on_hand,
    };
    let (product, on_hand) = (w.product, w.on_hand);
    exercise(
        a,
        "count",
        &count(product, on_hand - 3, on_hand),
        &count(product, on_hand - 4, on_hand),
        Some(&count(product, on_hand - 3, on_hand + 1)),
        Some(&count(99_999, 1, 1)),
        |s| stock_of(s, product) == Some(on_hand - 3),
    );
}

/// A standalone GST bill record as the device builds it.
pub fn gst_bill(id: u64, number: u64, customer: &str) -> String {
    const LINES: [GstBillLineInput<'static>; 1] = [GstBillLineInput {
        item: "Synthetic Water 1 L",
        unit: GstUnit::Piece,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_500),
        gst_basis_points: 500,
        ..GstBillLineInput::BLANK
    }];
    let scratch = alm_core::confirmation::demo_fixture::fixture().unwrap();
    let draft = GstBillDraft {
        id: GstBillId(id),
        number: GstBillNumber(number),
        customer: "placeholder",
        date: Some(day()),
        lines: &LINES,
        ..GstBillDraft::default()
    };
    let mut bill = scratch.gst_bill_record(Role::Owner, &draft).unwrap();
    bill.customer = customer.into();
    bill.to_record_text().unwrap()
}

fn saved_gst<A: SalesAuthority>(a: &mut A, case: &str, id: u64) -> u64 {
    let number = a
        .allocate_gst_number(Role::Owner, &gst_bill_key(id))
        .unwrap();
    setup(
        a,
        case,
        "setup",
        &Intent::SaveGstBill {
            bill: gst_bill(id, number, "Conformance Buyer"),
        },
    );
    number
}

pub fn allocate_gst_number<A: SalesAuthority>(a: &mut A) {
    let key = gst_bill_key(40);
    let before = state(a);
    assert_eq!(
        a.allocate_gst_number(Role::Manager, &key),
        Err(AuthorityError::Denied)
    );
    assert_eq!(state(a), before, "Manager allocation left an effect");
    let first = a.allocate_gst_number(Role::Owner, &key).unwrap();
    assert_eq!(a.allocate_gst_number(Role::Owner, &key), Ok(first));
    assert_eq!(
        a.allocate_gst_number(Role::Owner, &gst_bill_key(41)),
        Ok(first + 1)
    );
}

pub fn save_gst_bill<A: SalesAuthority>(a: &mut A) {
    let number = a
        .allocate_gst_number(Role::Owner, &gst_bill_key(50))
        .unwrap();
    let outcome = exercise(
        a,
        "gst",
        &Intent::SaveGstBill {
            bill: gst_bill(50, number, "Conformance Buyer"),
        },
        &Intent::SaveGstBill {
            bill: gst_bill(50, number, "Other Buyer"),
        },
        Some(&Intent::SaveGstBill {
            bill: gst_bill(50, number + 1, "Conformance Buyer"),
        }),
        Some(&Intent::SaveGstBill {
            bill: gst_bill(50, number, ""),
        }),
        |s| {
            s.gst_bills
                .iter()
                .any(|b| b.id == 50 && b.number == number && !b.cancelled)
        },
    );
    assert_eq!(outcome, Outcome::GstNumber(number));
}

pub fn cancel_gst_bill<A: SalesAuthority>(a: &mut A) {
    saved_gst(a, "gstcancel", 50);
    exercise(
        a,
        "gstcancel",
        &Intent::CancelGstBill { id: 50 },
        &Intent::DeleteGstBill { id: 50 },
        None,
        Some(&Intent::CancelGstBill { id: 999 }),
        |s| s.gst_bills.iter().any(|b| b.id == 50 && b.cancelled),
    );
}

pub fn delete_gst_bill<A: SalesAuthority>(a: &mut A) {
    let number = saved_gst(a, "gstdel", 50);
    exercise(
        a,
        "gstdel",
        &Intent::DeleteGstBill { id: 50 },
        &Intent::CancelGstBill { id: 50 },
        None,
        Some(&Intent::DeleteGstBill { id: 999 }),
        |s| !s.gst_bills.iter().any(|b| b.id == 50),
    );
    // Deleting the highest-numbered bill frees its number, as in the HTML.
    assert_eq!(
        a.allocate_gst_number(Role::Owner, &gst_bill_key(51)),
        Ok(number)
    );
}

pub fn import_gst_bill<A: SalesAuthority>(a: &mut A) {
    let taken = saved_gst(a, "gstimport", 50);
    let import = |id: u64, number: u64| Intent::ImportGstBill {
        bill: Some(gst_bill(id, number, "Moved Buyer")),
        deleted: None,
    };
    exercise(
        a,
        "gstimport",
        &import(60, 60),
        &import(60, 61),
        None,
        Some(&import(61, taken)),
        |s| s.gst_bills.iter().any(|b| b.id == 60 && b.number == 60),
    );
}

/// An unknown outcome resolves by lookup against the stored intent, with no effect: nothing
/// committed before the first send, the original outcome after it, and another intent under
/// the same request is reported, never run. A Manager is denied.
pub fn lookup<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    let good = sale(&w, 151, 1, w.sell);
    let other = sale(&w, 151, 2, w.sell);
    let request = req("lookup", "1");
    let before = state(a);
    assert_eq!(
        a.lookup(Role::Owner, &request, &good),
        Ok(alm_sync::Lookup::NotCommitted)
    );
    assert_eq!(
        a.lookup(Role::Manager, &request, &good),
        Err(AuthorityError::Denied)
    );
    assert_eq!(state(a), before, "lookup has no effect");
    let first = a.submit(Role::Owner, &request, &good).unwrap();
    let after = state(a);
    assert_eq!(
        a.lookup(Role::Owner, &request, &good),
        Ok(alm_sync::Lookup::Committed(first))
    );
    assert_eq!(
        a.lookup(Role::Owner, &request, &other),
        Ok(alm_sync::Lookup::OtherIntent)
    );
    assert_eq!(state(a), after, "lookup has no effect");
}

/// A Manager pull carries the catalog only: no owner values, sales, balances or stock.
pub fn manager_pull<A: SalesAuthority>(a: &mut A) {
    let w = world(a);
    setup(a, "pull", "sale", &sale(&w, 141, 1, w.sell));
    let batch = a.pull(Role::Manager, None).unwrap();
    assert_eq!(batch.role, Role::Manager);
    let s = batch.snapshot.unwrap();
    assert!(s.owner.is_none());
    assert!(s.sales.is_empty() && s.customers.is_empty() && s.stock.is_empty());
    assert!(s.products.is_empty() && s.suppliers.is_empty() && s.receipts.is_empty());
    assert!(!s.catalog.is_empty());
    let current = a.pull(Role::Owner, None).unwrap().checkpoint;
    assert_eq!(a.pull(Role::Owner, Some(current)).unwrap().snapshot, None);
}

/// One `#[test]` per case for an authority built by `$make`.
#[macro_export]
macro_rules! conformance_tests {
    ($make:expr) => {
        $crate::conformance_tests!(
            $make;
            post_sale, cancel_sale, restore_sale, edit_sale, record_receipt, save_cash_day,
            delete_cash_day, add_product, edit_product, delete_product, add_supplier,
            set_supplier_prices, save_purchase_order, receive_purchase_order,
            close_purchase_order, save_crate, delete_crate, save_opening, save_godown,
            count_stock, allocate_gst_number, save_gst_bill, cancel_gst_bill, delete_gst_bill,
            import_gst_bill, manager_pull, lookup
        );
    };
    ($make:expr; $($case:ident),+ $(,)?) => {
        $(
            #[test]
            fn $case() {
                let Some(mut authority) = $make else { return };
                conformance::$case(&mut authority);
            }
        )+
    };
}
