use std::{fs, path::Path};

use serde::Deserialize;

use crate::error::{EnvltError, Result};

/// Default number of activity-log entries kept per project.
pub const DEFAULT_HISTORY_LIMIT: usize = 20;
/// Default time `VaultStore::lock` waits for another `envlt` process.
pub const DEFAULT_LOCK_TIMEOUT_MS: u64 = 5000;

/// Resolved `envlt` configuration, after applying precedence: environment
/// variable, then `config.toml`, then the built-in default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Number of activity-log entries kept per project.
    pub history_limit: usize,
    /// How long `VaultStore::lock` waits for another process, in milliseconds.
    pub lock_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            history_limit: DEFAULT_HISTORY_LIMIT,
            lock_timeout_ms: DEFAULT_LOCK_TIMEOUT_MS,
        }
    }
}

/// On-disk shape of `config.toml`. Every field is optional so an empty or
/// partial file is valid; anything left unset falls back to the default.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    history_limit: Option<usize>,
    lock_timeout_ms: Option<u64>,
}

impl Config {
    /// Load configuration for the `envlt` home directory `root_dir`.
    ///
    /// `root_dir/config.toml` is optional; a missing file is treated the
    /// same as an empty one. `ENVLT_HISTORY_LIMIT` and
    /// `ENVLT_LOCK_TIMEOUT_MS`, when set, override the corresponding
    /// `config.toml` value.
    pub fn load(root_dir: &Path) -> Result<Config> {
        let config_path = root_dir.join("config.toml");
        let raw: RawConfig = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            toml::from_str(&content).map_err(|err| EnvltError::ConfigParse {
                path: config_path.clone(),
                message: err.to_string(),
            })?
        } else {
            RawConfig::default()
        };

        let history_limit = match std::env::var("ENVLT_HISTORY_LIMIT") {
            Ok(value) => value.parse().map_err(|_| EnvltError::InvalidConfigValue {
                key: "ENVLT_HISTORY_LIMIT".to_owned(),
                value,
            })?,
            Err(_) => raw.history_limit.unwrap_or(DEFAULT_HISTORY_LIMIT),
        };

        let lock_timeout_ms = match std::env::var("ENVLT_LOCK_TIMEOUT_MS") {
            Ok(value) => value.parse().map_err(|_| EnvltError::InvalidConfigValue {
                key: "ENVLT_LOCK_TIMEOUT_MS".to_owned(),
                value,
            })?,
            Err(_) => raw.lock_timeout_ms.unwrap_or(DEFAULT_LOCK_TIMEOUT_MS),
        };

        Ok(Config {
            history_limit,
            lock_timeout_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::test_support::ENV_VAR_LOCK;

    struct CleanupEnvVar(&'static str);

    impl Drop for CleanupEnvVar {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }

    #[test]
    fn load_uses_defaults_when_no_config_file_or_env_vars_exist() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("tempdir");
        let config = Config::load(temp.path()).expect("load");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn load_reads_values_from_config_toml() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("tempdir");
        fs::write(
            temp.path().join("config.toml"),
            "history_limit = 5\nlock_timeout_ms = 1000\n",
        )
        .expect("write config");

        let config = Config::load(temp.path()).expect("load");
        assert_eq!(config.history_limit, 5);
        assert_eq!(config.lock_timeout_ms, 1000);
    }

    #[test]
    fn load_ignores_a_missing_config_file() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("tempdir");
        let config = Config::load(temp.path()).expect("load");
        assert_eq!(config.history_limit, DEFAULT_HISTORY_LIMIT);
    }

    #[test]
    fn load_partial_config_falls_back_to_defaults_for_missing_fields() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("tempdir");
        fs::write(temp.path().join("config.toml"), "history_limit = 7\n").expect("write config");

        let config = Config::load(temp.path()).expect("load");
        assert_eq!(config.history_limit, 7);
        assert_eq!(config.lock_timeout_ms, DEFAULT_LOCK_TIMEOUT_MS);
    }

    #[test]
    fn env_var_overrides_config_toml() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("tempdir");
        fs::write(temp.path().join("config.toml"), "history_limit = 5\n").expect("write config");

        std::env::set_var("ENVLT_HISTORY_LIMIT", "99");
        let _guard = CleanupEnvVar("ENVLT_HISTORY_LIMIT");

        let config = Config::load(temp.path()).expect("load");
        assert_eq!(config.history_limit, 99);
    }

    #[test]
    fn invalid_env_var_value_is_a_clear_error() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("ENVLT_HISTORY_LIMIT", "not-a-number");
        let _guard = CleanupEnvVar("ENVLT_HISTORY_LIMIT");

        let temp = TempDir::new().expect("tempdir");
        let error = Config::load(temp.path()).expect_err("invalid value");
        assert!(matches!(
            error,
            EnvltError::InvalidConfigValue { key, .. } if key == "ENVLT_HISTORY_LIMIT"
        ));
    }

    #[test]
    fn invalid_config_toml_is_a_clear_error() {
        let _env_lock = ENV_VAR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("tempdir");
        fs::write(
            temp.path().join("config.toml"),
            "history_limit = \"not a number\"\n",
        )
        .expect("write config");

        let error = Config::load(temp.path()).expect_err("invalid config");
        assert!(matches!(error, EnvltError::ConfigParse { .. }));
    }
}
