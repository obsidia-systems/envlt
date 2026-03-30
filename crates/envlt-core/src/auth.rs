use std::{env, path::Path};

use keyring::{Entry, Error as KeyringError};

use crate::{
    error::{EnvltError, Result},
    vault::VaultStore,
};

const KEYRING_SERVICE: &str = "envlt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthStatus {
    pub env_var_present: bool,
    pub keyring_available: bool,
    pub keyring_target: String,
}

pub fn load_stored_passphrase(store: &VaultStore) -> Result<Option<String>> {
    match system_backend().get_password(store) {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(map_keyring_error(error)),
    }
}

pub fn save_stored_passphrase(store: &VaultStore, passphrase: &str) -> Result<()> {
    system_backend()
        .set_password(store, passphrase)
        .map_err(map_keyring_error)
}

pub fn clear_stored_passphrase(store: &VaultStore) -> Result<bool> {
    match system_backend().delete_password(store) {
        Ok(()) => Ok(true),
        Err(KeyringError::NoEntry) => Ok(false),
        Err(error) => Err(map_keyring_error(error)),
    }
}

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
    let root = store.root_dir();
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        env::current_dir()?.join(root)
    };

    Ok(path_to_string(&absolute))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

trait KeyringBackend {
    fn get_password(&self, store: &VaultStore) -> std::result::Result<String, KeyringError>;
    fn set_password(
        &self,
        store: &VaultStore,
        passphrase: &str,
    ) -> std::result::Result<(), KeyringError>;
    fn delete_password(&self, store: &VaultStore) -> std::result::Result<(), KeyringError>;
}

struct SystemKeyring;

impl KeyringBackend for SystemKeyring {
    fn get_password(&self, store: &VaultStore) -> std::result::Result<String, KeyringError> {
        let entry = entry_for_store(store).map_err(|error| match error {
            EnvltError::Keyring { message } => KeyringError::PlatformFailure(message.into()),
            other => KeyringError::PlatformFailure(other.to_string().into()),
        })?;
        entry.get_password()
    }

    fn set_password(
        &self,
        store: &VaultStore,
        passphrase: &str,
    ) -> std::result::Result<(), KeyringError> {
        let entry = entry_for_store(store).map_err(|error| match error {
            EnvltError::Keyring { message } => KeyringError::PlatformFailure(message.into()),
            other => KeyringError::PlatformFailure(other.to_string().into()),
        })?;
        entry.set_password(passphrase)
    }

    fn delete_password(&self, store: &VaultStore) -> std::result::Result<(), KeyringError> {
        let entry = entry_for_store(store).map_err(|error| match error {
            EnvltError::Keyring { message } => KeyringError::PlatformFailure(message.into()),
            other => KeyringError::PlatformFailure(other.to_string().into()),
        })?;
        entry.delete_credential()
    }
}

fn system_backend() -> SystemKeyring {
    SystemKeyring
}

fn entry_for_store(store: &VaultStore) -> Result<Entry> {
    let target = keyring_target(store)?;
    Entry::new(KEYRING_SERVICE, &target).map_err(map_keyring_error)
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
            keyring_target(store).expect("target")
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
    }

    fn load_with_backend(
        backend: &dyn KeyringBackend,
        store: &VaultStore,
    ) -> Result<Option<String>> {
        match backend.get_password(store) {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NoEntry) => Ok(None),
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
            .map_err(map_keyring_error)
    }

    fn clear_with_backend(backend: &dyn KeyringBackend, store: &VaultStore) -> Result<bool> {
        match backend.delete_password(store) {
            Ok(()) => Ok(true),
            Err(KeyringError::NoEntry) => Ok(false),
            Err(error) => Err(map_keyring_error(error)),
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
    fn fake_backend_roundtrip_works() {
        let temp = TempDir::new().expect("tempdir");
        let store = VaultStore::new(temp.path().join(".envlt"));
        let backend = FakeBackend::default();

        assert_eq!(load_with_backend(&backend, &store).expect("load"), None);

        save_with_backend(&backend, &store, "secret-passphrase").expect("save");
        assert_eq!(
            load_with_backend(&backend, &store).expect("load"),
            Some("secret-passphrase".to_owned())
        );

        assert!(clear_with_backend(&backend, &store).expect("clear"));
        assert_eq!(load_with_backend(&backend, &store).expect("load"), None);
        assert!(!clear_with_backend(&backend, &store).expect("clear again"));
    }
}
