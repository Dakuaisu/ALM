use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{CrateComponent, CrateRow, Money, ProductId, ProvisionalDraft, Role, SaleUnit};

use super::{ConfirmationError, ConfirmationFixture};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct CrateTemplateId(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
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
    Overflow,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub(super) struct CrateTemplateBook {
    last_id: u64,
    saved: BTreeMap<CrateTemplateId, CrateTemplate>,
    piece_mrps: BTreeMap<u64, Money>,
}

impl CrateTemplateId {
    /// Fixture-local template number, for carrying a selection across IPC.
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl CrateTemplateBook {
    pub(super) fn saved_templates(
        &self,
    ) -> impl Iterator<Item = (CrateTemplateId, &CrateTemplate)> {
        self.saved.iter().map(|(id, template)| (*id, template))
    }
}

fn gate(role: Role, capability: crate::permissions::Capability) -> Result<(), CrateTemplateError> {
    crate::permissions::require(role, capability).map_err(|_| CrateTemplateError::AccessDenied)
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
        gate(role, crate::permissions::Capability::Crates)?;
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
        gate(role, crate::permissions::Capability::SaveCrate)?;
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
        gate(role, crate::permissions::Capability::Crates)?;
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
        gate(role, crate::permissions::Capability::SaveCrate)?;
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
        gate(role, crate::permissions::Capability::DeleteCrate)?;
        self.crate_templates
            .saved
            .remove(&id)
            .map(|_| ())
            .ok_or(CrateTemplateError::NotFound)
    }

    /// Copies the template as a crate row after the draft's rows (`draft.items.push`); does not
    /// retain a template reference.
    pub fn load_crate_template(
        &self,
        role: Role,
        id: CrateTemplateId,
        quantity: NonZeroU32,
        draft: &mut ProvisionalDraft,
    ) -> Result<(), CrateTemplateError> {
        let template = self.crate_template(role, id)?;
        let row = CrateRow::new(quantity, template.components.clone())
            .with_price(template.name.clone(), template.price);
        draft.add_crate_row(row);
        Ok(())
    }
}

impl CrateTemplateBook {
    pub(super) fn book(&self) -> crate::projection::book::BookCrates {
        use crate::projection::book::{BookCrate, BookCrates};
        BookCrates {
            last_id: self.last_id,
            templates: self
                .saved
                .iter()
                .map(|(id, t)| BookCrate {
                    id: id.0,
                    name: t.name.clone(),
                    price: t.price,
                    components: t
                        .components
                        .iter()
                        .map(|c| {
                            (
                                c.product.value(),
                                c.quantity.get(),
                                c.unit == SaleUnit::Piece,
                            )
                        })
                        .collect(),
                })
                .collect(),
            piece_mrps: self.piece_mrps.iter().map(|(p, m)| (*p, *m)).collect(),
        }
    }

    pub(super) fn from_book(
        book: &crate::projection::book::BookCrates,
    ) -> Result<Self, ConfirmationError> {
        let mut saved = BTreeMap::new();
        for t in &book.templates {
            let components = t
                .components
                .iter()
                .map(|&(product, quantity, piece)| {
                    Ok(CrateComponent::new(
                        ProductId::new(product),
                        NonZeroU32::new(quantity).ok_or(ConfirmationError::InvalidDraft)?,
                        if piece {
                            SaleUnit::Piece
                        } else {
                            SaleUnit::Case
                        },
                    ))
                })
                .collect::<Result<_, ConfirmationError>>()?;
            saved.insert(
                CrateTemplateId(t.id),
                CrateTemplate {
                    name: t.name.clone(),
                    components,
                    price: t.price,
                },
            );
        }
        Ok(Self {
            last_id: book.last_id,
            saved,
            piece_mrps: book.piece_mrps.iter().copied().collect(),
        })
    }
}
