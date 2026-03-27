use std::{path::Path, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_passphrase};

pub fn run_use(service: &AppService, project: &Option<String>, out: &Path) -> Result<ExitCode> {
    let passphrase = read_passphrase(false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    service.write_env_file(&project, out, &passphrase)?;
    print_success("Environment file written.")?;
    Ok(ExitCode::SUCCESS)
}
