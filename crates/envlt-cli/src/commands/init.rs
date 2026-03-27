use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_passphrase};

pub fn run_init(service: &AppService) -> Result<ExitCode> {
    let passphrase = read_passphrase(true)?;
    service.init_vault(&passphrase)?;
    print_success("Vault initialized.")?;
    Ok(ExitCode::SUCCESS)
}
