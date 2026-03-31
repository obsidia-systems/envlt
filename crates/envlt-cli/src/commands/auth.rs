use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{auth_status, clear_stored_passphrase, save_stored_passphrase, AppService};
use serde_json::{json, to_string_pretty};

use crate::cli::{print_success, read_passphrase_without_keyring};
use crate::output::{render_table, OutputFormat};

pub fn run_auth_save(service: &AppService) -> Result<ExitCode> {
    let passphrase = read_passphrase_without_keyring(false)?;
    service.verify_vault_access(&passphrase)?;
    save_stored_passphrase(service.store(), &passphrase)?;
    print_success("Vault passphrase saved to the system keyring.")?;
    Ok(ExitCode::SUCCESS)
}

pub fn run_auth_clear(service: &AppService) -> Result<ExitCode> {
    if clear_stored_passphrase(service.store())? {
        print_success("Stored vault passphrase removed from the system keyring.")?;
    } else {
        print_success("No stored vault passphrase was found in the system keyring.")?;
    }

    Ok(ExitCode::SUCCESS)
}

pub fn run_auth_status(service: &AppService, format: OutputFormat) -> Result<ExitCode> {
    let status = auth_status(service.store())?;
    let env_value = if status.env_var_present {
        "present"
    } else {
        "missing"
    };
    let keyring_value = if status.keyring_available {
        "configured"
    } else {
        "not_configured"
    };

    match format {
        OutputFormat::Raw => {
            println!("env\t{env_value}");
            println!("keyring\t{keyring_value}");
            println!("target\t{}", status.keyring_target);
        }
        OutputFormat::Table => {
            let rows = vec![
                vec!["env".to_owned(), env_value.to_owned()],
                vec!["keyring".to_owned(), keyring_value.to_owned()],
                vec!["target".to_owned(), status.keyring_target],
            ];
            println!("{}", render_table(&["source", "status"], &rows));
        }
        OutputFormat::Json => {
            let payload = json!({
                "env": env_value,
                "keyring": keyring_value,
                "target": status.keyring_target,
            });
            println!("{}", to_string_pretty(&payload)?);
        }
    }

    Ok(ExitCode::SUCCESS)
}
