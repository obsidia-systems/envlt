use std::path::PathBuf;

use thiserror::Error;

/// Convenience type alias for `Result<T, EnvltError>`.
pub type Result<T> = std::result::Result<T, EnvltError>;

/// The error type used throughout `envlt-core`.
#[derive(Debug, Error)]
pub enum EnvltError {
    /// Could not resolve the envlt home directory.
    #[error("could not resolve the envlt home directory")]
    MissingHomeDirectory,
    /// Vault file not found at the expected path.
    #[error("vault not found at {path}")]
    VaultNotFound {
        /// Expected vault path.
        path: PathBuf,
    },
    /// Attempted to initialize a vault that already exists.
    #[error("vault already exists at {path}")]
    VaultAlreadyExists {
        /// Existing vault path.
        path: PathBuf,
    },
    /// Vault version does not match the supported version.
    #[error("unsupported vault version: expected {expected}, got {actual}")]
    UnsupportedVaultVersion {
        /// Supported version.
        expected: u32,
        /// Version found in the file.
        actual: u32,
    },
    /// Requested project was not found in the vault.
    #[error("project '{name}' was not found")]
    ProjectNotFound {
        /// Missing project name.
        name: String,
    },
    /// Attempted to create a project that already exists.
    #[error("project '{name}' already exists")]
    ProjectAlreadyExists {
        /// Existing project name.
        name: String,
    },
    /// Requested variable was not found in the project.
    #[error("variable '{key}' was not found in project '{project}'")]
    VariableNotFound {
        /// Project containing the variable.
        project: String,
        /// Missing variable key.
        key: String,
    },
    /// Could not resolve a project from CLI args or `.envlt-link`.
    #[error("could not resolve a project from --project or .envlt-link in {path}")]
    ProjectResolutionFailed {
        /// Directory where resolution was attempted.
        path: PathBuf,
    },
    /// Variable assignment is not in `KEY=VALUE` format.
    #[error("invalid variable assignment '{input}', expected KEY=VALUE")]
    InvalidAssignment {
        /// Raw input that failed parsing.
        input: String,
    },
    /// Failed to parse an `.env` file.
    #[error("failed to parse env file at {path}: {message}")]
    EnvParse {
        /// File path.
        path: PathBuf,
        /// Parse error message.
        message: String,
    },
    /// A required example variable was left empty without an override.
    #[error("missing value for example variable '{key}'")]
    MissingExampleValue {
        /// Variable key.
        key: String,
    },
    /// Failed to parse a `.envlt-link` file.
    #[error("failed to parse project link at {path}: {message}")]
    LinkParse {
        /// File path.
        path: PathBuf,
        /// Parse error message.
        message: String,
    },
    /// Bundle magic bytes do not match `ENVL`.
    #[error("bundle magic is invalid")]
    InvalidBundleMagic,
    /// Bundle version is not supported.
    #[error("unsupported bundle version: expected {expected}, got {actual}")]
    UnsupportedBundleVersion {
        /// Supported version.
        expected: u8,
        /// Version found in the bundle.
        actual: u8,
    },
    /// Bundle data is truncated or structurally invalid.
    #[error("bundle is truncated or malformed")]
    InvalidBundleLayout,
    /// Bundle header exceeds the maximum allowed size.
    #[error("bundle header is too large: {length} bytes")]
    BundleHeaderTooLarge {
        /// Header length in bytes.
        length: usize,
    },
    /// Failed to serialize the bundle header to JSON.
    #[error("failed to encode bundle header: {0}")]
    BundleHeaderSerialize(#[from] serde_json::Error),
    /// Decryption of the bundle payload failed (likely wrong passphrase).
    #[error("bundle payload decryption failed")]
    BundleDecryptFailed,
    /// Bundle payload is invalid or corrupted.
    #[error("bundle payload is invalid")]
    InvalidBundlePayload,
    /// Bundle KDF parameters are invalid.
    #[error("bundle KDF parameters are invalid")]
    InvalidBundleKdf,
    /// Import would overwrite an existing project without permission.
    #[error("bundle project '{name}' already exists")]
    BundleProjectAlreadyExists {
        /// Existing project name.
        name: String,
    },
    /// Generator type string is not recognized.
    #[error("unsupported generator type '{gen_type}'")]
    UnsupportedGenType {
        /// Unrecognized generator type.
        gen_type: String,
    },
    /// Requested generator length is invalid.
    #[error("invalid generator length: {length}")]
    InvalidGenLength {
        /// Requested length.
        length: usize,
    },
    /// System keyring operation failed.
    #[error("keyring error: {message}")]
    Keyring {
        /// Error message from the keyring backend.
        message: String,
    },
    /// Vault passphrase is incorrect or the vault is corrupted.
    #[error("vault passphrase is invalid or the vault is corrupted")]
    InvalidPassphrase,
    /// No command was provided for `envlt run`.
    #[error("missing command to run")]
    MissingCommand,
    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML serialization error.
    #[error("serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    /// TOML deserialization error.
    #[error("deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    /// `age` encryption error.
    #[error("encryption error: {0}")]
    AgeEncrypt(#[source] age::EncryptError),
    /// `age` decryption error.
    #[error("decryption error: {0}")]
    AgeDecrypt(#[source] age::DecryptError),
}
