use super::super::{CustomerReceiptInput, RequestId};
use super::*;
use crate::{DraftCustomer, DraftCustomerId, DraftId, DraftLine, ProductId, ProvisionalDraft};
use std::num::NonZeroU32;

#[test]
fn date_boundaries_are_checked_after_owner_gate() {
    let fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(1)).unwrap();
    let first = BillDate::new(1, 1, 1).unwrap();
    assert_eq!(
        fixture.dashboard(Role::Manager, first, DashboardPeriod::Week),
        Err(DashboardError::AccessDenied)
    );
    assert_eq!(
        fixture.dashboard(Role::Owner, first, DashboardPeriod::Week),
        Err(DashboardError::DateOutOfRange)
    );
    let report = fixture
        .dashboard(
            Role::Owner,
            BillDate::new(2024, 3, 1).unwrap(),
            DashboardPeriod::Month,
        )
        .unwrap();
    assert_eq!(
        report.prior.window.start,
        BillDate::new(2024, 2, 1).unwrap()
    );
    assert_eq!(report.prior.window.end, BillDate::new(2024, 2, 29).unwrap());
}

#[test]
fn identical_signed_receipts_keep_append_identity_before_filtering() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(1)).unwrap();
    let customer = DraftCustomerId::new("synthetic".into());
    fixture.register_customer(Role::Owner, &customer).unwrap();
    let day = BillDate::new(2024, 3, 1).unwrap();
    for date in [BillDate::new(2024, 2, 29).unwrap(), day, day] {
        fixture
            .record_customer_receipt(
                Role::Owner,
                CustomerReceiptInput {
                    customer_id: customer.clone(),
                    date,
                    amount: Money::from_minor_units(-7),
                    mode: ReceiptMode::Online,
                },
            )
            .unwrap();
    }
    let before = fixture.clone();
    let report = fixture
        .dashboard(Role::Owner, day, DashboardPeriod::Today)
        .unwrap();
    assert_eq!(
        report
            .collected_detail
            .iter()
            .map(|row| row.source)
            .collect::<Vec<_>>(),
        vec![
            DashboardSource::Receipt(DashboardReceiptId(2)),
            DashboardSource::Receipt(DashboardReceiptId(1))
        ]
    );
    assert_eq!(report.current.online, Money::from_minor_units(-14));
    assert_eq!(fixture, before);
}

#[test]
fn zero_paid_sale_appears_in_sales_but_not_collected_detail() {
    let product = ProductId::new(1);
    let mut fixture = ConfirmationFixture::new(product, 1, Money::from_minor_units(100)).unwrap();
    let id = DraftId::new(1);
    let day = BillDate::new(2024, 3, 1).unwrap();
    let draft = ProvisionalDraft::new(
        id,
        vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
    )
    .with_bill_date(Some(day))
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("zero-paid".into()),
        name: "Synthetic".into(),
    });
    fixture.register_draft(draft).unwrap();
    let review = fixture
        .review_named(Role::Owner, id, Money::from_minor_units(0))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    let report = fixture
        .dashboard(Role::Owner, day, DashboardPeriod::Today)
        .unwrap();
    assert_eq!(report.current.sales, Money::from_minor_units(100));
    assert_eq!(
        report.sales_ids,
        vec![DashboardSource::Bill {
            draft_id: id,
            invoice: 1
        }]
    );
    assert!(report.collected_detail.is_empty());
}
