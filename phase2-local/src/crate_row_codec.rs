use std::fmt::Write;

use crate::{CrateComponent, CrateRow, DraftStorageError, ProductId, SaleUnit};

pub(crate) fn encode(row: &CrateRow) -> String {
    let mut definition = row.quantity().to_string();
    for component in row.components() {
        let unit = match component.unit {
            SaleUnit::Piece => 0,
            SaleUnit::Case => 1,
        };
        let _ = write!(
            definition,
            ";{},{},{unit}",
            component.product.value(),
            component.quantity
        );
    }
    definition
}

pub(crate) fn decode(definition: &str) -> Result<CrateRow, DraftStorageError> {
    let invalid = || DraftStorageError::InvalidOutput;
    let mut parts = definition.split(';');
    let quantity = parts
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(invalid)?;
    let components = parts
        .map(|component| {
            let mut fields = component.split(',');
            let (Some(product), Some(quantity), Some(unit), None) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                return Err(invalid());
            };
            let unit = match unit {
                "0" => SaleUnit::Piece,
                "1" => SaleUnit::Case,
                _ => return Err(invalid()),
            };
            Ok(CrateComponent::new(
                ProductId::new(product.parse().map_err(|_| invalid())?),
                quantity.parse().map_err(|_| invalid())?,
                unit,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CrateRow::new(quantity, components))
}
