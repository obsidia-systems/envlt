use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use fs4::{FileExt, TryLockError};
use tempfile::NamedTempFile;
use zeroize::Zeroizing;

use crate::{
    error::{EnvltError, Result},
    vault::{
        crypto, migration,
        migration::MIN_SUPPORTED_VAULT_VERSION,
        model::{VaultData, VAULT_VERSION},
    },
};

/// Default time `VaultStore::lock` waits for another `envlt` process to
/// finish before giving up. Overridable via `ENVLT_LOCK_TIMEOUT_MS`.
const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Number of additional numbered backups kept beyond `vault.age.bak`, so a
/// write that corrupts the most recent backup doesn't destroy every prior
/// good copy.
const BACKUP_GENERATIONS: u32 = 2;

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

    /// Load and decrypt the vault, migrating an older on-disk format to the
    /// current version if needed. See [`Self::load_with_migration_info`] if
    /// the caller needs to know whether a migration actually happened.
    pub fn load(&self, passphrase: &str) -> Result<VaultData> {
        self.load_with_migration_info(passphrase)
            .map(|(vault, _migrated_from)| vault)
    }

    /// Load and decrypt the vault like [`Self::load`], additionally
    /// returning the on-disk version it was migrated from, or `None` if it
    /// was already at [`VAULT_VERSION`].
    ///
    /// A vault older than [`MIN_SUPPORTED_VAULT_VERSION`] or newer than
    /// [`VAULT_VERSION`] is rejected with [`EnvltError::UnsupportedVaultVersion`].
    /// When a migration is applied, the pre-migration ciphertext is written
    /// to `vault.v{old}.pre-migration.age` before the in-memory data is
    /// upgraded, so the original file is always recoverable.
    pub fn load_with_migration_info(&self, passphrase: &str) -> Result<(VaultData, Option<u32>)> {
        if !self.exists() {
            return Err(EnvltError::VaultNotFound {
                path: self.vault_path.clone(),
            });
        }

        let ciphertext = fs::read(&self.vault_path)?;
        let plaintext_bytes = crypto::decrypt(&ciphertext, passphrase)?;
        let plaintext =
            std::str::from_utf8(&plaintext_bytes).map_err(|err| EnvltError::EnvParse {
                path: self.vault_path.clone(),
                message: format!("vault content is not valid UTF-8: {err}"),
            })?;

        let mut table: toml::value::Table = toml::from_str(plaintext)?;
        let stored_version = table
            .get("version")
            .and_then(toml::Value::as_integer)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0);

        if !(MIN_SUPPORTED_VAULT_VERSION..=VAULT_VERSION).contains(&stored_version) {
            return Err(EnvltError::UnsupportedVaultVersion {
                expected: VAULT_VERSION,
                actual: stored_version,
            });
        }

        let migrated_from = if stored_version < VAULT_VERSION {
            self.write_pre_migration_backup(&ciphertext, stored_version)?;
            migration::migrate(&mut table, stored_version)?;
            Some(stored_version)
        } else {
            None
        };

        let migrated_toml = Zeroizing::new(toml::to_string(&table)?);
        let vault: VaultData = toml::from_str(&migrated_toml)?;
        Ok((vault, migrated_from))
    }

    /// Preserve the exact pre-migration ciphertext under a version-stamped
    /// name, distinct from the regular `vault.age.bak` rotation, so a
    /// migration can always be undone by restoring this file.
    fn write_pre_migration_backup(&self, ciphertext: &[u8], from_version: u32) -> Result<()> {
        let backup_path = self
            .root_dir
            .join(format!("vault.v{from_version}.pre-migration.age"));
        fs::write(&backup_path, ciphertext)?;
        set_restrictive_permissions(&backup_path)?;
        Ok(())
    }

    /// Encrypt and atomically save the vault, rotating backups first.
    pub fn save(&self, vault: &VaultData, passphrase: &str) -> Result<()> {
        create_dir_restricted(&self.root_dir)?;
        if self.vault_path.exists() {
            self.rotate_backups()?;
            fs::copy(&self.vault_path, &self.backup_path)?;
            set_restrictive_permissions(&self.backup_path)?;
        }
        let plaintext = Zeroizing::new(toml::to_string(vault)?);
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

    /// Path to a numbered, older backup generation (`vault.age.bak.{generation}`).
    ///
    /// `generation` 1 is the backup one save older than `vault.age.bak`, up
    /// to [`BACKUP_GENERATIONS`].
    pub fn numbered_backup_path(&self, generation: u32) -> PathBuf {
        self.root_dir.join(format!("vault.age.bak.{generation}"))
    }

    /// Shift `vault.age.bak` -> `vault.age.bak.1` -> ... -> `vault.age.bak.{BACKUP_GENERATIONS}`,
    /// dropping whatever previously occupied the oldest generation, so a
    /// single corrupted write can't destroy every prior good backup.
    fn rotate_backups(&self) -> Result<()> {
        for generation in (1..=BACKUP_GENERATIONS).rev() {
            let from = if generation == 1 {
                self.backup_path.clone()
            } else {
                self.numbered_backup_path(generation - 1)
            };
            let to = self.numbered_backup_path(generation);

            if from.exists() {
                fs::rename(&from, &to)?;
            }
        }
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
    use crate::vault::model::{Project, VaultData};

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
    fn save_rotates_backups_and_drops_oldest_beyond_retention() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().join(".envlt"));
        store.initialize("passphrase").expect("initialize");

        let vault_with_project = |name: &str| {
            let mut vault = VaultData::new();
            vault
                .projects
                .insert(name.to_owned(), Project::new(name, None));
            vault
        };

        for name in ["state-1", "state-2", "state-3", "state-4"] {
            store
                .save(&vault_with_project(name), "passphrase")
                .expect("save");
        }

        let sole_project_in = |path: &Path| -> String {
            let ciphertext = fs::read(path).expect("read backup");
            let plaintext = crypto::decrypt(&ciphertext, "passphrase").expect("decrypt backup");
            let vault: VaultData =
                toml::from_str(std::str::from_utf8(&plaintext).expect("utf8")).expect("parse");
            vault.projects.keys().next().cloned().expect("one project")
        };

        assert_eq!(sole_project_in(store.backup_path()), "state-3");
        assert_eq!(sole_project_in(&store.numbered_backup_path(1)), "state-2");
        assert_eq!(sole_project_in(&store.numbered_backup_path(2)), "state-1");
        assert!(!store.numbered_backup_path(3).exists());
    }

    #[test]
    fn load_migrates_v1_and_keeps_a_pre_migration_backup() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        fs::create_dir_all(store.root_dir()).expect("mkdir");

        let v1_toml = r#"
version = 1
created_at = "2024-01-01T00:00:00Z"
updated_at = "2024-01-01T00:00:00Z"

[projects.demo]
name = "demo"
created_at = "2024-01-01T00:00:00Z"
updated_at = "2024-01-01T00:00:00Z"

[projects.demo.variables]
"#;
        let original_ciphertext =
            crypto::encrypt(v1_toml.as_bytes(), "passphrase").expect("encrypt");
        fs::write(store.vault_path(), &original_ciphertext).expect("write v1 vault");

        let (vault, migrated_from) = store
            .load_with_migration_info("passphrase")
            .expect("load and migrate");

        assert_eq!(migrated_from, Some(1));
        assert_eq!(vault.version, VAULT_VERSION);
        assert!(vault.projects["demo"].activity_log.is_empty());

        let backup_path = store.root_dir().join("vault.v1.pre-migration.age");
        let backed_up = fs::read(&backup_path).expect("pre-migration backup exists");
        assert_eq!(backed_up, original_ciphertext);
    }

    #[test]
    fn load_with_migration_info_reports_none_for_a_current_vault() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().join(".envlt"));
        store.initialize("passphrase").expect("initialize");

        let (_vault, migrated_from) = store.load_with_migration_info("passphrase").expect("load");

        assert_eq!(migrated_from, None);
    }

    #[test]
    fn load_rejects_a_version_newer_than_current() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        fs::create_dir_all(store.root_dir()).expect("mkdir");

        let future_toml = format!("version = {}\n", VAULT_VERSION + 1);
        let ciphertext = crypto::encrypt(future_toml.as_bytes(), "passphrase").expect("encrypt");
        fs::write(store.vault_path(), ciphertext).expect("write future vault");

        let error = store
            .load("passphrase")
            .expect_err("future version rejected");
        assert!(matches!(
            error,
            EnvltError::UnsupportedVaultVersion {
                expected: VAULT_VERSION,
                actual,
            } if actual == VAULT_VERSION + 1
        ));
    }

    #[test]
    fn load_rejects_a_version_older_than_min_supported() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        fs::create_dir_all(store.root_dir()).expect("mkdir");

        let too_old_toml = "version = 0\n";
        let ciphertext = crypto::encrypt(too_old_toml.as_bytes(), "passphrase").expect("encrypt");
        fs::write(store.vault_path(), ciphertext).expect("write too-old vault");

        let error = store
            .load("passphrase")
            .expect_err("too-old version rejected");
        assert!(matches!(
            error,
            EnvltError::UnsupportedVaultVersion {
                expected: VAULT_VERSION,
                actual: 0,
            }
        ));
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
            contender
                .lock()
                .expect("second lock should eventually succeed")
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
