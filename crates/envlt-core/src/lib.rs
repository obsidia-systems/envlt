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
pub use env::{parse_env_file, parse_env_str, render_env};
pub use error::{EnvltError, Result};
pub use gen::{generate_custom_value, generate_value, supported_gen_types, Charset, GenType};
pub use vault::{
    infer_var_type, ActivityAction, ActivityEvent, Project, VarType, Variable, VaultData,
    VaultStore,
};
