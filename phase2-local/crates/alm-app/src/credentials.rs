//! The signed-in person's refresh token (never the password) in the platform's credential
//! store, keyed by the app ID and the account; access tokens stay in memory. macOS: Keychain;
//! Windows: Credential Manager; Android: an AES-GCM key held by the Android Keystore encrypts it
//! in app-private storage (`SecretsPlugin`). Elsewhere, and in tests, it is kept in memory only.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// The service name entries are filed under (the bundle identifier).
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) const APP_ID: &str = "in.alm.trial";

pub(crate) trait CredentialStore: Send {
    fn load(&self, account: &str) -> Result<Option<String>, String>;
    fn save(&self, account: &str, token: &str) -> Result<(), String>;
    /// Deleting an absent entry succeeds.
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// Kept in this process only; clones share the entries (a restart in tests).
#[derive(Clone, Default)]
pub(crate) struct MemoryStore(Arc<Mutex<BTreeMap<String, String>>>);

impl MemoryStore {
    #[cfg(test)]
    pub(crate) fn entries(&self) -> BTreeMap<String, String> {
        self.0.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

impl CredentialStore for MemoryStore {
    fn load(&self, account: &str) -> Result<Option<String>, String> {
        let map = self.0.lock().map_err(|_| "credential store poisoned")?;
        Ok(map.get(account).cloned())
    }

    fn save(&self, account: &str, token: &str) -> Result<(), String> {
        let mut map = self.0.lock().map_err(|_| "credential store poisoned")?;
        map.insert(account.to_owned(), token.to_owned());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        let mut map = self.0.lock().map_err(|_| "credential store poisoned")?;
        map.remove(account);
        Ok(())
    }
}

/// A store that refuses every call, as the macOS Keychain does with no default keychain.
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct UnavailableStore;

#[cfg(test)]
impl CredentialStore for UnavailableStore {
    fn load(&self, _: &str) -> Result<Option<String>, String> {
        Err("no default keychain".into())
    }

    fn save(&self, _: &str, _: &str) -> Result<(), String> {
        Err("no default keychain".into())
    }

    fn delete(&self, _: &str) -> Result<(), String> {
        Err("no default keychain".into())
    }
}

/// macOS Keychain or Windows Credential Manager (generic password, service = `service`).
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) struct PlatformStore {
    service: String,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl PlatformStore {
    pub(crate) fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    fn entry(&self, account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.service, account).map_err(|e| e.to_string())
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl CredentialStore for PlatformStore {
    fn load(&self, account: &str) -> Result<Option<String>, String> {
        match self.entry(account)?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save(&self, account: &str, token: &str) -> Result<(), String> {
        self.entry(account)?
            .set_password(token)
            .map_err(|e| e.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// The app's `SecretsPlugin` (MainActivity's package): Android Keystore AES-GCM.
#[cfg(target_os = "android")]
pub(crate) struct PlatformStore(pub(crate) tauri::plugin::PluginHandle<tauri::Wry>);

#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct Loaded {
    token: Option<String>,
}

#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct Done {}

#[cfg(target_os = "android")]
impl CredentialStore for PlatformStore {
    fn load(&self, account: &str) -> Result<Option<String>, String> {
        self.0
            .run_mobile_plugin::<Loaded>("load", BTreeMap::from([("account", account)]))
            .map(|loaded| loaded.token)
            .map_err(|e| e.to_string())
    }

    fn save(&self, account: &str, token: &str) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<Done>(
                "save",
                BTreeMap::from([("account", account), ("token", token)]),
            )
            .map(drop)
            .map_err(|e| e.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        self.0
            .run_mobile_plugin::<Done>("delete", BTreeMap::from([("account", account)]))
            .map(drop)
            .map_err(|e| e.to_string())
    }
}

/// Registers `SecretsPlugin`; its handle is managed as [`PlatformStore`]'s source.
#[cfg(target_os = "android")]
pub(crate) fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri::Manager;
    tauri::plugin::Builder::new("almsecrets")
        .setup(|app, api| {
            let handle = api.register_android_plugin("in.alm.trial", "SecretsPlugin")?;
            app.manage(SecretsHandle(handle));
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
pub(crate) struct SecretsHandle(pub(crate) tauri::plugin::PluginHandle<tauri::Wry>);

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod platform_tests {
    use super::{CredentialStore, PlatformStore};

    /// The real Keychain / Credential Manager under a test-only service name.
    #[test]
    fn the_platform_store_saves_loads_replaces_and_deletes_a_refresh_token() {
        let store = PlatformStore::new("in.alm.trial.test");
        let account = format!("test-{}", std::process::id());
        store.delete(&account).unwrap();
        assert_eq!(store.load(&account).unwrap(), None);
        store.save(&account, "synthetic-refresh-1").unwrap();
        assert_eq!(
            store.load(&account).unwrap().as_deref(),
            Some("synthetic-refresh-1")
        );
        store.save(&account, "synthetic-refresh-2").unwrap();
        assert_eq!(
            store.load(&account).unwrap().as_deref(),
            Some("synthetic-refresh-2")
        );
        let other = PlatformStore::new("in.alm.trial.test.other");
        assert_eq!(other.load(&account).unwrap(), None, "keyed by app ID");
        store.delete(&account).unwrap();
        assert_eq!(store.load(&account).unwrap(), None);
        store.delete(&account).unwrap();
    }
}
