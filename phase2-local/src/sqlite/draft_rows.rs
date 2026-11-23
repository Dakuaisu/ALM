use std::num::NonZeroU32;

use rusqlite::Row;

use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, DraftStorageError, ProductId,
    ProvisionalDraft, crate_row_codec,
};

use super::{integer, text};

#[derive(Eq, PartialEq)]
struct Metadata {
    date: Option<String>,
    kind: Option<i64>,
    customer_id: Option<String>,
    name: Option<String>,
    definition: Option<String>,
}

pub(super) struct DraftRow {
    metadata: Metadata,
    position: Option<i64>,
    product: Option<String>,
    quantity: Option<i64>,
}

impl DraftRow {
    pub(super) fn read(row: &Row<'_>) -> Result<Self, DraftStorageError> {
        Ok(Self {
            metadata: Metadata {
                date: text(row, 0)?,
                kind: integer(row, 1)?,
                customer_id: text(row, 2)?,
                name: text(row, 3)?,
                definition: text(row, 4)?,
            },
            position: integer(row, 5)?,
            product: text(row, 6)?,
            quantity: integer(row, 7)?,
        })
    }
}

fn customer(metadata: Metadata) -> Result<DraftCustomer, DraftStorageError> {
    match (metadata.kind, metadata.customer_id, metadata.name) {
        (Some(0), None, None) => Ok(DraftCustomer::Unassigned),
        (Some(1), None, Some(name)) => Ok(DraftCustomer::Typed(name)),
        (Some(2), Some(id), Some(name)) => Ok(DraftCustomer::Selected {
            id: DraftCustomerId::new(id),
            name,
        }),
        _ => Err(DraftStorageError::InvalidOutput),
    }
}

pub(super) fn decode(
    id: DraftId,
    rows: Vec<DraftRow>,
) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
    use DraftStorageError::InvalidOutput;
    let mut lines = Vec::new();
    let mut metadata = None;
    for (position, row) in rows.into_iter().enumerate() {
        if metadata
            .replace(row.metadata)
            .is_some_and(|previous| Some(&previous) != metadata.as_ref())
        {
            return Err(InvalidOutput);
        }
        let (index, product, quantity) = match (row.position, row.product, row.quantity) {
            (None, None, None) if position == 0 => continue,
            (Some(index), Some(product), Some(quantity)) => (index, product, quantity),
            _ => return Err(InvalidOutput),
        };
        if usize::try_from(index) != Ok(position) {
            return Err(InvalidOutput);
        }
        let product = product.parse().map_err(|_| InvalidOutput)?;
        let quantity = u32::try_from(quantity)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(InvalidOutput)?;
        lines.push(DraftLine::new(ProductId::new(product), quantity));
    }
    let Some(metadata) = metadata else {
        return Ok(None);
    };
    let bill_date = match &metadata.date {
        None => None,
        Some(value) => Some(value.parse::<BillDate>().map_err(|_| InvalidOutput)?),
    };
    let definition = metadata.definition.clone();
    let mut draft = ProvisionalDraft::new(id, lines)
        .with_customer(customer(metadata)?)
        .with_bill_date(bill_date);
    if let Some(definition) = definition {
        draft
            .add_crate_row(crate_row_codec::decode(&definition)?)
            .map_err(|_| InvalidOutput)?;
    }
    Ok(Some(draft))
}
