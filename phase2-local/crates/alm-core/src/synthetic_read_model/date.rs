use serde::{Deserialize, Deserializer, Serializer};

use crate::BillDate;

pub fn serialize<S: Serializer>(date: &BillDate, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(date)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BillDate, D::Error> {
    let value = String::deserialize(deserializer)?;
    value.parse().map_err(serde::de::Error::custom)
}

/// An optional `YYYY-MM-DD` date (absent when `None`).
pub mod optional {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::BillDate;

    #[allow(clippy::ref_option)]
    pub fn serialize<S: Serializer>(
        date: &Option<BillDate>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match date {
            Some(date) => serializer.collect_str(date),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<BillDate>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|value| value.parse().map_err(serde::de::Error::custom))
            .transpose()
    }
}
