use super::{ConfirmationError, ConfirmationFixture, Money, SalesRow, add, subtract};
use crate::DraftId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalesProfit {
    pub bill_ids: Vec<DraftId>,
    pub revenue: Money,
    pub cost: Money,
    pub profit: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SalesProfitError {
    UnknownFrozenCost(DraftId),
}

pub(super) fn calculate(
    fixture: &ConfirmationFixture,
    rows: &[SalesRow],
) -> Result<Result<SalesProfit, SalesProfitError>, ConfirmationError> {
    let zero = Money::from_minor_units(0);
    let mut result = SalesProfit {
        bill_ids: Vec::new(),
        revenue: zero,
        cost: zero,
        profit: zero,
    };
    for row in rows.iter().filter(|row| !row.cancelled) {
        let id = row.bill.draft_id;
        let Some(cost) = fixture.catalog.frozen_cost(id)? else {
            return Ok(Err(SalesProfitError::UnknownFrozenCost(id)));
        };
        let (review, _, _) = fixture
            .current_sales
            .get(&id)
            .ok_or(ConfirmationError::InvalidDraft)?;
        result.bill_ids.push(id);
        result.revenue = add(result.revenue, review.amounts()?.subtotal)?;
        result.cost = add(result.cost, Money::from_minor_units(cost))?;
    }
    result.profit = subtract(result.revenue, result.cost)?;
    Ok(Ok(result))
}
