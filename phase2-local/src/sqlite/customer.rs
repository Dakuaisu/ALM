use std::fmt::Write;

use crate::{DraftCustomer, DraftCustomerId, DraftStorageError};

pub(super) fn sql_text(text: &str) -> String {
    let mut result = String::from("CAST(X'");
    for byte in text.as_bytes() {
        // Writing to a String is infallible.
        let _ = write!(result, "{byte:02X}");
    }
    result.push_str("' AS TEXT)");
    result
}

fn text(hex: &str) -> Result<String, DraftStorageError> {
    let bytes = hex.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        if pair.len() != 2 {
            return Err(DraftStorageError::InvalidOutput);
        }
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        };
        let high = digit(pair[0]).ok_or(DraftStorageError::InvalidOutput)?;
        let low = digit(pair[1]).ok_or(DraftStorageError::InvalidOutput)?;
        decoded.push(high * 16 + low);
    }
    String::from_utf8(decoded).map_err(|_| DraftStorageError::InvalidOutput)
}

pub(super) fn decode(kind: &str, id: &str, name: &str) -> Result<DraftCustomer, DraftStorageError> {
    match kind {
        "0" if id.is_empty() && name.is_empty() => Ok(DraftCustomer::Unassigned),
        "1" if id.is_empty() => Ok(DraftCustomer::Typed(text(name)?)),
        "2" => Ok(DraftCustomer::Selected {
            id: DraftCustomerId::new(text(id)?),
            name: text(name)?,
        }),
        _ => Err(DraftStorageError::InvalidOutput),
    }
}
