use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{auth_status, clear_stored_passphrase, save_stored_passphrase, AppService};

use crate::cli::{print_success, read_passphrase_without_keyring};

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

pub fn run_auth_status(service: &AppService) -> Result<ExitCode> {
    let status = auth_status(service.store())?;

    println!(
        "env\t{}",
        if status.env_var_present {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "keyring\t{}",
        if status.keyring_available {
            "configured"
        } else {
            "not_configured"
        }
    );
    println!("target\t{}", status.keyring_target);

    Ok(ExitCode::SUCCESS)
}
