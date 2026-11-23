use super::{DraftStorageError, Money, Review, hex, unhex};
use crate::confirmation::{CheckoutTerms, LineTerms, ProductTerms, SaleUnit};

pub(super) fn encode(base: &str, terms: &CheckoutTerms) -> String {
    let lines = terms
        .lines
        .iter()
        .map(|line| {
            format!(
                "{},{},{},{}",
                match line.unit {
                    SaleUnit::Piece => 0,
                    SaleUnit::Case => 1,
                },
                line.product.pack,
                line.product.piece_price.minor_units(),
                line.product.case_price.minor_units()
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "4|{}|{}|{}",
        hex(base),
        terms.loading_rate.minor_units(),
        lines
    )
}

pub(super) fn decode(snapshot: &str) -> Result<Review, DraftStorageError> {
    let invalid = || DraftStorageError::InvalidOutput;
    let fields: Vec<_> = snapshot.split('|').collect();
    let [_, base, loading, lines] = fields.as_slice() else {
        return Err(invalid());
    };
    let decoded_base = unhex(base)?;
    if !decoded_base.starts_with("3|") {
        return Err(invalid());
    }
    let mut review = super::decode(&decoded_base)?;
    if matches!(review.payment, crate::confirmation::ProposedPayment::Legacy) {
        return Err(invalid());
    }
    let loading_rate = Money::from_minor_units(loading.parse().map_err(|_| invalid())?);
    if loading_rate.minor_units() < 0 {
        return Err(invalid());
    }
    let lines = lines
        .split(';')
        .map(|line| {
            let fields: Vec<_> = line.split(',').collect();
            let [unit, pack, piece, case] = fields.as_slice() else {
                return Err(invalid());
            };
            let unit = match *unit {
                "0" => SaleUnit::Piece,
                "1" => SaleUnit::Case,
                _ => return Err(invalid()),
            };
            let product = ProductTerms {
                pack: pack.parse().map_err(|_| invalid())?,
                piece_price: Money::from_minor_units(piece.parse().map_err(|_| invalid())?),
                case_price: Money::from_minor_units(case.parse().map_err(|_| invalid())?),
            };
            if product.piece_price.minor_units() < 0 || product.case_price.minor_units() < 0 {
                return Err(invalid());
            }
            Ok(LineTerms { unit, product })
        })
        .collect::<Result<Vec<_>, DraftStorageError>>()?;
    if lines.len() != review.draft.lines().len() {
        return Err(invalid());
    }
    review.checkout = Some(CheckoutTerms {
        lines,
        loading_rate,
    });
    review.amounts().map_err(|_| invalid())?;
    Ok(review)
}
