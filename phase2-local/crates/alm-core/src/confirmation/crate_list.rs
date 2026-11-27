//! Crates screen (legacy VIEWS.crates): saved templates with contents text and today's value.
use std::num::NonZeroU32;

use super::{ConfirmationFixture, CrateTemplate, CrateTemplateError, CrateTemplateId};
use crate::{CrateComponent, Money, ProductId, Role, SaleUnit};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrateListRow {
    pub id: CrateTemplateId,
    pub template: CrateTemplate,
    /// `fmtN(qty) + "cs"|"pc" + " " + item`, joined with " + ".
    pub contents: String,
    /// Sum of contents at today's rates, or why it cannot be computed.
    pub value: Result<Money, CrateTemplateError>,
}

/// One typed component row: a positive whole quantity and `Piece` or (otherwise) `Case`.
pub fn crate_component_from_text(
    product: ProductId,
    quantity: &str,
    unit: &str,
) -> Result<CrateComponent, CrateTemplateError> {
    let quantity: NonZeroU32 = quantity
        .trim()
        .parse()
        .map_err(|_| CrateTemplateError::Empty)?;
    let unit = if unit == "Piece" {
        SaleUnit::Piece
    } else {
        SaleUnit::Case
    };
    Ok(CrateComponent::new(product, quantity, unit))
}

impl ConfirmationFixture {
    /// The saved templates in creation order (DEC-034: Owner and Manager).
    pub fn crate_template_list(&self, role: Role) -> Result<Vec<CrateListRow>, CrateTemplateError> {
        crate::permissions::require(role, crate::permissions::Capability::Crates)
            .map_err(|_| CrateTemplateError::AccessDenied)?;
        Ok(self
            .crate_templates
            .saved_templates()
            .map(|(id, template)| CrateListRow {
                id,
                contents: template
                    .components
                    .iter()
                    .map(|component| {
                        let unit = match component.unit {
                            SaleUnit::Case => "cs",
                            SaleUnit::Piece => "pc",
                        };
                        let item = self
                            .catalog
                            .item_name(component.product)
                            .map_or_else(|| component.product.to_string(), str::to_owned);
                        format!("{}{unit} {item}", component.quantity)
                    })
                    .collect::<Vec<_>>()
                    .join(" + "),
                value: self.suggest_crate_price(role, &template.components),
                template: template.clone(),
            })
            .collect())
    }

    /// Looks up a template by its IPC number.
    pub fn crate_template_id(
        &self,
        role: Role,
        value: u64,
    ) -> Result<CrateTemplateId, CrateTemplateError> {
        crate::permissions::require(role, crate::permissions::Capability::Crates)
            .map_err(|_| CrateTemplateError::AccessDenied)?;
        self.crate_templates
            .saved_templates()
            .map(|(id, _)| id)
            .find(|id| id.value() == value)
            .ok_or(CrateTemplateError::NotFound)
    }

    /// New Bill's contents line under a crate row (`crateRow`): `qty + ("pc" for pieces) + " " +
    /// item`, joined with " + ", cut at 60 characters.
    pub fn crate_row_contents(&self, row: &crate::CrateRow) -> String {
        let text = row
            .components()
            .iter()
            .map(|component| {
                let unit = match component.unit {
                    SaleUnit::Piece => "pc",
                    SaleUnit::Case => "",
                };
                let item = self
                    .catalog
                    .item_name(component.product)
                    .map_or_else(|| component.product.to_string(), str::to_owned);
                format!("{}{unit} {item}", component.quantity)
            })
            .collect::<Vec<_>>()
            .join(" + ");
        text.chars().take(60).collect()
    }

    /// The saved template a crate line was loaded from: the same name, contents and rate. A
    /// template changed since the line was loaded matches no longer.
    ///
    /// # Errors
    /// Denies Managers; `NotFound` when no saved template matches.
    pub fn crate_template_of(
        &self,
        role: Role,
        row: &crate::CrateRow,
    ) -> Result<CrateTemplateId, CrateTemplateError> {
        crate::permissions::require(role, crate::permissions::Capability::Crates)
            .map_err(|_| CrateTemplateError::AccessDenied)?;
        self.crate_templates
            .saved_templates()
            .find(|(_, t)| {
                row.name() == Some(t.name.as_str())
                    && row.rate() == Some(t.price)
                    && row.components() == t.components.as_slice()
            })
            .map(|(id, _)| id)
            .ok_or(CrateTemplateError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::crate_component_from_text;
    use crate::confirmation::demo_fixture::fixture;
    use crate::confirmation::{CrateTemplate, CrateTemplateError};
    use crate::{Money, ProductId, Role};

    #[test]
    fn template_list_shows_contents_and_todays_value() {
        let mut fixture = fixture().unwrap();
        let components = vec![
            crate_component_from_text(ProductId::new(1), "2", "Case").unwrap(),
            crate_component_from_text(ProductId::new(2), " 3 ", "x").unwrap(),
        ];
        assert_eq!(
            crate_component_from_text(ProductId::new(1), "0", "Case"),
            Err(CrateTemplateError::Empty)
        );
        let id = fixture
            .save_crate_template(
                Role::Owner,
                CrateTemplate {
                    name: "Party Crate".into(),
                    components,
                    price: Money::from_minor_units(40_000),
                },
            )
            .unwrap();
        let rows = fixture.crate_template_list(Role::Owner).unwrap();
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].contents, "2cs Synthetic Lime + 3cs Synthetic Soda");
        assert_eq!(
            rows[0].value,
            Ok(Money::from_minor_units(2 * 12_050 + 3 * 6_250))
        );
        assert_eq!(fixture.crate_template_id(Role::Owner, id.value()), Ok(id));
        // DEC-034: a Manager sees the saved crates (selling rates only).
        assert_eq!(fixture.crate_template_list(Role::Manager), Ok(rows));
    }
}
