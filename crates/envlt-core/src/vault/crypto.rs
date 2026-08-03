use std::io::{Read, Write};

use age::secrecy::SecretString;
use age::{Decryptor, Encryptor};
use zeroize::Zeroizing;

use crate::error::{EnvltError, Result};

pub fn encrypt(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor = Encryptor::with_user_passphrase(SecretString::from(passphrase.to_owned()));
    let mut output = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut output)
        .map_err(EnvltError::AgeEncrypt)?;
    writer.write_all(plaintext)?;
    writer.finish()?;
    Ok(output)
}

/// Decrypt `ciphertext` and return the plaintext zeroized on drop, since it
/// is the entire vault or project contents -- every secret value included.
pub fn decrypt(ciphertext: &[u8], passphrase: &str) -> Result<Zeroizing<Vec<u8>>> {
    let decryptor = Decryptor::new(ciphertext).map_err(EnvltError::AgeDecrypt)?;
    let mut reader = match decryptor {
        Decryptor::Passphrase(decryptor) => decryptor
            .decrypt(&SecretString::from(passphrase.to_owned()), None)
            .map_err(|_| EnvltError::InvalidPassphrase)?,
        _ => return Err(EnvltError::InvalidPassphrase),
    };

    let mut output = Zeroizing::new(Vec::new());
    reader.read_to_end(&mut output)?;
    Ok(output)
}
