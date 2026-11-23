use super::*;
use crate::DraftCustomerId;
use std::time::Duration;

const CUSTOMER: &str = "A&B </title><script>alert('x')</script> \"Shop\"";
const ESCAPED_CUSTOMER: &str =
    "A&amp;B &lt;/title&gt;&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &quot;Shop&quot;";
const LINES: &[EstimateLineDisplay<'static>] = &[EstimateLineDisplay {
    product: ProductId::new(1),
    label: "Fixture Cola & <case>",
}];

fn estimate_fixture() -> ConfirmationFixture {
    estimate_fixture_with_costs(&[
        (1, Money::from_minor_units(4_321)),
        (4, Money::from_minor_units(5_678)),
    ])
}

pub(super) fn estimate_fixture_with_costs(lots: &[(u32, Money)]) -> ConfirmationFixture {
    let mut f = l05_01::preview_fixture(5);
    f.catalog.seed_lots(ProductId::new(1), lots).unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("a".into()),
        name: CUSTOMER.into(),
    }))
    .unwrap();
    let review = f
        .review_named(
            Role::Owner,
            DraftId::new(1),
            Money::from_minor_units(10_000),
        )
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    f.preview_payments.push(PreviewPayment {
        customer_id: "a".into(),
        amount: Money::from_minor_units(6_000),
        date: BillDate::new(2024, 3, 1).unwrap(),
        after_invoice: 1,
        mode: None,
        metadata: None,
    });
    f
}

pub(super) fn request(role: Role, balance_mode: BalanceMode) -> EstimateRequest<'static> {
    EstimateRequest {
        role,
        draft_id: DraftId::new(1),
        balance_mode,
        display: EstimateDisplay {
            today: BillDate::new(2024, 3, 15).unwrap(),
            lines: LINES,
        },
    }
}

fn assert_safe(value: &str) {
    let lower = value.to_ascii_lowercase();
    for secret in [
        "cost", "profit", "4321", "5678", "9999", "15001", "43.21", "56.78", "99.99", "150.01",
    ] {
        assert!(!lower.contains(secret), "private value leaked: {secret}");
    }
}

fn assert_capture(document: &PopupDocument, mode: BalanceMode) {
    let html = &document.html;
    assert!(html.starts_with("<!doctype html><html><head>"));
    assert!(html.ends_with("</body></html>"));
    assert!(html.contains(&format!("<title>Estimate-1-{ESCAPED_CUSTOMER}</title>")));
    assert!(html.contains("<p>Date: 29-Feb-2024</p>"));
    assert!(html.contains(&format!("<p>Bill To: {ESCAPED_CUSTOMER}</p>")));
    assert!(html.contains(
        "<td>Fixture Cola &amp; &lt;case&gt;</td><td>2</td><td>₹125.00</td><td>₹250.00</td>"
    ));
    assert!(html.contains("<th colspan=\"2\">Items Total</th><td>2</td><td>₹250.00</td>"));
    assert!(html.contains("<th>Loading/Delivery</th><td>₹0.00</td>"));
    assert!(html.contains("<th>Bill Amount</th><td>₹250.00</td>"));
    let (paid, balance) = match mode {
        BalanceMode::AsOnBillDate => ("Paid", "<th>Balance Due</th><td>₹150.00</td>"),
        BalanceMode::Latest => (
            "Paid on this bill",
            "<th>Total Due (as on 15-Mar-2024)</th><td>₹90.00</td>",
        ),
    };
    assert!(html.contains(&format!("<th>{paid}</th><td>₹100.00</td>")));
    assert!(html.contains(balance));
    let lower = html.to_ascii_lowercase();
    assert!(!lower.contains("<script"));
    assert!(!lower.contains("onload="));
    assert!(!lower.contains("javascript:"));
    assert_safe(html);
}

fn print_case(mode: BalanceMode) {
    let f = estimate_fixture();
    let before = f.clone();
    let projection = f.bill_preview(Role::Owner, DraftId::new(1), mode).unwrap();
    assert_safe(&format!("{projection:?}"));
    let mut captured = Vec::new();
    f.print_estimate(request(Role::Owner, mode), &mut |popup| {
        captured.push(popup)
    })
    .unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].target, "_print");
    assert_eq!(captured[0].intent, PrintIntent::OnLoad);
    assert_capture(&captured[0], mode);
    assert_eq!(f, before);
}

fn pdf_case(mode: BalanceMode) {
    let f = estimate_fixture();
    let before = f.clone();
    let projection = f.bill_preview(Role::Owner, DraftId::new(1), mode).unwrap();
    assert_safe(&format!("{projection:?}"));
    let mut captured = Vec::new();
    f.save_estimate_as_pdf(request(Role::Owner, mode), &mut |popup| {
        captured.push(popup);
    })
    .unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].target, "_pdf");
    assert_eq!(
        captured[0].intent,
        PrintIntent::AfterLoad(Duration::from_millis(350))
    );
    assert_capture(&captured[0], mode);
    assert_eq!(f, before);
}

#[test]
fn l05_02_print_uses_as_on_bill_date_balance() {
    print_case(BalanceMode::AsOnBillDate);
}

#[test]
fn l05_02_print_uses_latest_balance() {
    print_case(BalanceMode::Latest);
}

#[test]
fn l05_02_pdf_uses_as_on_bill_date_balance() {
    pdf_case(BalanceMode::AsOnBillDate);
}

#[test]
fn l05_02_pdf_uses_latest_balance() {
    pdf_case(BalanceMode::Latest);
}

#[test]
fn l05_02_denied_actions_do_not_emit_or_mutate() {
    let f = estimate_fixture();
    let before = f.clone();
    let mut captured = Vec::new();
    let request = request(Role::Manager, BalanceMode::Latest);
    assert_eq!(
        f.print_estimate(request, &mut |popup| captured.push(popup)),
        Err(EstimateError::Preview(ConfirmationError::AccessDenied))
    );
    assert_eq!(
        f.save_estimate_as_pdf(request, &mut |popup| captured.push(popup)),
        Err(EstimateError::Preview(ConfirmationError::AccessDenied))
    );
    assert!(captured.is_empty());
    assert_eq!(f, before);
}

#[test]
fn l05_02_walk_in_latest_uses_bill_date_source() {
    let mut f = l05_01::preview_fixture(5);
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Typed("Local - Neighbor".into())))
        .unwrap();
    let review = f.review_walk_in_cash(Role::Owner, DraftId::new(1)).unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let before = f.clone();
    let mut captured = Vec::new();
    f.print_estimate(request(Role::Owner, BalanceMode::Latest), &mut |popup| {
        captured.push(popup);
    })
    .unwrap();
    assert_eq!(captured.len(), 1);
    assert!(
        captured[0]
            .html
            .contains("<th>Balance Due</th><td>₹0.00</td>")
    );
    assert_eq!(f, before);
}

#[test]
fn l05_02_cancelled_estimate_preserves_negative_latest_balance() {
    let mut f = estimate_fixture();
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    let before = f.clone();
    let mut captured = Vec::new();
    f.save_estimate_as_pdf(request(Role::Owner, BalanceMode::Latest), &mut |popup| {
        captured.push(popup);
    })
    .unwrap();
    assert_eq!(captured.len(), 1);
    assert!(captured[0].html.contains("<p>CANCELLED</p>"));
    assert!(
        captured[0]
            .html
            .contains("<th>Total Due (as on 15-Mar-2024)</th><td>₹-60.00</td>")
    );
    assert_safe(&captured[0].html);
    assert_eq!(f, before);
}

#[test]
fn l05_02_later_estimate_includes_the_previous_balance_and_date() {
    let mut f = estimate_fixture();
    f.register_draft(draft(2, 1).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("a".into()),
        name: CUSTOMER.into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(2), Money::from_minor_units(0))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(2), &review).unwrap();
    let before = f.clone();
    let mut captured = Vec::new();
    f.print_estimate(
        EstimateRequest {
            draft_id: DraftId::new(2),
            ..request(Role::Owner, BalanceMode::AsOnBillDate)
        },
        &mut |popup| captured.push(popup),
    )
    .unwrap();
    assert_eq!(captured.len(), 1);
    assert!(
        captured[0]
            .html
            .contains("<th>Prev Balance (as on 01-Mar-2024)</th><td>₹90.00</td>")
    );
    assert!(
        captured[0]
            .html
            .contains("<th>Balance Due</th><td>₹215.00</td>")
    );
    assert_eq!(f, before);
}

#[test]
fn l05_02_overflow_does_not_emit_or_mutate() {
    let mut f = estimate_fixture();
    f.preview_payments.push(PreviewPayment {
        customer_id: "a".into(),
        amount: Money::from_minor_units(i64::MIN),
        date: BillDate::new(2024, 3, 2).unwrap(),
        after_invoice: 1,
        mode: None,
        metadata: None,
    });
    let before = f.clone();
    let mut captured = Vec::new();
    let request = request(Role::Owner, BalanceMode::Latest);
    assert_eq!(
        f.print_estimate(request, &mut |popup| captured.push(popup)),
        Err(EstimateError::Preview(ConfirmationError::Overflow))
    );
    assert_eq!(
        f.save_estimate_as_pdf(request, &mut |popup| captured.push(popup)),
        Err(EstimateError::Preview(ConfirmationError::Overflow))
    );
    assert!(captured.is_empty());
    assert_eq!(f, before);
}
