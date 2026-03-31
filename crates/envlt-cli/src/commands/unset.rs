use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_passphrase};

pub fn run_unset(service: &AppService, project: &Option<String>, key: &str) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    service.unset_variable(&project, key, &passphrase)?;
    print_success("Variable removed.")?;
    Ok(ExitCode::SUCCESS)
}
