#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{AppError, AppResult, GhErrorKind, gh};

const SERVICE: &str = "com.teams4soft.qapture";
const ACCOUNT: &str = "github";

/// Almacén del token de GitHub. El token nunca se guarda en archivos.
pub trait TokenStore: Send + Sync {
    fn get(&self) -> AppResult<Option<String>>;
    fn set(&self, token: &str) -> AppResult<()>;
    fn delete(&self) -> AppResult<()>;
}

fn keyring_error() -> AppError {
    let message = if cfg!(windows) {
        "No se pudo usar el Administrador de credenciales de Windows."
    } else {
        "No se encontró un llavero del sistema (Secret Service). Instala y desbloquea gnome-keyring o KWallet y vuelve a intentar."
    };
    gh(GhErrorKind::Keyring, message)
}

/// Llavero del sistema (Credential Manager en Windows, Secret Service en Linux).
#[derive(Default)]
pub struct KeyringStore;

impl KeyringStore {
    pub fn new() -> Self {
        Self
    }

    fn entry(&self) -> AppResult<keyring::Entry> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|_| keyring_error())
    }
}

impl TokenStore for KeyringStore {
    fn get(&self) -> AppResult<Option<String>> {
        match self.entry()?.get_password() {
            Ok(t) => Ok(Some(t)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(keyring_error()),
        }
    }

    fn set(&self, token: &str) -> AppResult<()> {
        self.entry()?
            .set_password(token)
            .map_err(|_| keyring_error())
    }

    fn delete(&self) -> AppResult<()> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(keyring_error()),
        }
    }
}

/// Almacén en memoria, para tests de este y otros módulos.
#[cfg(test)]
#[derive(Default)]
pub struct MemoryTokenStore {
    token: Mutex<Option<String>>,
    fail: AtomicBool,
}

#[cfg(test)]
impl MemoryTokenStore {
    /// Almacén cuyas operaciones devuelven siempre el error `Keyring`.
    pub fn failing() -> Self {
        Self {
            token: Mutex::new(None),
            fail: AtomicBool::new(true),
        }
    }

    fn check(&self) -> AppResult<()> {
        if self.fail.load(Ordering::SeqCst) {
            Err(keyring_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
impl TokenStore for MemoryTokenStore {
    fn get(&self) -> AppResult<Option<String>> {
        self.check()?;
        Ok(self.token.lock().unwrap().clone())
    }

    fn set(&self, token: &str) -> AppResult<()> {
        self.check()?;
        *self.token.lock().unwrap() = Some(token.to_string());
        Ok(())
    }

    fn delete(&self) -> AppResult<()> {
        self.check()?;
        *self.token.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_message() -> &'static str {
        if cfg!(windows) {
            "No se pudo usar el Administrador de credenciales de Windows."
        } else {
            "No se encontró un llavero del sistema (Secret Service). Instala y desbloquea gnome-keyring o KWallet y vuelve a intentar."
        }
    }

    #[test]
    fn memory_store_round_trip() {
        let s = MemoryTokenStore::default();
        assert_eq!(s.get().unwrap(), None);
        s.set("tok").unwrap();
        assert_eq!(s.get().unwrap().as_deref(), Some("tok"));
        s.delete().unwrap();
        assert_eq!(s.get().unwrap(), None);
    }

    #[test]
    fn failing_store_reports_keyring_error() {
        let s = MemoryTokenStore::failing();
        for e in [
            s.get().unwrap_err(),
            s.set("x").unwrap_err(),
            s.delete().unwrap_err(),
        ] {
            assert_eq!(e.code(), "gh_keyring");
            assert_eq!(e.user_message(), expected_message());
        }
    }

    #[test]
    #[ignore = "usa el llavero real del sistema"]
    fn keyring_store_round_trip() {
        let s = KeyringStore::new();
        s.delete().unwrap();
        assert_eq!(s.get().unwrap(), None);
        s.set("qapture-test-token").unwrap();
        assert_eq!(s.get().unwrap().as_deref(), Some("qapture-test-token"));
        s.delete().unwrap();
        assert_eq!(s.get().unwrap(), None);
    }
}
