use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current vault format version.
pub const VAULT_VERSION: u32 = 1;
const SECRET_HINTS: [&str; 9] = [
    "KEY",
    "SECRET",
    "PASSWORD",
    "PASS",
    "TOKEN",
    "CREDENTIAL",
    "PRIVATE",
    "API_KEY",
    "AUTH",
];

/// Top-level encrypted vault containing all projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    /// Format version of this vault.
    pub version: u32,
    /// UTC timestamp when the vault was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
    /// Map of project names to their data.
    pub projects: BTreeMap<String, Project>,
}

impl VaultData {
    /// Create a new empty vault with the current version and timestamp.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            version: VAULT_VERSION,
            created_at: now,
            updated_at: now,
            projects: BTreeMap::new(),
        }
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl Default for VaultData {
    fn default() -> Self {
        Self::new()
    }
}

/// A named collection of environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project name (used as the lookup key in the vault).
    pub name: String,
    /// Optional filesystem path associated with the project.
    pub path: Option<PathBuf>,
    /// UTC timestamp when the project was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
    /// Sorted map of variable keys to their values and metadata.
    pub variables: BTreeMap<String, Variable>,
}

impl Project {
    /// Create a new project with the given name and optional path.
    pub fn new(name: impl Into<String>, path: Option<PathBuf>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            path,
            created_at: now,
            updated_at: now,
            variables: BTreeMap::new(),
        }
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// A single environment variable with type metadata and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// The variable value.
    pub value: String,
    /// Classification of the variable (secret, config, or plain).
    #[serde(default)]
    pub var_type: VarType,
    /// UTC timestamp when the variable was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
}

impl Variable {
    /// Create a new variable with a type inferred from `name`.
    pub fn new(name: &str, value: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            value: value.into(),
            var_type: infer_var_type(name),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new variable with an explicit type.
    pub fn new_with_type(value: impl Into<String>, var_type: VarType) -> Self {
        let now = Utc::now();
        Self {
            value: value.into(),
            var_type,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the value and bump the `updated_at` timestamp.
    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.updated_at = Utc::now();
    }

    /// Update the type and bump the `updated_at` timestamp.
    pub fn set_type(&mut self, var_type: VarType) {
        self.var_type = var_type;
        self.updated_at = Utc::now();
    }
}

/// Classification for a variable based on sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VarType {
    /// Sensitive value that should be masked in output.
    Secret,
    /// Non-sensitive configuration value.
    #[default]
    Config,
    /// Explicitly marked as non-sensitive.
    Plain,
}

/// Infer the variable type from common naming conventions.
pub fn infer_var_type(name: &str) -> VarType {
    let uppercase_name = name.to_ascii_uppercase();
    if SECRET_HINTS
        .iter()
        .any(|hint| uppercase_name.contains(hint))
    {
        VarType::Secret
    } else {
        VarType::Config
    }
}

#[cfg(test)]
mod tests {
    use super::{infer_var_type, VarType, Variable};

    #[test]
    fn infers_secret_type_from_sensitive_key_names() {
        assert_eq!(infer_var_type("API_KEY"), VarType::Secret);
        assert_eq!(infer_var_type("db_password"), VarType::Secret);
        assert_eq!(infer_var_type("auth_token"), VarType::Secret);
    }

    #[test]
    fn infers_config_type_when_name_is_not_sensitive() {
        assert_eq!(infer_var_type("PORT"), VarType::Config);
        assert_eq!(infer_var_type("APP_ENV"), VarType::Config);
    }

    #[test]
    fn new_variable_uses_inferred_type() {
        let variable = Variable::new("JWT_SECRET", "top-secret");
        assert_eq!(variable.var_type, VarType::Secret);
    }

    #[test]
    fn new_with_type_uses_explicit_type() {
        let variable = Variable::new_with_type("value", VarType::Plain);
        assert_eq!(variable.var_type, VarType::Plain);
    }
}
