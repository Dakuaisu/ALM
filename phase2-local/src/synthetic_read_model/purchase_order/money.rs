use crate::Money;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S: Serializer>(value: &Money, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(value.minor_units())
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Money, D::Error> {
    i64::deserialize(deserializer).map(Money::from_minor_units)
}

pub mod optional {
    use crate::Money;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<Money>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.map(Money::minor_units).serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Money>, D::Error> {
        Option::<i64>::deserialize(deserializer).map(|value| value.map(Money::from_minor_units))
    }
}
