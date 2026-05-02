use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use tempfile::NamedTempFile;

use crate::{
    error::{EnvltError, Result},
    vault::{
        crypto,
        model::{VaultData, VAULT_VERSION},
    },
};

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

    /// Create a new empty vault and encrypt it with the given passphrase.
    pub fn initialize(&self, passphrase: &str) -> Result<()> {
        if self.exists() {
            return Err(EnvltError::VaultAlreadyExists {
                path: self.vault_path.clone(),
            });
        }

        fs::create_dir_all(&self.root_dir)?;
        let vault = VaultData::new();
        self.save(&vault, passphrase)
    }

    /// Load and decrypt the vault, verifying its version.
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
        let vault: VaultData = toml::from_str(&plaintext)?;

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
        fs::create_dir_all(&self.root_dir)?;
        if self.vault_path.exists() {
            fs::copy(&self.vault_path, &self.backup_path)?;
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
        temp.persist(&self.vault_path).map_err(|err| err.error)?;
        Ok(())
    }
}
