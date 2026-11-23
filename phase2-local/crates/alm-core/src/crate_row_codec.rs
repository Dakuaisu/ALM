use std::fmt::Write;

use crate::{CrateComponent, CrateRow, InvalidStoredText, Money, ProductId, SaleUnit};

pub fn encode(row: &CrateRow) -> String {
    let mut definition = row.quantity().to_string();
    if let (Some(name), Some(rate)) = (row.name(), row.rate()) {
        let _ = write!(definition, ",{},", rate.minor_units());
        // Three decimal digits per UTF-8 byte keep the existing SQLite alphabet.
        for byte in name.bytes() {
            let _ = write!(definition, "{byte:03}");
        }
    }
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

pub fn decode(definition: &str) -> Result<CrateRow, InvalidStoredText> {
    let invalid = || InvalidStoredText;
    let mut parts = definition.split(';');
    let mut header = parts.next().ok_or_else(invalid)?.split(',');
    let quantity = header
        .next()
        .ok_or_else(invalid)?
        .parse()
        .map_err(|_| invalid())?;
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
    let row = CrateRow::new(quantity, components);
    match (header.next(), header.next(), header.next()) {
        (None, None, None) => Ok(row),
        (Some(rate), Some(name), None) if rate.bytes().all(|b| b.is_ascii_digit()) => {
            let rate = rate.parse::<i64>().map_err(|_| invalid())?;
            Ok(row.with_price(decode_name(name)?, Money::from_minor_units(rate)))
        }
        _ => Err(invalid()),
    }
}

fn decode_name(name: &str) -> Result<String, InvalidStoredText> {
    let invalid = || InvalidStoredText;
    let (triples, remaining) = name.as_bytes().as_chunks::<3>();
    if name.is_empty() || !remaining.is_empty() || !name.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let bytes = triples
        .iter()
        .map(|digits| {
            std::str::from_utf8(digits)
                .map_err(|_| invalid())?
                .parse::<u8>()
                .map_err(|_| invalid())
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_ad_hoc_definition_keeps_the_same_bytes() {
        let definition = "2;2,6,0;1,3,1";
        let row = decode(definition).unwrap();
        assert_eq!(row.name(), None);
        assert_eq!(row.rate(), None);
        assert_eq!(encode(&row), definition);
    }

    #[test]
    fn copied_definition_rejects_invalid_rate_and_name() {
        for invalid in ["2,-1,065", "2,1,", "2,1,65", "2,1,256", "2,1,255"] {
            assert!(matches!(decode(invalid), Err(InvalidStoredText)));
        }
    }
}
