use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_passphrase_without_keyring};

pub fn run_init(service: &AppService) -> Result<ExitCode> {
    let passphrase = read_passphrase_without_keyring(true)?;
    service.init_vault(&passphrase)?;
    print_success("Vault initialized.")?;
    Ok(ExitCode::SUCCESS)
}
