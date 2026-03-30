use std::{fs, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_bundle_passphrase, read_passphrase};

pub fn run_import(
    service: &AppService,
    file: &std::path::Path,
    overwrite: bool,
) -> Result<ExitCode> {
    let vault_passphrase = read_passphrase(service.store(), false)?;
    let bundle_passphrase = read_bundle_passphrase(false)?;
    let bundle = fs::read(file)?;
    let project =
        service.import_project_bundle(&bundle, &vault_passphrase, &bundle_passphrase, overwrite)?;
    print_success(&format!("Bundle imported for project '{project}'."))?;
    Ok(ExitCode::SUCCESS)
}
