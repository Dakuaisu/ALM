use super::seed::{OWNER, date, history_fixture};
use crate::confirmation::{ConfirmationError, CreationKey, DashboardPeriod, SalesPeriod};
use crate::{DraftId, Money};

#[test]
fn l04_month_includes_future_records_and_custom_end_is_inclusive() {
    let fixture = history_fixture();
    let today = date("2025-03-15");
    let month = fixture
        .sales_history(OWNER, today, SalesPeriod::ThisMonth, "")
        .unwrap();
    let bounded = fixture
        .sales_history(
            OWNER,
            today,
            SalesPeriod::Custom((Some(date("2025-03-01")), Some(today))),
            "",
        )
        .unwrap();
    assert_eq!(month.sales.minor_units(), 6145);
    assert_eq!(bounded.sales.minor_units(), 4047);
    assert!(
        month
            .rows
            .iter()
            .any(|row| row.bill.draft_id == DraftId::new(41))
    );
    assert!(
        !bounded
            .rows
            .iter()
            .any(|row| row.bill.draft_id == DraftId::new(41))
    );
}

#[test]
fn l04_creation_order_uses_keys_then_newest_post_and_missing_zero() {
    let mut fixture = history_fixture();
    for (id, key) in &mut fixture.first_posts {
        *key = match id.value() {
            41 | 36 => Some(CreationKey::new(9)),
            12 => Some(CreationKey::new(0)),
            73 => Some(CreationKey::new(5)),
            24 | 58 => None,
            _ => unreachable!(),
        };
    }
    let before = fixture.clone();
    let history = fixture
        .sales_history(OWNER, date("2025-03-15"), SalesPeriod::All, "")
        .unwrap();
    assert_eq!(
        history
            .rows
            .iter()
            .map(|row| row.bill.draft_id.value())
            .collect::<Vec<_>>(),
        [36, 41, 73, 58, 24, 12]
    );
    assert_eq!(fixture, before);
}

#[test]
fn l04_dashboard_today_matches_history_on_supported_control() {
    let mut fixture = history_fixture();
    let guest = DraftId::new(58);
    fixture.current_sales.remove(&guest);
    fixture.first_posts.retain(|(id, _)| *id != guest);
    let before = fixture.clone();
    let today = date("2025-03-15");
    let history = fixture
        .sales_history(OWNER, today, SalesPeriod::Today, "")
        .unwrap();
    let dashboard = fixture
        .dashboard(OWNER, today, DashboardPeriod::Today)
        .unwrap();
    assert_eq!(
        (
            history.sales,
            history.bill_paid,
            history.receipt_paid,
            history.total_collected
        ),
        (
            dashboard.current.sales,
            dashboard.current.bill_paid,
            dashboard.current.receipts,
            dashboard.current.collected
        )
    );
    assert_eq!(fixture, before);
}

#[test]
fn l04_history_reports_overflow_instead_of_wrapping() {
    let mut fixture = history_fixture();
    fixture.preview_payments[0].amount = Money::from_minor_units(i64::MAX);
    fixture.preview_payments[1].amount = Money::from_minor_units(1);
    let before = fixture.clone();
    assert_eq!(
        fixture.sales_history(OWNER, date("2025-03-15"), SalesPeriod::All, ""),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(fixture, before);
}
