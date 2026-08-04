use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{read_passphrase, resolve_environment};

pub fn run_get(
    service: &AppService,
    key: &str,
    project: &Option<String>,
    env: &Option<String>,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let environment = resolve_environment(env.as_deref(), None)?;
    let value = service.get_variable_value(&project, &environment, key, &passphrase)?;
    println!("{value}");
    Ok(ExitCode::SUCCESS)
}
