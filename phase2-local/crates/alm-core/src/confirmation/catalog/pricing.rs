use crate::{Money, confirmation::ConfirmationError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductPriceInput {
    Buy(Money),
    Margin(Money),
    Sell(Money),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct ProductPricing {
    buy: Money,
    margin: Money,
    sell: Money,
}

impl ProductPricing {
    pub(super) const fn from_parts(buy: Money, margin: Money, sell: Money) -> Self {
        Self { buy, margin, sell }
    }

    pub(super) const fn at_sell(sell: Money) -> Self {
        Self {
            buy: Money::from_minor_units(0),
            margin: sell,
            sell,
        }
    }

    pub const fn buy(self) -> Money {
        self.buy
    }

    pub const fn margin(self) -> Money {
        self.margin
    }

    pub const fn sell(self) -> Money {
        self.sell
    }

    pub(super) fn set_receipt_buy(&mut self, buy: Money) -> Result<(), ConfirmationError> {
        let margin = self
            .sell
            .minor_units()
            .checked_sub(buy.minor_units())
            .ok_or(ConfirmationError::Overflow)?;
        self.buy = buy;
        self.margin = Money::from_minor_units(margin);
        Ok(())
    }

    pub(super) fn apply(&mut self, input: ProductPriceInput) -> Result<(), ConfirmationError> {
        let amount = |value: Option<i64>| {
            value
                .map(Money::from_minor_units)
                .ok_or(ConfirmationError::Overflow)
        };
        match input {
            ProductPriceInput::Buy(buy) => {
                self.sell = amount(buy.minor_units().checked_add(self.margin.minor_units()))?;
                self.buy = buy;
            }
            ProductPriceInput::Margin(margin) => {
                self.sell = amount(self.buy.minor_units().checked_add(margin.minor_units()))?;
                self.margin = margin;
            }
            ProductPriceInput::Sell(sell) => {
                self.margin = amount(sell.minor_units().checked_sub(self.buy.minor_units()))?;
                self.sell = sell;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l09_01_overflow_leaves_all_price_fields_unchanged() {
        let before = ProductPricing::at_sell(Money::from_minor_units(i64::MAX));
        let mut pricing = before;

        assert_eq!(
            pricing.apply(ProductPriceInput::Buy(Money::from_minor_units(1))),
            Err(ConfirmationError::Overflow)
        );
        assert_eq!(pricing, before);
    }
}
