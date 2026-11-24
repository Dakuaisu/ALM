use super::*;
use alm_core::confirmation::demo_fixture::{CUSTOMER, customer_id, directory, fixture};
use alm_core::confirmation::owner_billing::{OwnerPosting, SaleTerms};

const DAY: &str = "2025-04-30";

pub(crate) fn day() -> BillDate {
    DAY.parse().expect("fixed test date")
}

fn review(fixture: &mut ConfirmationFixture) -> OwnerPosting {
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            fixture,
            &directory(),
            Role::Owner,
            Some(&draft()),
            SaleTerms {
                date: DAY,
                paid: "20.00",
            },
        )
        .expect("review");
    posting
}

/// Posts the reviewed bill #1: ₹120.50 total, ₹20 paid in cash on 2025-04-30.
pub(crate) fn posted(fixture: &mut ConfirmationFixture) {
    review(fixture)
        .confirm(fixture, Role::Owner, 1)
        .expect("post");
}

pub(crate) fn draft() -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: CUSTOMER.into(),
    })
}

#[test]
fn sales_show_confirmed_owner_amount_not_provisional_amount() {
    let mut fixture = fixture().expect("synthetic fixture");
    let posting = review(&mut fixture);
    assert_eq!(fixture.posted_count(), 0);
    let receipt = posting.confirm(&mut fixture, Role::Owner, 1).expect("post");
    assert_eq!(receipt.total.minor_units(), 12_050);
    assert_eq!(posting.confirm(&mut fixture, Role::Owner, 1), Ok(receipt));
    assert_eq!(fixture.posted_count(), 1);
    let html = views::sales_screen(
        &projected(&fixture),
        Role::Owner,
        &Default::default(),
        day(),
    )
    .expect("sales");
    assert!(html.contains("<td class=\"num\">₹121</td>"));
    assert!(html.contains("₹20 bills"));
    assert!(html.contains("Synthetic Customer"));
}

#[test]
fn manager_cannot_confirm_an_owner_review_or_read_sales() {
    let mut fixture = fixture().expect("synthetic fixture");
    let posting = review(&mut fixture);
    assert!(posting.confirm(&mut fixture, Role::Manager, 1).is_err());
    assert_eq!(fixture.posted_count(), 0);
    let html = views::sales_screen(
        &projected(&fixture),
        Role::Manager,
        &Default::default(),
        day(),
    )
    .expect("denial");
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("tbl-wrap"));
    assert!(!html.contains('₹'));
}

#[test]
fn standalone_receipt_changes_sales_collected_and_survives_cancellation() {
    let mut fixture = fixture().expect("synthetic fixture");
    let posting = review(&mut fixture);
    posting.confirm(&mut fixture, Role::Owner, 1).expect("post");
    crate::receipts::save(&mut fixture, &directory(), Role::Owner, receipt_terms())
        .expect("receipt");
    let html =
        views::receipt_screen(&fixture, &directory(), Role::Owner, day()).expect("receipt screen");
    assert!(html.contains(r#"data-search="receipt_customer_search""#));
    let options =
        views::receipt_customer_options(&fixture, &directory(), Role::Owner, "customer").unwrap();
    assert!(
        options.contains("<span>Synthetic Customer</span><small>balance ₹70</small>"),
        "{options}"
    );
    fixture
        .cancel_sale(Role::Owner, DraftId::new(1))
        .expect("cancel");
    let sales = views::sales_screen(
        &projected(&fixture),
        Role::Owner,
        &Default::default(),
        day(),
    )
    .expect("sales");
    assert!(sales.contains("(₹0 bills · ₹31 received)"));
    assert!(sales.contains("CANCELLED"));
}

#[test]
fn manager_cannot_save_a_standalone_receipt_or_read_its_form() {
    let mut fixture = fixture().expect("synthetic fixture");
    assert!(
        crate::receipts::save(&mut fixture, &directory(), Role::Manager, receipt_terms()).is_err()
    );
    let html = views::receipt_screen(&fixture, &directory(), Role::Manager, day()).expect("denial");
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("<form"));
    assert_eq!(fixture.customer_due(customer_id().value()).minor_units(), 0);
}

fn receipt_terms() -> crate::receipts::Terms {
    crate::receipts::Terms {
        customer: "batch1-customer".into(),
        date: "2025-05-01".into(),
        amount: "30.50".into(),
        mode: crate::receipts::Mode::Online,
    }
}

#[test]
fn new_bill_renders_connected_choices_in_source_layout() {
    let root = std::env::temp_dir().join(format!("alm-bill-markup-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut trial = Trial::open(&root, Some(day())).unwrap();
    let mut html = trial.render("bill").unwrap();
    html += &trial.bill_customer_search("");
    html += &trial.bill_item_search("");
    drop(trial);
    std::fs::remove_dir_all(root).unwrap();
    for markup in [
        "data-title=\"New Bill\"",
        "class=\"f3\"",
        r#"data-search="bill_customer_search""#,
        r#"data-search="bill_item_search""#,
        r#"data-fill="customer=batch1-customer&amp;customer_q=Synthetic%20Customer" data-submit="select""#,
        r#"data-fill="customer=batch1-market&amp;customer_q=Synthetic%20Market""#,
        r#"data-fill="product=1&amp;product_q=Synthetic%20Lime" data-submit="add"><span>Synthetic Lime"#,
        r#"data-fill="product=2&amp;product_q=Synthetic%20Soda" data-submit="add"><span>Synthetic Soda"#,
        "Bill Amount",
    ] {
        assert!(html.contains(markup), "missing {markup}");
    }
}

#[test]
fn review_screen_amounts_equal_the_core_projection() {
    let fixture = fixture().expect("synthetic fixture");
    let draft = ProvisionalDraft::new(
        DraftId::new(7),
        vec![
            DraftLine::new(ProductId::new(1), NonZeroU32::new(3).unwrap()),
            DraftLine::new(ProductId::new(2), NonZeroU32::new(2).unwrap()),
        ],
    );
    let priced = alm_core::confirmation::owner_billing::priced_draft(&fixture, Role::Owner, &draft)
        .expect("priced");
    let html = views::review_screen(&views::ReviewView {
        role_is_owner: true,
        draft: Some(&draft),
        priced: Ok(priced.clone()),
    });
    for line in &priced.lines {
        assert!(html.contains(&format!("₹{}", gst_print::inr(line.amount))));
    }
    assert_eq!(priced.total.minor_units(), 48_650);
    assert!(html.contains("<b>Owner total ₹486.50</b>"), "{html}");
    let denied = views::review_screen(&views::ReviewView {
        role_is_owner: false,
        draft: Some(&draft),
        priced: alm_core::confirmation::owner_billing::priced_draft(
            &fixture,
            Role::Manager,
            &draft,
        ),
    });
    assert!(!denied.contains('₹'));
}

#[test]
fn counter_lists_counts_and_recent_bills_without_amounts() {
    let mut fixture = fixture().expect("synthetic fixture");
    review(&mut fixture)
        .confirm(&mut fixture, Role::Owner, 1)
        .expect("post");
    let core = fixture.counter(Role::Owner, day()).expect("counter");
    let html = views::counter_screen(&fixture, Role::Owner, day()).expect("counter");
    assert_eq!(core.billed_today_count, 1);
    assert_eq!(core.attention.debtors, 1);
    assert!(html.contains("1 bill billed today · 30-Apr-2025"));
    assert!(html.contains("<b>1</b> customer to collect from"));
    assert!(html.contains(r#"<div class="rnm">Synthetic Customer</div><div class="rsub">#1 · 30-Apr-2025</div></span><span class="tag credit">Credit</span>"#));
    assert!(!html.contains('₹'), "Counter carries no amounts");
    let manager = views::counter_screen(&fixture, Role::Manager, day()).expect("denial");
    assert!(manager.contains("role=\"alert\""));
    assert!(!manager.contains("Synthetic Customer"));
}

#[test]
fn sales_balance_column_shows_the_core_balance_side() {
    let mut fixture = fixture().expect("synthetic fixture");
    review(&mut fixture)
        .confirm(&mut fixture, Role::Owner, 1)
        .expect("post");
    let html = views::sales_screen(
        &projected(&fixture),
        Role::Owner,
        &Default::default(),
        day(),
    )
    .unwrap();
    assert!(html.contains(r#"<span class="tag credit">Credit</span></td><td class="num">₹121</td><td class="num neg">₹101</td>"#), "{html}");
    crate::receipts::save(
        &mut fixture,
        &directory(),
        Role::Owner,
        crate::receipts::Terms {
            customer: "batch1-customer".into(),
            date: "2025-05-01".into(),
            amount: "150.49".into(),
            mode: crate::receipts::Mode::Cash,
        },
    )
    .unwrap();
    assert_eq!(
        fixture.customer_due(customer_id().value()).minor_units(),
        -4_999
    );
    let html = views::sales_screen(
        &projected(&fixture),
        Role::Owner,
        &Default::default(),
        day(),
    )
    .unwrap();
    assert!(
        html.contains(r#"<td class="num pos">₹50 adv</td>"#),
        "{html}"
    );
}

#[test]
fn screens_display_ddisp_dates_and_forms_keep_iso() {
    let mut fixture = fixture().expect("synthetic fixture");
    posted(&mut fixture);
    let sales = views::sales_screen(
        &projected(&fixture),
        Role::Owner,
        &Default::default(),
        day(),
    )
    .unwrap();
    assert!(sales.contains("today: 30-Apr-2025"));
    assert!(sales.contains("<tr style=\"\"><td>30-Apr-2025</td>"));
    let receipt = views::receipt_screen(&fixture, &directory(), Role::Owner, day()).unwrap();
    assert!(receipt.contains(r#"type="date" name="date" value="2025-04-30""#));
    let counter = views::counter_screen(&fixture, Role::Owner, day()).unwrap();
    assert!(counter.contains("1 bill billed today · 30-Apr-2025"));
}

/// The Owner projection of `fixture`, as the device would store it.
pub(crate) fn projected(fixture: &ConfirmationFixture) -> alm_core::projection::ProjectionSnapshot {
    fixture
        .projection(
            Role::Owner,
            &directory(),
            &alm_core::confirmation::manual_cashbook::ManualCashBook::default(),
        )
        .expect("projection")
}
