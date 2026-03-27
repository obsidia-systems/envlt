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

#[derive(Debug, Clone)]
pub struct VaultStore {
    root_dir: PathBuf,
    vault_path: PathBuf,
    backup_path: PathBuf,
}

impl VaultStore {
    pub fn new(root_dir: PathBuf) -> Self {
        let vault_path = root_dir.join("vault.age");
        let backup_path = root_dir.join("vault.age.bak");
        Self {
            root_dir,
            vault_path,
            backup_path,
        }
    }

    pub fn from_env() -> Result<Self> {
        if let Some(root) = std::env::var_os("ENVLT_HOME") {
            return Ok(Self::new(PathBuf::from(root)));
        }

        let home = dirs::home_dir().ok_or(EnvltError::MissingHomeDirectory)?;
        Ok(Self::new(home.join(".envlt")))
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }

    pub fn exists(&self) -> bool {
        self.vault_path.exists()
    }

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
