use std::{
    env,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use keyring::Entry;
use keyring::Error as KeyringError;

use crate::{
    error::{EnvltError, Result},
    vault::VaultStore,
};

const KEYRING_SERVICE_PREFIX: &str = "envlt-";
const KEYRING_ACCOUNT: &str = "vault";

/// Authentication state reported by `auth_status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthStatus {
    /// Whether `ENVLT_PASSPHRASE` is set in the environment.
    pub env_var_present: bool,
    /// Whether a passphrase is stored in the system keyring.
    pub keyring_available: bool,
    /// Keyring service target derived from the vault store path.
    pub keyring_target: String,
}

/// Load a passphrase from the system keyring if one is stored.
pub fn load_stored_passphrase(store: &VaultStore) -> Result<Option<String>> {
    load_with_backend(&system_backend(), store)
}

/// Save a passphrase to the system keyring after verifying it can be read back.
pub fn save_stored_passphrase(store: &VaultStore, passphrase: &str) -> Result<()> {
    save_with_backend(&system_backend(), store, passphrase)
}

/// Remove the stored passphrase from the system keyring.
pub fn clear_stored_passphrase(store: &VaultStore) -> Result<bool> {
    clear_with_backend(&system_backend(), store)
}

fn load_with_backend(backend: &dyn KeyringBackend, store: &VaultStore) -> Result<Option<String>> {
    match backend.get_password(store) {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => match backend.get_password_legacy(store) {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        },
        Err(error) => Err(map_keyring_error(error)),
    }
}

fn save_with_backend(
    backend: &dyn KeyringBackend,
    store: &VaultStore,
    passphrase: &str,
) -> Result<()> {
    backend
        .set_password(store, passphrase)
        .map_err(map_keyring_error)?;

    match backend.get_password(store) {
        Ok(saved_passphrase) if saved_passphrase == passphrase => Ok(()),
        Ok(_) => Err(EnvltError::Keyring {
            message: "keyring write verification failed: stored value mismatch".to_owned(),
        }),
        Err(error) => Err(map_keyring_error(error)),
    }
}

fn clear_with_backend(backend: &dyn KeyringBackend, store: &VaultStore) -> Result<bool> {
    let primary_removed = match backend.delete_password(store) {
        Ok(()) => true,
        Err(KeyringError::NoEntry) => false,
        Err(error) => return Err(map_keyring_error(error)),
    };

    let legacy_removed = match backend.delete_password_legacy(store) {
        Ok(()) => true,
        Err(KeyringError::NoEntry) => false,
        Err(error) => return Err(map_keyring_error(error)),
    };

    Ok(primary_removed || legacy_removed)
}

/// Report whether environment or keyring authentication sources are available.
pub fn auth_status(store: &VaultStore) -> Result<AuthStatus> {
    let env_var_present = env::var_os("ENVLT_PASSPHRASE").is_some();
    let keyring_target = keyring_target(store)?;
    let keyring_available = load_stored_passphrase(store)?.is_some();

    Ok(AuthStatus {
        env_var_present,
        keyring_available,
        keyring_target,
    })
}

fn map_keyring_error(error: KeyringError) -> EnvltError {
    EnvltError::Keyring {
        message: error.to_string(),
    }
}

fn keyring_target(store: &VaultStore) -> Result<String> {
    let absolute = resolve_absolute_path(store.root_dir())?;

    Ok(path_to_string(&absolute))
}

fn resolve_absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };

    // Canonicalize when possible so equivalent paths map to one keyring target.
    match absolute.canonicalize() {
        Ok(path) => Ok(path),
        Err(_) => Ok(absolute),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

trait KeyringBackend {
    fn get_password(&self, store: &VaultStore) -> std::result::Result<String, KeyringError>;
    fn get_password_legacy(&self, store: &VaultStore) -> std::result::Result<String, KeyringError>;
    fn set_password(
        &self,
        store: &VaultStore,
        passphrase: &str,
    ) -> std::result::Result<(), KeyringError>;
    fn delete_password(&self, store: &VaultStore) -> std::result::Result<(), KeyringError>;
    fn delete_password_legacy(&self, store: &VaultStore) -> std::result::Result<(), KeyringError>;
}

struct SystemKeyring;

impl KeyringBackend for SystemKeyring {
    fn get_password(&self, store: &VaultStore) -> std::result::Result<String, KeyringError> {
        let entry = entry_for_store(store).map_err(to_platform_failure)?;
        entry.get_password()
    }

    fn get_password_legacy(&self, store: &VaultStore) -> std::result::Result<String, KeyringError> {
        let entry = legacy_entry_for_store(store).map_err(to_platform_failure)?;
        entry.get_password()
    }

    fn set_password(
        &self,
        store: &VaultStore,
        passphrase: &str,
    ) -> std::result::Result<(), KeyringError> {
        let entry = entry_for_store(store).map_err(to_platform_failure)?;
        entry.set_password(passphrase)
    }

    fn delete_password(&self, store: &VaultStore) -> std::result::Result<(), KeyringError> {
        let entry = entry_for_store(store).map_err(to_platform_failure)?;
        entry.delete_credential()
    }

    fn delete_password_legacy(&self, store: &VaultStore) -> std::result::Result<(), KeyringError> {
        let entry = legacy_entry_for_store(store).map_err(to_platform_failure)?;
        entry.delete_credential()
    }
}

fn system_backend() -> SystemKeyring {
    SystemKeyring
}

fn to_platform_failure(error: EnvltError) -> KeyringError {
    match error {
        EnvltError::Keyring { message } => KeyringError::PlatformFailure(message.into()),
        other => KeyringError::PlatformFailure(other.to_string().into()),
    }
}

fn entry_for_store(store: &VaultStore) -> Result<Entry> {
    let service = keyring_service(store)?;
    Entry::new(&service, KEYRING_ACCOUNT).map_err(map_keyring_error)
}

fn legacy_entry_for_store(store: &VaultStore) -> Result<Entry> {
    let target = keyring_target(store)?;
    Entry::new("envlt", &target).map_err(map_keyring_error)
}

fn keyring_service(store: &VaultStore) -> Result<String> {
    let target = keyring_target(store)?;
    Ok(format!(
        "{KEYRING_SERVICE_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(target.as_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Mutex};

    use tempfile::TempDir;

    use super::*;

    #[derive(Default)]
    struct FakeBackend {
        entries: Mutex<HashMap<String, String>>,
    }

    impl FakeBackend {
        fn key(&self, store: &VaultStore) -> String {
            keyring_service(store).expect("service")
        }
    }

    impl KeyringBackend for FakeBackend {
        fn get_password(&self, store: &VaultStore) -> std::result::Result<String, KeyringError> {
            self.entries
                .lock()
                .expect("lock")
                .get(&self.key(store))
                .cloned()
                .ok_or(KeyringError::NoEntry)
        }

        fn get_password_legacy(
            &self,
            _store: &VaultStore,
        ) -> std::result::Result<String, KeyringError> {
            Err(KeyringError::NoEntry)
        }

        fn set_password(
            &self,
            store: &VaultStore,
            passphrase: &str,
        ) -> std::result::Result<(), KeyringError> {
            self.entries
                .lock()
                .expect("lock")
                .insert(self.key(store), passphrase.to_owned());
            Ok(())
        }

        fn delete_password(&self, store: &VaultStore) -> std::result::Result<(), KeyringError> {
            self.entries
                .lock()
                .expect("lock")
                .remove(&self.key(store))
                .map(|_| ())
                .ok_or(KeyringError::NoEntry)
        }

        fn delete_password_legacy(
            &self,
            _store: &VaultStore,
        ) -> std::result::Result<(), KeyringError> {
            Err(KeyringError::NoEntry)
        }
    }

    #[test]
    fn keyring_target_uses_absolute_store_path() {
        let temp = TempDir::new().expect("tempdir");
        let store = VaultStore::new(temp.path().join(".envlt"));

        let target = keyring_target(&store).expect("target");

        assert!(target.contains(".envlt"));
        assert!(PathBuf::from(&target).is_absolute());
    }

    #[test]
    fn relative_store_path_is_normalized() {
        let temp = TempDir::new().expect("tempdir");
        std::fs::create_dir_all(temp.path().join(".envlt")).expect("create .envlt dir");

        let store = VaultStore::new(temp.path().join(".envlt/../.envlt"));
        let target = keyring_target(&store).expect("target");

        assert!(PathBuf::from(&target).is_absolute());
        assert!(!target.contains(".."));
    }

    #[test]
    fn fake_backend_roundtrip_works() {
        let temp = TempDir::new().expect("tempdir");
        let store = VaultStore::new(temp.path().join(".envlt"));
        let backend = FakeBackend::default();

        assert_eq!(
            super::load_with_backend(&backend, &store).expect("load"),
            None
        );

        super::save_with_backend(&backend, &store, "secret-passphrase").expect("save");
        assert_eq!(
            super::load_with_backend(&backend, &store).expect("load"),
            Some("secret-passphrase".to_owned())
        );

        assert!(super::clear_with_backend(&backend, &store).expect("clear"));
        assert_eq!(
            super::load_with_backend(&backend, &store).expect("load"),
            None
        );
        assert!(!super::clear_with_backend(&backend, &store).expect("clear again"));
    }

    /// Exercises the real OS keychain (not `FakeBackend`), using a
    /// throwaway, uniquely-named `VaultStore` path so it can't collide with
    /// or disturb a real vault's stored credential. Ignored by default
    /// since CI sandboxes may not have a usable OS keychain; run explicitly
    /// with `cargo test -- --ignored` on a machine that does.
    #[test]
    #[ignore = "touches the real OS keychain; run explicitly on a supported machine"]
    fn system_keyring_roundtrip_works_against_the_real_os_backend() {
        let temp = TempDir::new().expect("tempdir");
        let store = VaultStore::new(
            temp.path()
                .join(format!("envlt-keyring-smoke-test-{}", std::process::id())),
        );

        // Make sure we start from a clean slate, then guarantee cleanup even
        // if an assertion below fails.
        let _ = super::clear_stored_passphrase(&store);
        struct CleanupKeychainEntry(VaultStore);
        impl Drop for CleanupKeychainEntry {
            fn drop(&mut self) {
                let _ = super::clear_stored_passphrase(&self.0);
            }
        }
        let _cleanup = CleanupKeychainEntry(store.clone());

        assert_eq!(
            super::load_stored_passphrase(&store).expect("load before save"),
            None
        );

        super::save_stored_passphrase(&store, "real-keychain-smoke-test-passphrase")
            .expect("save to real keychain");
        assert_eq!(
            super::load_stored_passphrase(&store).expect("load after save"),
            Some("real-keychain-smoke-test-passphrase".to_owned())
        );

        assert!(super::clear_stored_passphrase(&store).expect("clear"));
        assert_eq!(
            super::load_stored_passphrase(&store).expect("load after clear"),
            None
        );
    }
}
