#![deny(missing_docs)]

//! Core library for envlt — a local-first encrypted environment vault.
//!
//! This crate provides vault storage, encryption, environment variable parsing,
//! and project management primitives. The CLI binary (`envlt-cli`) wraps these
//! operations in a user-friendly command-line interface.

/// Application service and diagnostic types.
pub mod app;
/// Keyring-backed passphrase storage.
pub mod auth;
/// Encrypted project bundle format.
pub mod bundle;
/// Persistent user configuration (`config.toml`).
pub mod config;
/// Environment file parser and renderer.
pub mod env;
/// Error types used throughout the crate.
pub mod error;
/// Secure value generators.
pub mod gen;
/// Project link file helpers.
pub mod link;
/// Vault storage, encryption, and data models.
pub mod vault;

pub use app::{
    AppService, DiagnosticCheck, DiagnosticSeverity, DoctorReport, ExampleDiff, ProjectDiff,
    RemoveProjectResult, RunEnvironment, VariableView,
};
pub use auth::{
    auth_status, clear_stored_passphrase, load_stored_passphrase, save_stored_passphrase,
    AuthStatus,
};
pub use config::Config;
pub use env::{parse_env_file, parse_env_str, render_env};
pub use error::{EnvltError, Result};
pub use gen::{generate_custom_value, generate_value, supported_gen_types, Charset, GenType};
pub use vault::{
    infer_var_type, synthesize_variable_events, ActivityAction, ActivityEvent, Environment,
    Project, VarType, Variable, VariableVersion, VaultData, VaultStore, DEFAULT_ENVIRONMENT,
};

/// Test-only helpers shared across the crate's test modules.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    /// `Config::load` reads `ENVLT_HISTORY_LIMIT`/`ENVLT_LOCK_TIMEOUT_MS`
    /// from the process environment, which is global state shared by every
    /// test in this binary. Any test that sets or relies on the absence of
    /// these variables must hold this lock for its duration, or it can
    /// observe another such test's env var mid-mutation (tests run
    /// concurrently by default).
    pub(crate) static ENV_VAR_LOCK: Mutex<()> = Mutex::new(());
}
