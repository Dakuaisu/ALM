use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::IgnoredAny};
use serde_json::Number;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub(super) enum PriceSupplierEntry {
    Name(String),
    Object(PriceSupplierObject),
    Array(Vec<Self>),
    Number(Number),
    Boolean(bool),
    Null,
}

impl From<&str> for PriceSupplierEntry {
    fn from(name: &str) -> Self {
        Self::Name(name.into())
    }
}

impl PriceSupplierEntry {
    pub fn is_name(&self, name: &str) -> bool {
        matches!(self, Self::Name(value) if value == name)
    }

    pub fn has_id(&self, id: &str) -> bool {
        matches!(self, Self::Object(object) if object.0.get("id").is_some_and(|value| value.is_name(id)))
    }

    pub fn header(&self) -> String {
        match self {
            Self::Array(values) => values.iter().map(Self::string_value).collect::<String>(),
            value => value.string_value(),
        }
    }

    fn string_value(&self) -> String {
        match self {
            Self::Name(name) => name.clone(),
            Self::Object(_) => "[object Object]".into(),
            Self::Array(values) => values
                .iter()
                .map(Self::string_value)
                .collect::<Vec<_>>()
                .join(","),
            Self::Number(number) => number.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Null => String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(super) struct PriceSupplierObject(BTreeMap<String, PriceSupplierEntry>);

impl PriceSupplierObject {
    pub fn version(&self) -> u64 {
        match self.0.get("_v") {
            Some(PriceSupplierEntry::Number(value)) => value.as_u64().unwrap_or_default(),
            _ => 0,
        }
    }
}

pub(super) fn load_price_suppliers<'de, D: Deserializer<'de>>(
    input: D,
) -> Result<Vec<PriceSupplierEntry>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Field {
        Array(Vec<PriceSupplierEntry>),
        Other(IgnoredAny),
    }
    Ok(match Field::deserialize(input)? {
        Field::Array(entries) => entries,
        Field::Other(_) => Vec::new(),
    })
}
