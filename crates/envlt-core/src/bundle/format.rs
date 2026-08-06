use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use chrono::{DateTime, Utc};
use rand::rngs::SysRng;
use rand::TryRng;
use scrypt::{scrypt, Params};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    error::{EnvltError, Result},
    vault::Project,
};

/// Magic bytes at the start of every `.evlt` bundle.
pub const BUNDLE_MAGIC: [u8; 4] = *b"ENVL";
/// Current bundle format version.
///
/// Bumped from `1` to `2` when environments were introduced: a bundle now
/// carries exactly one environment (see [`encrypt_project_bundle`]) rather
/// than a project's flat variable map, so v1 bundles can no longer be
/// decoded and are rejected outright by [`decode_archive`].
pub const BUNDLE_VERSION: u8 = 2;
/// Length of the ChaCha20-Poly1305 nonce in bytes.
pub const BUNDLE_NONCE_LEN: usize = 12;
/// Length of the ChaCha20-Poly1305 authentication tag in bytes.
pub const BUNDLE_TAG_LEN: usize = 16;
/// Length of the KDF salt in bytes.
pub const BUNDLE_SALT_LEN: usize = 16;
const BUNDLE_KEY_LEN: usize = 32;

/// scrypt `log_n` used by bundles produced before KDF params were recorded
/// in the header. Matches `Params::recommended()` as of scrypt 0.11, so
/// older bundles keep decrypting with the exact values they were encrypted
/// with even if a future scrypt release changes its recommendation.
fn legacy_kdf_log_n() -> u8 {
    17
}

/// scrypt `r` used by bundles produced before KDF params were recorded in
/// the header. See [`legacy_kdf_log_n`].
fn legacy_kdf_r() -> u32 {
    8
}

/// scrypt `p` used by bundles produced before KDF params were recorded in
/// the header. See [`legacy_kdf_log_n`].
fn legacy_kdf_p() -> u32 {
    1
}

/// Metadata header stored inside an encrypted bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleHeader {
    /// Name of the exported project.
    pub project: String,
    /// Name of the single environment this bundle carries.
    pub environment: String,
    /// UTC timestamp when the bundle was created.
    pub exported_at: DateTime<Utc>,
    /// envlt version that produced the bundle.
    pub envlt_version: String,
    /// Base64-encoded KDF salt.
    pub kdf_salt_b64: String,
    /// scrypt `log_n` parameter used to derive the bundle key.
    #[serde(default = "legacy_kdf_log_n")]
    pub kdf_log_n: u8,
    /// scrypt `r` parameter used to derive the bundle key.
    #[serde(default = "legacy_kdf_r")]
    pub kdf_r: u32,
    /// scrypt `p` parameter used to derive the bundle key.
    #[serde(default = "legacy_kdf_p")]
    pub kdf_p: u32,
}

/// In-memory representation of an encrypted `.evlt` bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleArchive {
    /// Parsed header metadata.
    pub header: BundleHeader,
    /// Encryption nonce.
    pub nonce: [u8; BUNDLE_NONCE_LEN],
    /// Encrypted payload (project TOML).
    pub ciphertext: Vec<u8>,
    /// Authentication tag.
    pub tag: [u8; BUNDLE_TAG_LEN],
}

/// Serialize a `BundleArchive` into its on-disk byte representation.
pub fn encode_archive(archive: &BundleArchive) -> Result<Vec<u8>> {
    let header_json = serde_json::to_vec(&archive.header)?;
    let header_len: u16 =
        header_json
            .len()
            .try_into()
            .map_err(|_| EnvltError::BundleHeaderTooLarge {
                length: header_json.len(),
            })?;

    let mut output = Vec::with_capacity(
        4 + 1
            + 2
            + header_json.len()
            + BUNDLE_NONCE_LEN
            + archive.ciphertext.len()
            + BUNDLE_TAG_LEN,
    );
    output.extend_from_slice(&BUNDLE_MAGIC);
    output.push(BUNDLE_VERSION);
    output.extend_from_slice(&header_len.to_be_bytes());
    output.extend_from_slice(&header_json);
    output.extend_from_slice(&archive.nonce);
    output.extend_from_slice(&archive.ciphertext);
    output.extend_from_slice(&archive.tag);
    Ok(output)
}

/// Deserialize a `BundleArchive` from raw bytes.
pub fn decode_archive(bytes: &[u8]) -> Result<BundleArchive> {
    if bytes.len() < 4 + 1 + 2 + BUNDLE_NONCE_LEN + BUNDLE_TAG_LEN {
        return Err(EnvltError::InvalidBundleLayout);
    }

    if bytes[0..4] != BUNDLE_MAGIC {
        return Err(EnvltError::InvalidBundleMagic);
    }

    let version = bytes[4];
    if version != BUNDLE_VERSION {
        return Err(EnvltError::UnsupportedBundleVersion {
            expected: BUNDLE_VERSION,
            actual: version,
        });
    }

    let header_len = u16::from_be_bytes([bytes[5], bytes[6]]) as usize;
    let header_start = 7;
    let header_end = header_start + header_len;
    let nonce_end = header_end + BUNDLE_NONCE_LEN;

    if bytes.len() < nonce_end + BUNDLE_TAG_LEN {
        return Err(EnvltError::InvalidBundleLayout);
    }

    let payload_len = bytes.len() - nonce_end - BUNDLE_TAG_LEN;
    let tag_start = nonce_end + payload_len;

    let header: BundleHeader = serde_json::from_slice(&bytes[header_start..header_end])?;

    let mut nonce = [0_u8; BUNDLE_NONCE_LEN];
    nonce.copy_from_slice(&bytes[header_end..nonce_end]);

    let ciphertext = bytes[nonce_end..tag_start].to_vec();

    let mut tag = [0_u8; BUNDLE_TAG_LEN];
    tag.copy_from_slice(&bytes[tag_start..]);

    Ok(BundleArchive {
        header,
        nonce,
        ciphertext,
        tag,
    })
}

/// Encrypt a project into a portable `.evlt` bundle.
///
/// `project` must contain exactly the single environment named
/// `environment_name` -- callers (see `AppService::export_project_bundle`)
/// build a "shadow" project shaped this way rather than exporting a
/// project's full multi-environment state, so that sharing one
/// environment's bundle can never leak another's.
pub fn encrypt_project_bundle(
    project: &Project,
    environment_name: &str,
    bundle_passphrase: &str,
    envlt_version: &str,
) -> Result<Vec<u8>> {
    let mut nonce = [0_u8; BUNDLE_NONCE_LEN];
    let mut salt = [0_u8; BUNDLE_SALT_LEN];
    SysRng
        .try_fill_bytes(&mut nonce)
        .expect("failed to read system randomness");
    SysRng
        .try_fill_bytes(&mut salt)
        .expect("failed to read system randomness");

    let kdf_params = Params::RECOMMENDED;
    let header = BundleHeader {
        project: project.name.clone(),
        environment: environment_name.to_owned(),
        exported_at: Utc::now(),
        envlt_version: envlt_version.to_owned(),
        kdf_salt_b64: STANDARD_NO_PAD.encode(salt),
        kdf_log_n: kdf_params.log_n(),
        kdf_r: kdf_params.r(),
        kdf_p: kdf_params.p(),
    };

    let plaintext = Zeroizing::new(toml::to_string(project)?);
    let key = derive_key(bundle_passphrase, &salt, &kdf_params)?;
    let cipher = ChaCha20Poly1305::new(&Key::from(*key));
    let mut ciphertext = cipher
        .encrypt(&Nonce::from(nonce), plaintext.as_bytes())
        .map_err(|_| EnvltError::BundleDecryptFailed)?;

    let tag_start = ciphertext
        .len()
        .checked_sub(BUNDLE_TAG_LEN)
        .ok_or(EnvltError::InvalidBundlePayload)?;
    let tag_slice = ciphertext.split_off(tag_start);

    let mut tag = [0_u8; BUNDLE_TAG_LEN];
    tag.copy_from_slice(&tag_slice);

    encode_archive(&BundleArchive {
        header,
        nonce,
        ciphertext,
        tag,
    })
}

/// Decrypt a `.evlt` bundle and restore the project it contains.
pub fn decrypt_project_bundle(bundle_bytes: &[u8], bundle_passphrase: &str) -> Result<Project> {
    let archive = decode_archive(bundle_bytes)?;
    let salt = STANDARD_NO_PAD
        .decode(&archive.header.kdf_salt_b64)
        .map_err(|_| EnvltError::InvalidBundlePayload)?;
    let kdf_params = Params::new(
        archive.header.kdf_log_n,
        archive.header.kdf_r,
        archive.header.kdf_p,
    )
    .map_err(|_| EnvltError::InvalidBundleKdf)?;
    let key = derive_key(bundle_passphrase, &salt, &kdf_params)?;
    let cipher = ChaCha20Poly1305::new(&Key::from(*key));

    let mut payload = archive.ciphertext;
    payload.extend_from_slice(&archive.tag);

    let plaintext = Zeroizing::new(
        cipher
            .decrypt(&Nonce::from(archive.nonce), payload.as_ref())
            .map_err(|_| EnvltError::BundleDecryptFailed)?,
    );
    let plaintext =
        std::str::from_utf8(&plaintext).map_err(|_| EnvltError::InvalidBundlePayload)?;
    let project: Project = toml::from_str(plaintext)?;

    if project.name != archive.header.project {
        return Err(EnvltError::InvalidBundlePayload);
    }
    if !project
        .environments
        .contains_key(&archive.header.environment)
    {
        return Err(EnvltError::InvalidBundlePayload);
    }

    Ok(project)
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    params: &Params,
) -> Result<Zeroizing<[u8; BUNDLE_KEY_LEN]>> {
    let mut key = Zeroizing::new([0_u8; BUNDLE_KEY_LEN]);
    scrypt(passphrase.as_bytes(), salt, params, &mut key[..])
        .map_err(|_| EnvltError::InvalidBundleKdf)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_archive, decrypt_project_bundle, encode_archive, encrypt_project_bundle,
        BundleArchive, BundleHeader, BUNDLE_MAGIC,
    };
    use crate::error::EnvltError;
    use crate::vault::{Environment, Project, Variable};
    use chrono::Utc;

    #[test]
    fn bundle_archive_roundtrip_preserves_fields() {
        let archive = BundleArchive {
            header: BundleHeader {
                project: "api-payments".to_owned(),
                environment: "local".to_owned(),
                exported_at: Utc::now(),
                envlt_version: "0.1.0".to_owned(),
                kdf_salt_b64: "salt".to_owned(),
                kdf_log_n: 17,
                kdf_r: 8,
                kdf_p: 1,
            },
            nonce: [7_u8; 12],
            ciphertext: vec![1, 2, 3, 4, 5],
            tag: [9_u8; 16],
        };

        let encoded = encode_archive(&archive).expect("encode");
        let decoded = decode_archive(&encoded).expect("decode");

        assert_eq!(decoded, archive);
    }

    #[test]
    fn decode_rejects_invalid_magic() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"NOPE");
        bytes.push(2);
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&[0_u8; 12]);
        bytes.extend_from_slice(&[0_u8; 16]);

        let error = decode_archive(&bytes).expect_err("invalid magic");
        assert!(matches!(error, EnvltError::InvalidBundleMagic));
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&BUNDLE_MAGIC);
        bytes.push(99);
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&[0_u8; 12]);
        bytes.extend_from_slice(&[0_u8; 16]);

        let error = decode_archive(&bytes).expect_err("invalid version");
        assert!(matches!(
            error,
            EnvltError::UnsupportedBundleVersion {
                expected: 2,
                actual: 99
            }
        ));
    }

    #[test]
    fn decode_rejects_a_pre_environments_v1_bundle() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&BUNDLE_MAGIC);
        bytes.push(1);
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&[0_u8; 12]);
        bytes.extend_from_slice(&[0_u8; 16]);

        let error = decode_archive(&bytes).expect_err("v1 bundle rejected");
        assert!(matches!(
            error,
            EnvltError::UnsupportedBundleVersion {
                expected: 2,
                actual: 1
            }
        ));
    }

    #[test]
    fn encrypted_bundle_roundtrip_restores_project() {
        let mut project = Project::new("api-auth", None);
        let mut environment = Environment::new("local");
        environment.variables.insert(
            "JWT_SECRET".to_owned(),
            Variable::new("JWT_SECRET", "super-secret"),
        );
        project.environments.insert("local".to_owned(), environment);

        let bundle = encrypt_project_bundle(&project, "local", "bundle-password", "0.1.0")
            .expect("bundle encode");
        let restored = decrypt_project_bundle(&bundle, "bundle-password").expect("bundle decode");

        assert_eq!(restored.name, project.name);
        assert_eq!(
            restored
                .environments
                .get("local")
                .and_then(|environment| environment.variables.get("JWT_SECRET"))
                .map(|variable| variable.value()),
            Some("super-secret")
        );
    }

    #[test]
    fn encrypted_bundle_rejects_wrong_password() {
        let project = Project::new("api-auth", None);
        let bundle = encrypt_project_bundle(&project, "local", "bundle-password", "0.1.0")
            .expect("bundle encode");

        let error = decrypt_project_bundle(&bundle, "wrong-password").expect_err("wrong pass");
        assert!(matches!(error, EnvltError::BundleDecryptFailed));
    }

    #[test]
    fn decrypt_rejects_a_payload_missing_the_header_environment() {
        // A tampered or corrupted bundle whose payload doesn't actually
        // contain the environment its own header claims to carry.
        let mut project = Project::new("api-auth", None);
        project
            .environments
            .insert("staging".to_owned(), Environment::new("staging"));

        let bundle = encrypt_project_bundle(&project, "local", "bundle-password", "0.1.0")
            .expect("bundle encode");

        let error = decrypt_project_bundle(&bundle, "bundle-password").expect_err("mismatch");
        assert!(matches!(error, EnvltError::InvalidBundlePayload));
    }

    /// Bundles written before KDF params were recorded in the header must
    /// keep decrypting with the exact scrypt settings they were encrypted
    /// with, even though `Params::recommended()` may change in a future
    /// scrypt release. Simulates such a legacy header (from early in the v2
    /// bundle format's lifetime) by hand-encoding one without
    /// `kdf_log_n`/`kdf_r`/`kdf_p` and relying on `serde` defaults to fill
    /// in the historical values (log_n=17, r=8, p=1).
    #[test]
    fn decrypt_falls_back_to_legacy_kdf_params_for_headers_without_them() {
        use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            ChaCha20Poly1305, Key, Nonce,
        };
        use scrypt::{scrypt, Params};

        let mut project = Project::new("legacy-project", None);
        project
            .environments
            .insert("local".to_owned(), Environment::new("local"));
        let plaintext = toml::to_string(&project).expect("serialize project");

        let salt = [3_u8; super::BUNDLE_SALT_LEN];
        let nonce = [4_u8; super::BUNDLE_NONCE_LEN];
        let params = Params::new(17, 8, 1).expect("legacy params");
        let mut key = [0_u8; 32];
        scrypt(b"legacy-password", &salt, &params, &mut key).expect("derive key");

        let cipher = ChaCha20Poly1305::new(&Key::from(key));
        let mut ciphertext = cipher
            .encrypt(&Nonce::from(nonce), plaintext.as_bytes())
            .expect("encrypt");
        let tag: Vec<u8> = ciphertext.split_off(ciphertext.len() - super::BUNDLE_TAG_LEN);

        // Hand-built header JSON with no kdf_log_n/kdf_r/kdf_p, matching what
        // encrypt_project_bundle produced before this field was added.
        let header_json = serde_json::json!({
            "project": "legacy-project",
            "environment": "local",
            "exported_at": Utc::now().to_rfc3339(),
            "envlt_version": "0.3.0",
            "kdf_salt_b64": STANDARD_NO_PAD.encode(salt),
        })
        .to_string();
        let header_bytes = header_json.into_bytes();

        let mut bundle = Vec::new();
        bundle.extend_from_slice(&BUNDLE_MAGIC);
        bundle.push(2);
        bundle.extend_from_slice(&(header_bytes.len() as u16).to_be_bytes());
        bundle.extend_from_slice(&header_bytes);
        bundle.extend_from_slice(&nonce);
        bundle.extend_from_slice(&ciphertext);
        bundle.extend_from_slice(&tag);

        let restored =
            decrypt_project_bundle(&bundle, "legacy-password").expect("decrypt legacy bundle");
        assert_eq!(restored.name, "legacy-project");
    }
}
