use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_bundle_passphrase, read_passphrase};

pub fn run_export(service: &AppService, project: &str, out: &PathBuf) -> Result<ExitCode> {
    let vault_passphrase = read_passphrase(service.store(), false)?;
    let bundle_passphrase = read_bundle_passphrase(true)?;
    let bundle = service.export_project_bundle(project, &vault_passphrase, &bundle_passphrase)?;
    fs::write(out, bundle)?;
    print_success("Bundle exported.")?;
    Ok(ExitCode::SUCCESS)
}
