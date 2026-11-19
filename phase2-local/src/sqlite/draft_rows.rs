use std::num::NonZeroU32;

use crate::{
    BillDate, DraftId, DraftLine, DraftStorageError, ProductId, ProvisionalDraft, crate_row_codec,
};

use super::customer;

pub(super) fn decode(
    id: DraftId,
    output: &[u8],
) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
    use DraftStorageError::InvalidOutput;
    let output = std::str::from_utf8(output).map_err(|_| InvalidOutput)?;
    let mut lines = Vec::new();
    let mut metadata = None;
    for (position, row) in output.lines().enumerate() {
        let columns = row.split('|').collect::<Vec<_>>();
        let &[
            date,
            kind,
            customer_id,
            name,
            definition,
            index,
            product,
            quantity,
        ] = columns.as_slice()
        else {
            return Err(InvalidOutput);
        };
        let fields = (date, kind, customer_id, name, definition);
        if metadata
            .replace(fields)
            .is_some_and(|previous| previous != fields)
        {
            return Err(InvalidOutput);
        }
        if index == "NULL" && product == "NULL" && quantity == "NULL" && position == 0 {
            continue;
        }
        if index.parse::<usize>() != Ok(position) {
            return Err(InvalidOutput);
        }
        let product = product.parse().map_err(|_| InvalidOutput)?;
        let quantity = quantity.parse::<NonZeroU32>().map_err(|_| InvalidOutput)?;
        lines.push(DraftLine::new(ProductId::new(product), quantity));
    }
    let Some((date, kind, customer_id, name, definition)) = metadata else {
        return Ok(None);
    };
    let bill_date = match date {
        "NULL" => None,
        value => Some(value.parse::<BillDate>().map_err(|_| InvalidOutput)?),
    };
    let mut draft = ProvisionalDraft::new(id, lines)
        .with_customer(customer::decode(kind, customer_id, name)?)
        .with_bill_date(bill_date);
    if definition != "NULL" {
        draft
            .add_crate_row(crate_row_codec::decode(definition)?)
            .map_err(|_| InvalidOutput)?;
    }
    Ok(Some(draft))
}
