use std::io::{self, Write};

use anyhow::{anyhow, Result};
use envlt_core::VarType;

pub fn read_passphrase(confirm: bool) -> Result<String> {
    if let Some(passphrase) = std::env::var_os("ENVLT_PASSPHRASE") {
        return passphrase
            .into_string()
            .map_err(|_| anyhow!("ENVLT_PASSPHRASE contains invalid UTF-8"));
    }

    let passphrase = rpassword::prompt_password("Vault passphrase: ")?;
    if confirm {
        let confirmation = rpassword::prompt_password("Confirm passphrase: ")?;
        if passphrase != confirmation {
            return Err(anyhow!("passphrases do not match"));
        }
    }

    Ok(passphrase)
}

pub fn read_bundle_passphrase(confirm: bool) -> Result<String> {
    if let Some(passphrase) = std::env::var_os("ENVLT_BUNDLE_PASSPHRASE") {
        return passphrase
            .into_string()
            .map_err(|_| anyhow!("ENVLT_BUNDLE_PASSPHRASE contains invalid UTF-8"));
    }

    let passphrase = rpassword::prompt_password("Bundle passphrase: ")?;
    if confirm {
        let confirmation = rpassword::prompt_password("Confirm bundle passphrase: ")?;
        if passphrase != confirmation {
            return Err(anyhow!("bundle passphrases do not match"));
        }
    }

    Ok(passphrase)
}

pub fn print_success(message: &str) -> Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{message}")?;
    Ok(())
}

pub fn read_example_value(key: &str, var_type: VarType) -> Result<String> {
    let env_key = format!("ENVLT_EXAMPLE_{}", sanitize_env_key(key));
    if let Some(value) = std::env::var_os(&env_key) {
        return value
            .into_string()
            .map_err(|_| anyhow!("{env_key} contains invalid UTF-8"));
    }

    match var_type {
        VarType::Secret => rpassword::prompt_password(format!("{key}: ")).map_err(Into::into),
        VarType::Config | VarType::Plain => {
            let mut stdout = io::stdout().lock();
            write!(stdout, "{key}: ")?;
            stdout.flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            Ok(input.trim_end_matches(['\r', '\n']).to_owned())
        }
    }
}

pub fn read_gen_type(default: &str) -> Result<String> {
    let env_key = "ENVLT_GEN_TYPE";
    if let Some(value) = std::env::var_os(env_key) {
        return value
            .into_string()
            .map_err(|_| anyhow!("{env_key} contains invalid UTF-8"));
    }

    let mut stdout = io::stdout().lock();
    write!(stdout, "Generator type [{default}]: ")?;
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let value = input.trim_end_matches(['\r', '\n']).trim();
    if value.is_empty() {
        Ok(default.to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn sanitize_env_key(key: &str) -> String {
    key.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}
