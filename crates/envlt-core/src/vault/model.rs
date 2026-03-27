use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub projects: BTreeMap<String, Project>,
}

impl VaultData {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            version: VAULT_VERSION,
            created_at: now,
            updated_at: now,
            projects: BTreeMap::new(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl Default for VaultData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub variables: BTreeMap<String, Variable>,
}

impl Project {
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

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub value: String,
    #[serde(default)]
    pub var_type: VarType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Variable {
    pub fn new(name: &str, value: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            value: value.into(),
            var_type: infer_var_type(name),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_with_type(value: impl Into<String>, var_type: VarType) -> Self {
        let now = Utc::now();
        Self {
            value: value.into(),
            var_type,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.updated_at = Utc::now();
    }

    pub fn set_type(&mut self, var_type: VarType) {
        self.var_type = var_type;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VarType {
    Secret,
    #[default]
    Config,
    Plain,
}

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
