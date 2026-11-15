use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "windows")]
mod dpapi;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
mod vault;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::{missing_default, set_in_login};

const SERVICE: &str = "app.nicedb.desktop";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "no default keyring: create one in Seahorse (GNOME Keyring) or KWallet, then try again"
    )]
    NoKeyring,
    #[error("Windows Credential Manager is full: remove old entries, then try again")]
    Full,
    #[error("keychain: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

fn other(e: impl std::fmt::Display) -> Error {
    Error::Other(e.to_string())
}

#[cfg(target_os = "windows")]
fn full(e: &keyring::Error) -> bool {
    const ERROR_NOT_ENOUGH_MEMORY: u32 = 8;
    matches!(e, keyring::Error::PlatformFailure(inner)
        if inner.downcast_ref::<keyring::windows::Error>().is_some_and(|w| w.0 == ERROR_NOT_ENOUGH_MEMORY))
}

#[cfg(not(target_os = "windows"))]
fn full(_: &keyring::Error) -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
fn missing_default(_: &keyring::Error) -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
fn set_in_login(_: &str, _: &str, _: &str) -> Result<()> {
    Err(Error::NoKeyring)
}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

static FILE_FALLBACK: AtomicBool = AtomicBool::new(false);

pub fn allow_file_fallback(on: bool) {
    FILE_FALLBACK.store(on, Ordering::Relaxed);
}

pub struct Keychain {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    vault: PathBuf,
}

impl Keychain {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            vault: dir.into().join("secrets.dpapi.json"),
        }
    }

    #[cfg(target_os = "windows")]
    fn vault_get(&self, key: &str) -> Result<Option<String>> {
        vault::get(&self.vault, key, dpapi::unprotect)
    }

    #[cfg(not(target_os = "windows"))]
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn vault_get(&self, _: &str) -> Result<Option<String>> {
        Ok(None)
    }

    #[cfg(target_os = "windows")]
    fn vault_set(&self, key: &str, value: &str) -> Result<()> {
        if FILE_FALLBACK.load(Ordering::Relaxed) {
            vault::set(&self.vault, key, value, dpapi::protect)
        } else {
            Err(Error::Full)
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[allow(clippy::unused_self)]
    fn vault_set(&self, _: &str, _: &str) -> Result<()> {
        Err(Error::Full)
    }

    #[cfg(target_os = "windows")]
    fn vault_delete(&self, key: &str) -> Result<()> {
        vault::delete(&self.vault, key)
    }

    #[cfg(not(target_os = "windows"))]
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn vault_delete(&self, _: &str) -> Result<()> {
        Ok(())
    }

    fn entry(key: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(SERVICE, key).map_err(other)
    }
}

impl SecretStore for Keychain {
    fn get(&self, key: &str) -> Result<Option<String>> {
        match Self::entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => self.vault_get(key),
            Err(e) if missing_default(&e) => Ok(None),
            Err(e) => Err(other(e)),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        match Self::entry(key)?.set_password(value) {
            Ok(()) => self.vault_delete(key),
            Err(e) if missing_default(&e) => set_in_login(SERVICE, key, value),
            Err(e) if full(&e) => self.vault_set(key, value),
            Err(e) => Err(other(e)),
        }
    }

    fn delete(&self, key: &str) -> Result<()> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => self.vault_delete(key),
            Err(e) if missing_default(&e) => Ok(()),
            Err(e) => Err(other(e)),
        }
    }
}

#[derive(Default)]
pub struct Memory(Mutex<HashMap<String, String>>);

impl SecretStore for Memory {
    fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .0
            .lock()
            .map_err(|_| other("poisoned"))?
            .get(key)
            .cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.0
            .lock()
            .map_err(|_| other("poisoned"))?
            .insert(key.into(), value.into());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.0.lock().map_err(|_| other("poisoned"))?.remove(key);
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    #[test]
    fn full_vault_is_recognised() {
        let failure =
            |code| keyring::Error::PlatformFailure(Box::new(keyring::windows::Error(code)));
        assert!(super::full(&failure(8)));
        assert!(!super::full(&failure(5)));
    }
}
