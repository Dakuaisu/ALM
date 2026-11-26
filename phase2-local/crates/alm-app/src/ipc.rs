use serde::{Deserialize, Deserializer, de};

pub(crate) struct State<'a, T> {
    inner: &'a T,
    access: crate::session_policy::Access,
}

impl<'de, R: tauri::Runtime> tauri::ipc::CommandArg<'de, R> for State<'de, crate::AppState> {
    fn from_command(
        item: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        let access = crate::session_policy::classify(item.name)
            .ok_or_else(|| tauri::ipc::InvokeError::from("Unknown command policy."))?;
        let inner: tauri::State<'de, crate::AppState> = tauri::State::from_command(item)?;
        Ok(Self {
            inner: inner.inner(),
            access,
        })
    }
}

impl State<'_, crate::AppState> {
    #[cfg(test)]
    pub(crate) fn for_test<'a>(
        inner: &'a crate::AppState,
        command: &str,
    ) -> Result<State<'a, crate::AppState>, String> {
        let access = crate::session_policy::classify(command).ok_or("Unknown command policy.")?;
        Ok(State { inner, access })
    }
    pub(crate) fn lock(&self) -> Result<std::sync::MutexGuard<'_, crate::Trial>, String> {
        let trial = self
            .inner
            .lock()
            .map_err(|_| "app state poisoned".to_owned())?;
        crate::session_policy::authorize(&trial, self.access)?;
        Ok(trial)
    }
}

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

/// A draft or posted bill's ID as its text (`7`, or `<account>:<device>-<n>`).
pub(crate) struct IpcDraft(alm_core::DraftId);

impl IpcDraft {
    pub(crate) const fn value(self) -> alm_core::DraftId {
        self.0
    }
}

impl<'de> Deserialize<'de> for IpcDraft {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        alm_core::DraftId::parse(&text)
            .map(Self)
            .map_err(|_| de::Error::custom("expected a draft ID"))
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
