use serde::{Deserialize, Deserializer, Serializer};

use crate::BillDate;

pub fn serialize<S: Serializer>(date: &BillDate, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(date)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BillDate, D::Error> {
    let value = String::deserialize(deserializer)?;
    value.parse().map_err(serde::de::Error::custom)
}
