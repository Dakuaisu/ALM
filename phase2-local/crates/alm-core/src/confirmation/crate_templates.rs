use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{CrateComponent, CrateRow, Money, ProductId, ProvisionalDraft, Role, SaleUnit};

use super::{ConfirmationError, ConfirmationFixture};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CrateTemplateId(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrateTemplate {
    pub name: String,
    pub components: Vec<CrateComponent>,
    pub price: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrateTemplateError {
    AccessDenied,
    NotFound,
    Empty,
    InvalidPrice,
    MissingPieceMrp(ProductId),
    DraftHasRows,
    Overflow,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct CrateTemplateBook {
    last_id: u64,
    saved: BTreeMap<CrateTemplateId, CrateTemplate>,
    piece_mrps: BTreeMap<u64, Money>,
}

const fn owner(role: Role) -> Result<(), CrateTemplateError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(CrateTemplateError::AccessDenied),
    }
}

impl CrateTemplate {
    fn checked(mut self) -> Result<Self, CrateTemplateError> {
        if self.components.is_empty() {
            return Err(CrateTemplateError::Empty);
        }
        if self.price.minor_units() < 0 {
            return Err(CrateTemplateError::InvalidPrice);
        }
        if self.name.is_empty() {
            self.name = "Crate".into();
        }
        Ok(self)
    }
}

impl ConfirmationFixture {
    /// Seeds synthetic Piece MRP, independent of the product's checkout price.
    pub fn set_crate_piece_mrp(
        &mut self,
        product: ProductId,
        mrp: Money,
    ) -> Result<(), ConfirmationError> {
        if self.catalog.stock(product).is_none() {
            return Err(ConfirmationError::InvalidDraft);
        }
        if mrp.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        self.crate_templates.piece_mrps.insert(product.value(), mrp);
        Ok(())
    }

    /// Computes the current suggestion for one crate without changing the stored price.
    pub fn suggest_crate_price(
        &self,
        role: Role,
        components: &[CrateComponent],
    ) -> Result<Money, CrateTemplateError> {
        owner(role)?;
        components
            .iter()
            .try_fold(0_i64, |sum, component| {
                let Some(sell) = self.catalog.sell_rate(component.product) else {
                    return Ok(sum);
                };
                let rate = match component.unit {
                    SaleUnit::Piece => *self
                        .crate_templates
                        .piece_mrps
                        .get(&component.product.value())
                        .ok_or(CrateTemplateError::MissingPieceMrp(component.product))?,
                    SaleUnit::Case => sell,
                };
                rate.minor_units()
                    .checked_mul(i64::from(component.quantity.get()))
                    .and_then(|amount| sum.checked_add(amount))
                    .ok_or(CrateTemplateError::Overflow)
            })
            .map(Money::from_minor_units)
    }

    pub fn save_crate_template(
        &mut self,
        role: Role,
        template: CrateTemplate,
    ) -> Result<CrateTemplateId, CrateTemplateError> {
        owner(role)?;
        let template = template.checked()?;
        let next = self
            .crate_templates
            .last_id
            .checked_add(1)
            .ok_or(CrateTemplateError::Overflow)?;
        let id = CrateTemplateId(next);
        self.crate_templates.saved.insert(id, template);
        self.crate_templates.last_id = next;
        Ok(id)
    }

    pub fn crate_template(
        &self,
        role: Role,
        id: CrateTemplateId,
    ) -> Result<&CrateTemplate, CrateTemplateError> {
        owner(role)?;
        self.crate_templates
            .saved
            .get(&id)
            .ok_or(CrateTemplateError::NotFound)
    }

    pub fn edit_crate_template(
        &mut self,
        role: Role,
        id: CrateTemplateId,
        template: CrateTemplate,
    ) -> Result<(), CrateTemplateError> {
        owner(role)?;
        let saved = self
            .crate_templates
            .saved
            .get_mut(&id)
            .ok_or(CrateTemplateError::NotFound)?;
        *saved = template.checked()?;
        Ok(())
    }

    pub fn delete_crate_template(
        &mut self,
        role: Role,
        id: CrateTemplateId,
    ) -> Result<(), CrateTemplateError> {
        owner(role)?;
        self.crate_templates
            .saved
            .remove(&id)
            .map(|_| ())
            .ok_or(CrateTemplateError::NotFound)
    }

    /// Copies the template as the only row in a draft; does not retain a template reference.
    pub fn load_crate_template(
        &self,
        role: Role,
        id: CrateTemplateId,
        quantity: NonZeroU32,
        draft: &mut ProvisionalDraft,
    ) -> Result<(), CrateTemplateError> {
        let template = self.crate_template(role, id)?;
        if !draft.lines().is_empty() {
            return Err(CrateTemplateError::DraftHasRows);
        }
        let row = CrateRow::new(quantity, template.components.clone())
            .with_price(template.name.clone(), template.price);
        draft
            .add_crate_row(row)
            .map_err(|_| CrateTemplateError::DraftHasRows)
    }
}
