use super::*;
use crate::confirmation::ProductTerms;
use crate::{DraftCustomerId, SaleUnit};

const ID: DraftId = DraftId::new(1);
const PRODUCT: ProductId = ProductId::new(1);
const REQUEST: RequestId = RequestId::new(19);
const RECEIVED: Money = Money::from_minor_units(40);
const PRICE: Money = Money::from_minor_units(125);

fn customer() -> DraftCustomer {
    DraftCustomer::Selected {
        id: DraftCustomerId::new("buyer".into()),
        name: "Synthetic".into(),
    }
}

fn fixture_with_checkout() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[
            (1, Money::from_minor_units(80)),
            (4, Money::from_minor_units(20)),
        ],
        PRICE,
    )
    .unwrap();
    fixture
        .set_checkout_product(
            PRODUCT,
            ProductTerms {
                pack: NonZeroU32::MIN,
                piece_price: PRICE,
                case_price: PRICE,
            },
        )
        .unwrap();
    fixture
}

fn assert_unposted(fixture: &ConfirmationFixture) {
    assert_eq!(fixture.stock(), 5);
    assert_eq!(fixture.catalog.lot_snapshot(PRODUCT), [1, 4]);
    assert!(fixture.catalog.allocation_snapshot().is_empty());
    assert_eq!(fixture.catalog.allocated_cost(ID), None);
    assert_eq!(fixture.next_invoice, 1);
    assert_eq!(fixture.ledger, 0);
    assert_eq!(fixture.received_ledger, 0);
    assert!(fixture.customer_balances.is_empty());
    assert!(fixture.requests.is_empty());
    assert!(fixture.posted.is_empty());
}

#[test]
fn l03_03_checkout_reports_the_first_missing_field_and_preserves_approval() {
    use ConfirmationError::{MissingBillDate, MissingBillableItems, MissingCustomerName};
    let mut fixture = fixture_with_checkout();
    fixture
        .register_draft(draft(1, 1).with_customer(customer()))
        .unwrap();
    let approved = fixture
        .review_checkout(Role::Owner, ID, &[SaleUnit::Case], RECEIVED)
        .unwrap();
    let approvals = BTreeMap::from([(ID, approved.clone())]);

    for (has_line, has_name, has_date, expected) in [
        (false, true, true, MissingBillableItems),
        (true, false, true, MissingCustomerName),
        (true, true, false, MissingBillDate),
        (false, false, false, MissingBillableItems),
        (false, false, true, MissingBillableItems),
        (false, true, false, MissingBillableItems),
        (true, false, false, MissingCustomerName),
    ] {
        let (lines, units) = if has_line {
            (
                vec![DraftLine::new(PRODUCT, NonZeroU32::MIN)],
                &[SaleUnit::Case][..],
            )
        } else {
            (vec![], &[][..])
        };
        let customer = if has_name {
            customer()
        } else {
            DraftCustomer::Unassigned
        };
        fixture
            .register_draft(
                ProvisionalDraft::new(ID, lines)
                    .with_customer(customer)
                    .with_bill_date(if has_date { test_bill_date() } else { None }),
            )
            .unwrap();
        assert_eq!(
            fixture.review_checkout(Role::Manager, ID, units, RECEIVED),
            Err(ConfirmationError::AccessDenied)
        );
        assert_eq!(
            fixture.review_checkout(Role::Owner, ID, units, RECEIVED),
            Err(expected)
        );
        assert_eq!(
            fixture.confirm(Role::Owner, REQUEST, &approved),
            Err(ConfirmationError::StaleReview)
        );
        assert_unposted(&fixture);
        assert_eq!(fixture.checkout_approvals, approvals);
    }
}

#[test]
fn l03_03_name_guard_uses_truthiness_before_stricter_payment_validation() {
    let mut fixture = fixture_with_checkout();
    for (customer, date, expected) in [
        (
            DraftCustomer::Typed(String::new()),
            test_bill_date(),
            ConfirmationError::MissingCustomerName,
        ),
        (
            DraftCustomer::Selected {
                id: DraftCustomerId::new("buyer".into()),
                name: String::new(),
            },
            test_bill_date(),
            ConfirmationError::MissingCustomerName,
        ),
        (
            DraftCustomer::Typed(" ".into()),
            None,
            ConfirmationError::MissingBillDate,
        ),
        (
            DraftCustomer::Typed(" ".into()),
            test_bill_date(),
            ConfirmationError::InvalidDraft,
        ),
        (
            DraftCustomer::Selected {
                id: DraftCustomerId::new(String::new()),
                name: "Synthetic".into(),
            },
            test_bill_date(),
            ConfirmationError::InvalidDraft,
        ),
    ] {
        fixture
            .register_draft(draft(1, 1).with_customer(customer).with_bill_date(date))
            .unwrap();
        assert_eq!(
            fixture.review_checkout(Role::Owner, ID, &[SaleUnit::Case], RECEIVED),
            Err(expected)
        );
        assert_unposted(&fixture);
        assert!(fixture.checkout_approvals.is_empty());
    }
}
