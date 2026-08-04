use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_bundle_passphrase, read_passphrase, resolve_environment};

pub fn run_export(
    service: &AppService,
    project: &str,
    env: &Option<String>,
    out: &PathBuf,
) -> Result<ExitCode> {
    let vault_passphrase = read_passphrase(service.store(), false)?;
    let bundle_passphrase = read_bundle_passphrase(true)?;
    let environment = resolve_environment(env.as_deref(), None)?;
    let bundle = service.export_project_bundle(
        project,
        &environment,
        &vault_passphrase,
        &bundle_passphrase,
    )?;
    fs::write(out, bundle)?;
    print_success("Bundle exported.")?;
    Ok(ExitCode::SUCCESS)
}
