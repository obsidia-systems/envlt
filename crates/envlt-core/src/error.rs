use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, EnvltError>;

#[derive(Debug, Error)]
pub enum EnvltError {
    #[error("could not resolve the envlt home directory")]
    MissingHomeDirectory,
    #[error("vault not found at {path}")]
    VaultNotFound { path: PathBuf },
    #[error("vault already exists at {path}")]
    VaultAlreadyExists { path: PathBuf },
    #[error("unsupported vault version: expected {expected}, got {actual}")]
    UnsupportedVaultVersion { expected: u32, actual: u32 },
    #[error("project '{name}' was not found")]
    ProjectNotFound { name: String },
    #[error("project '{name}' already exists")]
    ProjectAlreadyExists { name: String },
    #[error("variable '{key}' was not found in project '{project}'")]
    VariableNotFound { project: String, key: String },
    #[error("could not resolve a project from --project or .envlt-link in {path}")]
    ProjectResolutionFailed { path: PathBuf },
    #[error("invalid variable assignment '{input}', expected KEY=VALUE")]
    InvalidAssignment { input: String },
    #[error("failed to parse env file at {path}: {message}")]
    EnvParse { path: PathBuf, message: String },
    #[error("missing value for example variable '{key}'")]
    MissingExampleValue { key: String },
    #[error("failed to parse project link at {path}: {message}")]
    LinkParse { path: PathBuf, message: String },
    #[error("bundle magic is invalid")]
    InvalidBundleMagic,
    #[error("unsupported bundle version: expected {expected}, got {actual}")]
    UnsupportedBundleVersion { expected: u8, actual: u8 },
    #[error("bundle is truncated or malformed")]
    InvalidBundleLayout,
    #[error("bundle header is too large: {length} bytes")]
    BundleHeaderTooLarge { length: usize },
    #[error("failed to encode bundle header: {0}")]
    BundleHeaderSerialize(#[from] serde_json::Error),
    #[error("bundle payload decryption failed")]
    BundleDecryptFailed,
    #[error("bundle payload is invalid")]
    InvalidBundlePayload,
    #[error("bundle KDF parameters are invalid")]
    InvalidBundleKdf,
    #[error("bundle project '{name}' already exists")]
    BundleProjectAlreadyExists { name: String },
    #[error("unsupported generator type '{gen_type}'")]
    UnsupportedGenType { gen_type: String },
    #[error("invalid generator length: {length}")]
    InvalidGenLength { length: usize },
    #[error("keyring error: {message}")]
    Keyring { message: String },
    #[error("vault passphrase is invalid or the vault is corrupted")]
    InvalidPassphrase,
    #[error("missing command to run")]
    MissingCommand,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("encryption error: {0}")]
    AgeEncrypt(#[source] age::EncryptError),
    #[error("decryption error: {0}")]
    AgeDecrypt(#[source] age::DecryptError),
}
