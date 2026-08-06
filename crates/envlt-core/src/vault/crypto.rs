use std::io::{Read, Write};

use age::secrecy::SecretString;
use age::{Decryptor, Encryptor, Identity};
use zeroize::Zeroizing;

use crate::error::{EnvltError, Result};

pub fn encrypt(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor = Encryptor::with_user_passphrase(SecretString::from(passphrase.to_owned()));
    let mut output = Vec::new();
    let mut writer = encryptor.wrap_output(&mut output)?;
    writer.write_all(plaintext)?;
    writer.finish()?;
    Ok(output)
}

/// Decrypt `ciphertext` and return the plaintext zeroized on drop, since it
/// is the entire vault or project contents -- every secret value included.
pub fn decrypt(ciphertext: &[u8], passphrase: &str) -> Result<Zeroizing<Vec<u8>>> {
    let decryptor = Decryptor::new(ciphertext).map_err(EnvltError::AgeDecrypt)?;
    if !decryptor.is_scrypt() {
        return Err(EnvltError::InvalidPassphrase);
    }

    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()));
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn Identity))
        .map_err(|_| EnvltError::InvalidPassphrase)?;

    let mut output = Zeroizing::new(Vec::new());
    reader.read_to_end(&mut output)?;
    Ok(output)
}
