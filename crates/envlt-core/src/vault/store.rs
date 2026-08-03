use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use fs4::{FileExt, TryLockError};
use tempfile::NamedTempFile;

use crate::{
    error::{EnvltError, Result},
    vault::{
        crypto,
        model::{VaultData, VAULT_VERSION},
    },
};

/// Default time `VaultStore::lock` waits for another `envlt` process to
/// finish before giving up. Overridable via `ENVLT_LOCK_TIMEOUT_MS`.
const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);

fn lock_timeout() -> Duration {
    std::env::var("ENVLT_LOCK_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_LOCK_TIMEOUT)
}

/// RAII guard holding an exclusive, cross-process lock on the vault.
///
/// The lock is released when this guard is dropped, and also automatically
/// by the OS if the process crashes while holding it -- so there is no
/// stale-lock state to clean up.
#[derive(Debug)]
pub struct VaultLock {
    file: fs::File,
}

impl Drop for VaultLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

/// Manages the on-disk location, encryption, and backup of the vault file.
#[derive(Debug, Clone)]
pub struct VaultStore {
    root_dir: PathBuf,
    vault_path: PathBuf,
    backup_path: PathBuf,
}

impl VaultStore {
    /// Create a new `VaultStore` rooted at the given directory.
    pub fn new(root_dir: PathBuf) -> Self {
        let vault_path = root_dir.join("vault.age");
        let backup_path = root_dir.join("vault.age.bak");
        Self {
            root_dir,
            vault_path,
            backup_path,
        }
    }

    /// Create a `VaultStore` from `ENVLT_HOME` or the default `~/.envlt` path.
    pub fn from_env() -> Result<Self> {
        if let Some(root) = std::env::var_os("ENVLT_HOME") {
            return Ok(Self::new(PathBuf::from(root)));
        }

        let home = dirs::home_dir().ok_or(EnvltError::MissingHomeDirectory)?;
        Ok(Self::new(home.join(".envlt")))
    }

    /// Path to the envlt home directory.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Path to the encrypted vault file (`vault.age`).
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Path to the automatic backup file (`vault.age.bak`).
    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }

    /// Whether the vault file already exists on disk.
    pub fn exists(&self) -> bool {
        self.vault_path.exists()
    }

    /// Acquire an exclusive, cross-process lock on the vault.
    ///
    /// Callers performing a read-modify-write sequence (load, mutate,
    /// save) should hold this lock for the whole sequence so that two
    /// concurrent `envlt` processes cannot race on `vault.age` and silently
    /// discard each other's changes. Blocks up to [`LOCK_TIMEOUT`] while
    /// another process holds the lock before returning
    /// [`EnvltError::VaultLocked`].
    pub fn lock(&self) -> Result<VaultLock> {
        create_dir_restricted(&self.root_dir)?;
        let lock_path = self.root_dir.join("vault.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)?;

        let deadline = Instant::now() + lock_timeout();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => return Ok(VaultLock { file }),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(LOCK_POLL_INTERVAL);
                }
                Err(_) => return Err(EnvltError::VaultLocked { path: lock_path }),
            }
        }
    }

    /// Create a new empty vault and encrypt it with the given passphrase.
    pub fn initialize(&self, passphrase: &str) -> Result<()> {
        if self.exists() {
            return Err(EnvltError::VaultAlreadyExists {
                path: self.vault_path.clone(),
            });
        }

        create_dir_restricted(&self.root_dir)?;
        let vault = VaultData::new();
        self.save(&vault, passphrase)
    }

    /// Load and decrypt the vault, verifying its version.
    ///
    /// Automatically migrates vaults from version 1 to version 2 by
    /// accepting v1 on load (thanks to `serde(default)` on `activity_log`)
    /// and setting the in-memory version to 2 so the next `save()` persists
    /// the vault in the new format.
    pub fn load(&self, passphrase: &str) -> Result<VaultData> {
        if !self.exists() {
            return Err(EnvltError::VaultNotFound {
                path: self.vault_path.clone(),
            });
        }

        let ciphertext = fs::read(&self.vault_path)?;
        let plaintext = crypto::decrypt(&ciphertext, passphrase)?;
        let plaintext = String::from_utf8(plaintext).map_err(|err| EnvltError::EnvParse {
            path: self.vault_path.clone(),
            message: format!("vault content is not valid UTF-8: {err}"),
        })?;
        let mut vault: VaultData = toml::from_str(&plaintext)?;

        if vault.version == 1 {
            vault.version = VAULT_VERSION;
        }

        if vault.version != VAULT_VERSION {
            return Err(EnvltError::UnsupportedVaultVersion {
                expected: VAULT_VERSION,
                actual: vault.version,
            });
        }

        Ok(vault)
    }

    /// Encrypt and atomically save the vault, creating a backup first.
    pub fn save(&self, vault: &VaultData, passphrase: &str) -> Result<()> {
        create_dir_restricted(&self.root_dir)?;
        if self.vault_path.exists() {
            fs::copy(&self.vault_path, &self.backup_path)?;
            set_restrictive_permissions(&self.backup_path)?;
        }
        let plaintext = toml::to_string(vault)?;
        let ciphertext = crypto::encrypt(plaintext.as_bytes(), passphrase)?;

        let parent = self
            .vault_path
            .parent()
            .ok_or_else(|| EnvltError::VaultNotFound {
                path: self.vault_path.clone(),
            })?;
        let mut temp = NamedTempFile::new_in(parent)?;
        temp.write_all(&ciphertext)?;
        temp.flush()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = temp.as_file().metadata()?.permissions();
            permissions.set_mode(0o600);
            temp.as_file().set_permissions(permissions)?;
        }

        temp.as_file().sync_all()?;
        temp.persist(&self.vault_path).map_err(|err| err.error)?;
        sync_dir(parent)?;
        Ok(())
    }
}

/// Create `dir` (and its parents) restricted to the current user on Unix.
fn create_dir_restricted(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(dir)?.permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(dir, permissions)?;
    }

    Ok(())
}

/// Restrict `path` to the current user on Unix; no-op elsewhere.
#[allow(unused_variables)]
fn set_restrictive_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

/// fsync a directory so a preceding rename into it survives a crash, on Unix.
#[allow(unused_variables)]
fn sync_dir(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(dir)?.sync_all()?;
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;

    use super::*;
    use crate::vault::model::VaultData;

    #[test]
    fn initialize_restricts_home_dir_and_vault_file_permissions() {
        let home = TempDir::new().expect("tempdir");
        let root_dir = home.path().join(".envlt");
        let store = VaultStore::new(root_dir.clone());

        store.initialize("passphrase").expect("initialize");

        let dir_mode = fs::metadata(&root_dir).expect("dir metadata").permissions();
        assert_eq!(dir_mode.mode() & 0o777, 0o700);

        let file_mode = fs::metadata(store.vault_path())
            .expect("vault metadata")
            .permissions();
        assert_eq!(file_mode.mode() & 0o777, 0o600);
    }

    #[test]
    fn save_restricts_backup_file_permissions() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().join(".envlt"));
        store.initialize("passphrase").expect("initialize");

        store
            .save(&VaultData::new(), "passphrase")
            .expect("second save creates backup");

        let backup_mode = fs::metadata(store.backup_path())
            .expect("backup metadata")
            .permissions();
        assert_eq!(backup_mode.mode() & 0o777, 0o600);
    }

    #[test]
    fn lock_blocks_a_second_holder_until_the_first_is_dropped() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().join(".envlt"));
        store.initialize("passphrase").expect("initialize");

        let contender = store.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();

        let first_lock = store.lock().expect("first lock");
        let handle = std::thread::spawn(move || {
            ready_tx.send(()).expect("signal ready");
            contender.lock().expect("second lock should eventually succeed")
        });

        // Make sure the contender is actively waiting before we release the lock.
        ready_rx.recv().expect("contender signaled");
        std::thread::sleep(Duration::from_millis(100));

        drop(first_lock);

        handle.join().expect("contender thread panicked");
    }

    #[test]
    fn lock_times_out_if_never_released() {
        std::env::set_var("ENVLT_LOCK_TIMEOUT_MS", "200");
        let _guard = CleanupEnvVar("ENVLT_LOCK_TIMEOUT_MS");

        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().join(".envlt"));
        store.initialize("passphrase").expect("initialize");

        let _first_lock = store.lock().expect("first lock");

        let started = Instant::now();
        let result = store.lock();

        assert!(matches!(result, Err(EnvltError::VaultLocked { .. })));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    struct CleanupEnvVar(&'static str);

    impl Drop for CleanupEnvVar {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }
}
