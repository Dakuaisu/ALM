use super::seed::{OWNER, date, history_fixture};
use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, CustomerReceiptInput, ProductTerms, ReceiptMode,
    RequestId, SalesPeriod, SalesProfitError,
};
use crate::{
    CrateComponent, CrateRow, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, SaleUnit,
};
use std::num::NonZeroU32;

fn fixture(crate_sale: bool) -> ConfirmationFixture {
    let product = ProductId::new(1);
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        product,
        &[(10, Money::from_minor_units(300))],
        Money::from_minor_units(1000),
    )
    .unwrap();
    fixture
        .set_checkout_product(
            product,
            ProductTerms {
                pack: NonZeroU32::new(2).unwrap(),
                piece_price: Money::from_minor_units(1000),
                case_price: Money::from_minor_units(1800),
            },
        )
        .unwrap();
    fixture
        .set_loading_rate(Money::from_minor_units(200))
        .unwrap();
    fixture
        .set_crate_rate(Money::from_minor_units(1800))
        .unwrap();
    let lines = if crate_sale {
        vec![]
    } else {
        vec![DraftLine::new(product, NonZeroU32::MIN)]
    };
    let mut draft = ProvisionalDraft::new(DraftId::new(1), lines)
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("buyer".into()),
            name: "Synthetic Buyer".into(),
        })
        .with_bill_date(Some(date("2025-04-01")));
    if crate_sale {
        draft
            .add_crate_row(CrateRow::new(
                NonZeroU32::MIN,
                vec![CrateComponent::new(
                    product,
                    NonZeroU32::MIN,
                    SaleUnit::Case,
                )],
            ))
            .unwrap();
    }
    fixture.register_draft(draft).unwrap();
    let units = if crate_sale {
        &[][..]
    } else {
        &[SaleUnit::Case][..]
    };
    let review = fixture
        .review_checkout(OWNER, DraftId::new(1), units, Money::from_minor_units(0))
        .unwrap();
    fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();
    fixture
}

#[test]
fn l04_checkout_and_crate_reject_in_range_before_search_but_not_outside_range() {
    for crate_sale in [false, true] {
        let fixture = fixture(crate_sale);
        let before = fixture.clone();
        assert_eq!(
            fixture.sales_history(OWNER, date("2025-04-01"), SalesPeriod::Today, "no match"),
            Err(ConfirmationError::UnsupportedSalesRecord(DraftId::new(1)))
        );
        let outside = fixture
            .sales_history(OWNER, date("2025-03-15"), SalesPeriod::Today, "")
            .unwrap();
        assert!(outside.rows.is_empty());
        assert_eq!(fixture, before);
    }
}

#[test]
fn l04_unknown_cost_does_not_hide_rows_or_collections() {
    let product = ProductId::new(1);
    let id = DraftId::new(17);
    let customer = DraftCustomerId::new("buyer".into());
    let mut fixture = ConfirmationFixture::new(product, 1, Money::from_minor_units(500)).unwrap();
    fixture
        .register_draft(
            ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
                .with_customer(DraftCustomer::Selected {
                    id: customer.clone(),
                    name: "Synthetic Buyer".into(),
                })
                .with_bill_date(Some(date("2025-04-01"))),
        )
        .unwrap();
    let review = fixture
        .review_named(OWNER, id, Money::from_minor_units(100))
        .unwrap();
    fixture.confirm(OWNER, RequestId::new(17), &review).unwrap();
    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: customer,
                date: date("2025-04-01"),
                amount: Money::from_minor_units(50),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();
    let before = fixture.clone();
    let result = fixture.sales_history(OWNER, date("2025-04-01"), SalesPeriod::Today, "");
    assert!(result.is_ok());
    let history = result.unwrap();
    assert_eq!(history.profit, Err(SalesProfitError::UnknownFrozenCost(id)));
    assert_eq!(history.rows[0].bill.draft_id, id);
    assert_eq!(history.total_collected.minor_units(), 150);
    assert_eq!(history.collected.details.len(), 2);
    assert_eq!(fixture, before);
}

#[test]
fn l04_profit_uses_reviewed_line_subtotal_not_receipt_total() {
    let mut fixture = history_fixture();
    let (_, receipt, _) = fixture.current_sales.get_mut(&DraftId::new(73)).unwrap();
    receipt.total = Money::from_minor_units(1600);
    let before = fixture.clone();
    let history = fixture
        .sales_history(OWNER, date("2025-03-01"), SalesPeriod::Today, "")
        .unwrap();
    let profit = history.profit.unwrap();
    assert_eq!(history.sales.minor_units(), 1600);
    assert_eq!(profit.revenue.minor_units(), 1499);
    assert_eq!(profit.cost.minor_units(), 950);
    assert_eq!(profit.profit.minor_units(), 549);
    assert_eq!(fixture, before);
}
