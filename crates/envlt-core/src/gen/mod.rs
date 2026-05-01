use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use uuid::Uuid;

use crate::{
    error::{EnvltError, Result},
    vault::VarType,
};

const API_KEY_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const HEX_ALPHABET: &[u8] = b"0123456789abcdef";
const ALNUM_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const SYMBOL_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{}:,.?";
const PASSWORD_WORDS: &[&str] = &[
    "amber", "anchor", "apricot", "atlas", "bamboo", "beacon", "birch", "bravo", "cactus",
    "canyon", "cedar", "citron", "cobalt", "comet", "coral", "cosmos", "crystal", "delta", "ember",
    "falcon", "forest", "frost", "galaxy", "garden", "glacier", "harbor", "hazel", "island",
    "jungle", "lagoon", "lantern", "meadow", "meteor", "midnight", "mosaic", "nova", "oasis",
    "onyx", "orchid", "pepper", "phoenix", "pine", "prairie", "quartz", "rainbow", "reef", "river",
    "rocket", "saffron", "shadow", "solstice", "spruce", "summit", "sunset", "thunder", "topaz",
    "valley", "velvet", "violet", "willow",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenType {
    JwtSecret,
    Uuid,
    ApiKey,
    Token,
    Password,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    Alnum,
    Hex,
    Symbols,
}

impl GenType {
    pub fn parse(input: &str) -> Result<Self> {
        match input {
            "jwt-secret" => Ok(Self::JwtSecret),
            "uuid" => Ok(Self::Uuid),
            "api-key" => Ok(Self::ApiKey),
            "token" => Ok(Self::Token),
            "password" => Ok(Self::Password),
            _ => Err(EnvltError::UnsupportedGenType {
                gen_type: input.to_owned(),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::JwtSecret => "jwt-secret",
            Self::Uuid => "uuid",
            Self::ApiKey => "api-key",
            Self::Token => "token",
            Self::Password => "password",
        }
    }

    pub fn default_var_type(self) -> VarType {
        match self {
            Self::JwtSecret | Self::ApiKey | Self::Token | Self::Password => VarType::Secret,
            Self::Uuid => VarType::Config,
        }
    }
}

pub fn supported_gen_types() -> &'static [GenType] {
    const TYPES: &[GenType] = &[
        GenType::JwtSecret,
        GenType::Uuid,
        GenType::ApiKey,
        GenType::Token,
        GenType::Password,
    ];
    TYPES
}

pub fn generate_value(gen_type: GenType) -> String {
    match gen_type {
        GenType::JwtSecret => generate_hex(32),
        GenType::Uuid => Uuid::new_v4().to_string(),
        GenType::ApiKey => generate_base58(40),
        GenType::Token => generate_base64url_chars(48),
        GenType::Password => generate_memorable_password(4),
    }
}

pub fn generate_custom_value(len: usize, charset: Charset) -> Result<String> {
    if len == 0 {
        return Err(EnvltError::InvalidGenLength { length: len });
    }

    let alphabet = match charset {
        Charset::Alnum => ALNUM_ALPHABET,
        Charset::Hex => HEX_ALPHABET,
        Charset::Symbols => SYMBOL_ALPHABET,
    };

    Ok(generate_from_alphabet(len, alphabet))
}

fn generate_hex(byte_len: usize) -> String {
    let mut bytes = vec![0_u8; byte_len];
    OsRng.fill_bytes(&mut bytes);
    let mut output = String::with_capacity(byte_len * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn generate_base64url_chars(len: usize) -> String {
    let mut bytes = vec![0_u8; len];
    OsRng.fill_bytes(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    encoded.chars().take(len).collect()
}

fn generate_base58(len: usize) -> String {
    generate_from_alphabet(len, API_KEY_ALPHABET)
}

fn generate_from_alphabet(len: usize, alphabet: &[u8]) -> String {
    let alphabet_len = alphabet.len();
    let max_valid = 256 - (256 % alphabet_len);
    let mut output = String::with_capacity(len);

    while output.len() < len {
        let mut byte = 0_u8;
        OsRng.fill_bytes(std::slice::from_mut(&mut byte));
        if (byte as usize) >= max_valid {
            continue;
        }
        let index = (byte as usize) % alphabet_len;
        output.push(alphabet[index] as char);
    }

    output
}

fn generate_memorable_password(words: usize) -> String {
    let word_count = PASSWORD_WORDS.len();
    let max_valid = 256 - (256 % word_count);
    let mut output = Vec::with_capacity(words);

    while output.len() < words {
        let mut byte = 0_u8;
        OsRng.fill_bytes(std::slice::from_mut(&mut byte));
        if (byte as usize) >= max_valid {
            continue;
        }
        let index = (byte as usize) % word_count;
        output.push(PASSWORD_WORDS[index]);
    }

    output.join("-")
}

#[cfg(test)]
mod tests {
    use super::{generate_custom_value, generate_value, supported_gen_types, Charset, GenType};

    #[test]
    fn supported_types_match_expected_values() {
        let types: Vec<_> = supported_gen_types()
            .iter()
            .map(|kind| kind.as_str())
            .collect();
        assert_eq!(
            types,
            vec!["jwt-secret", "uuid", "api-key", "token", "password"]
        );
    }

    #[test]
    fn jwt_secret_is_64_hex_chars() {
        let value = generate_value(GenType::JwtSecret);
        assert_eq!(value.len(), 64);
        assert!(value.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn api_key_has_expected_length() {
        let value = generate_value(GenType::ApiKey);
        assert_eq!(value.len(), 40);
    }

    #[test]
    fn token_has_expected_length() {
        let value = generate_value(GenType::Token);
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn custom_hex_generation_respects_length_and_charset() {
        let value = generate_custom_value(64, Charset::Hex).expect("custom hex");
        assert_eq!(value.len(), 64);
        assert!(value.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn custom_symbols_generation_respects_length() {
        let value = generate_custom_value(32, Charset::Symbols).expect("custom symbols");
        assert_eq!(value.len(), 32);
    }

    #[test]
    fn password_generation_uses_multiple_words() {
        let value = generate_value(GenType::Password);
        let parts: Vec<_> = value.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert!(parts.iter().all(|part| !part.is_empty()));
    }
}
