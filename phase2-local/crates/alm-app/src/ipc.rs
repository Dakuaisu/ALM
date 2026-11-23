use serde::{Deserialize, Deserializer, de};

/// An unsigned 64-bit ID carried as decimal text to avoid JavaScript rounding.
pub(crate) struct IpcId(u64);

impl IpcId {
    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for IpcId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let digits = String::deserialize(deserializer)?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(de::Error::custom("expected an unsigned decimal ID"));
        }
        digits.parse().map(Self).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::IpcId;
    use tauri::ipc::InvokeResponseBody;

    #[test]
    fn decimal_id_strings_preserve_all_64_bits() {
        for (json, expected) in [
            (r#""0""#, 0),
            (r#""9007199254740993""#, 9_007_199_254_740_993),
            (r#""18446744073709551615""#, u64::MAX),
        ] {
            let id = InvokeResponseBody::Json(json.into())
                .deserialize::<IpcId>()
                .unwrap();
            assert_eq!(id.value(), expected);
        }
    }

    #[test]
    fn invalid_id_payloads_are_rejected() {
        for json in [
            "1",
            "9007199254740993",
            "null",
            r#""""#,
            r#""-1""#,
            r#""+1""#,
            r#""1.0""#,
            r#""1e3""#,
            r#"" 1 ""#,
            r#""abc""#,
            r#""18446744073709551616""#,
        ] {
            assert!(
                InvokeResponseBody::Json(json.into())
                    .deserialize::<IpcId>()
                    .is_err(),
                "unexpectedly accepted {json}"
            );
        }
    }
}
