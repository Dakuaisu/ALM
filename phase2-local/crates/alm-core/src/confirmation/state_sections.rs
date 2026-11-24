//! DEVELOPMENT SIMULATION ONLY: the Owner device projection of the simulated authority's whole
//! read state, one JSON section per fixture field, so every Owner screen renders from the
//! device projection rather than the simulation. Never built for a Manager.
use serde_json::{Map, Value};

use super::manual_cashbook::ManualCashBook;
use super::{ConfirmationError, ConfirmationFixture};
use crate::Role;

const CASHBOOK: &str = "cashbook";

/// Serializes maps whose keys JSON cannot hold as a list of pairs.
pub(crate) mod pairs {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(
        map: &BTreeMap<K, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(map.iter())
    }

    pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}

impl ConfirmationFixture {
    /// The Owner's state sections: `(section name, JSON)` in name order.
    ///
    /// # Errors
    /// Denies Managers; fails if the state does not serialize.
    pub fn state_sections(
        &self,
        role: Role,
        cashbook: &ManualCashBook,
    ) -> Result<Vec<(String, String)>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let invalid = |_| ConfirmationError::InvalidDraft;
        let Value::Object(fields) = serde_json::to_value(self).map_err(invalid)? else {
            return Err(ConfirmationError::InvalidDraft);
        };
        let mut sections: Vec<(String, String)> = fields
            .into_iter()
            .map(|(name, value)| Ok((name, serde_json::to_string(&value).map_err(invalid)?)))
            .collect::<Result<_, ConfirmationError>>()?;
        sections.push((
            CASHBOOK.into(),
            serde_json::to_string(cashbook).map_err(invalid)?,
        ));
        sections.sort();
        Ok(sections)
    }

    /// Rebuilds a read-only copy of the state from stored sections.
    ///
    /// # Errors
    /// Rejects missing, unknown or malformed sections.
    pub fn from_state_sections(
        sections: &[(String, String)],
    ) -> Result<(Self, ManualCashBook), ConfirmationError> {
        let invalid = |_| ConfirmationError::InvalidDraft;
        let mut fields = Map::new();
        let mut cashbook = None;
        for (name, body) in sections {
            let value: Value = serde_json::from_str(body).map_err(invalid)?;
            if name == CASHBOOK {
                cashbook = Some(serde_json::from_value(value).map_err(invalid)?);
            } else {
                fields.insert(name.clone(), value);
            }
        }
        let fixture = serde_json::from_value(Value::Object(fields)).map_err(invalid)?;
        Ok((fixture, cashbook.ok_or(ConfirmationError::InvalidDraft)?))
    }
}
