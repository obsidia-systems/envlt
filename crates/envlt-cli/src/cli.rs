use std::{
    io::{self, IsTerminal, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use envlt_core::{
    link::find_project_link, load_stored_passphrase, AppService, VarType, VaultStore,
};
use zeroize::Zeroizing;

/// Resolve the environment to operate on: an explicit `--env` flag takes
/// priority, then the environment recorded on the nearest `.envlt-link`
/// (if any), then `envlt_core::DEFAULT_ENVIRONMENT`.
pub fn resolve_environment(
    explicit_environment: Option<&str>,
    current_dir: Option<&Path>,
) -> Result<String> {
    let current_dir = match current_dir {
        Some(dir) => dir.to_path_buf(),
        None => std::env::current_dir()?,
    };

    let link_environment =
        find_project_link(&current_dir)?.and_then(|(_link_dir, _project, environment)| environment);

    Ok(AppService::resolve_environment_name(
        explicit_environment,
        link_environment.as_deref(),
    ))
}

/// A passphrase held only long enough to authenticate, zeroized on drop so
/// it doesn't linger in memory for the rest of the process's lifetime.
pub type Passphrase = Zeroizing<String>;

pub fn read_passphrase(store: &VaultStore, confirm: bool) -> Result<Passphrase> {
    if let Some(passphrase) = read_env_passphrase()? {
        return Ok(passphrase);
    }

    match load_stored_passphrase(store) {
        Ok(Some(passphrase)) => return Ok(passphrase),
        Ok(None) => {}
        Err(error) => {
            // Falling back to an interactive prompt only helps when there is
            // a human at the other end of stdin to answer it. In a script or
            // CI job, silently trying anyway would just hide the real
            // problem behind a confusing "no input" failure from the prompt
            // itself, so surface the keyring error directly instead.
            if !io::stdin().is_terminal() {
                return Err(anyhow!(
                    "failed to load stored passphrase from the system keyring: {error}"
                ));
            }
            eprintln!("Warning: failed to load stored passphrase: {error}");
        }
    }

    prompt_passphrase(confirm)
}

pub fn read_passphrase_without_keyring(confirm: bool) -> Result<Passphrase> {
    if let Some(passphrase) = read_env_passphrase()? {
        return Ok(passphrase);
    }

    prompt_passphrase(confirm)
}

pub fn read_passphrase_if_available(store: &VaultStore) -> Result<Option<Passphrase>> {
    if let Some(passphrase) = read_env_passphrase()? {
        return Ok(Some(passphrase));
    }

    Ok(load_stored_passphrase(store)?)
}

fn read_env_passphrase() -> Result<Option<Passphrase>> {
    if let Some(passphrase) = std::env::var_os("ENVLT_PASSPHRASE") {
        return passphrase
            .into_string()
            .map(|value| Some(Zeroizing::new(value)))
            .map_err(|_| anyhow!("ENVLT_PASSPHRASE contains invalid UTF-8"));
    }

    Ok(None)
}

fn prompt_passphrase(confirm: bool) -> Result<Passphrase> {
    let passphrase = Zeroizing::new(rpassword::prompt_password("Vault passphrase: ")?);
    if confirm {
        let confirmation = Zeroizing::new(rpassword::prompt_password("Confirm passphrase: ")?);
        if passphrase != confirmation {
            return Err(anyhow!("passphrases do not match"));
        }
    }

    Ok(passphrase)
}

pub fn read_bundle_passphrase(confirm: bool) -> Result<Passphrase> {
    if let Some(passphrase) = std::env::var_os("ENVLT_BUNDLE_PASSPHRASE") {
        return passphrase
            .into_string()
            .map(Zeroizing::new)
            .map_err(|_| anyhow!("ENVLT_BUNDLE_PASSPHRASE contains invalid UTF-8"));
    }

    let passphrase = Zeroizing::new(rpassword::prompt_password("Bundle passphrase: ")?);
    if confirm {
        let confirmation =
            Zeroizing::new(rpassword::prompt_password("Confirm bundle passphrase: ")?);
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
        VarType::Plain => {
            let mut stdout = io::stdout().lock();
            write!(stdout, "{key}: ")?;
            stdout.flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            Ok(input.trim_end_matches(['\r', '\n']).to_owned())
        }
    }
}

pub fn read_generate_type(default: &str) -> Result<String> {
    let env_key = "ENVLT_GENERATE_TYPE";
    let supported = "jwt-secret, uuid, api-key, token, password";
    let value = read_prompt_line(
        Some(env_key),
        &format!("Generator type [{default}] ({supported}): "),
    )?;
    if value.is_empty() {
        Ok(default.to_owned())
    } else {
        Ok(value)
    }
}

pub fn read_generate_save_choice() -> Result<bool> {
    let value = read_prompt_line(Some("ENVLT_GENERATE_SAVE"), "Save to vault? [y/N]: ")?;
    Ok(matches!(
        value.to_ascii_lowercase().as_str(),
        "y" | "yes" | "true" | "1"
    ))
}

pub fn read_generate_set_key() -> Result<String> {
    let value = read_prompt_line(
        Some("ENVLT_GENERATE_SET_KEY"),
        "Variable key to store generated value: ",
    )?;
    if value.is_empty() {
        Err(anyhow!("variable key cannot be empty"))
    } else {
        Ok(value)
    }
}

pub fn read_generate_project() -> Result<Option<String>> {
    let value = read_prompt_line(
        Some("ENVLT_GENERATE_PROJECT"),
        "Project name (leave empty to use .envlt-link): ",
    )?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub fn confirm_action(env_key: Option<&str>, prompt: &str) -> Result<bool> {
    let value = read_prompt_line(env_key, prompt)?;
    Ok(matches!(
        value.to_ascii_lowercase().as_str(),
        "y" | "yes" | "true" | "1"
    ))
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

fn read_prompt_line(env_key: Option<&str>, prompt: &str) -> Result<String> {
    if let Some(env_key) = env_key {
        if let Some(value) = std::env::var_os(env_key) {
            return value
                .into_string()
                .map_err(|_| anyhow!("{env_key} contains invalid UTF-8"));
        }
    }

    let mut stdout = io::stdout().lock();
    write!(stdout, "{prompt}")?;
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim_end_matches(['\r', '\n']).trim().to_owned())
}
