use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{AppService, EnvltError, VarType};

use crate::cli::{print_success, read_passphrase, resolve_environment};

pub fn run_set(
    service: &AppService,
    project: &Option<String>,
    env: &Option<String>,
    assignment: &str,
    secret: bool,
    plain: bool,
) -> Result<ExitCode> {
    let (key, value) = assignment
        .split_once('=')
        .ok_or_else(|| EnvltError::InvalidAssignment {
            input: assignment.to_owned(),
        })?;
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let environment = resolve_environment(env.as_deref(), None)?;
    let var_type = match (secret, plain) {
        (true, false) => Some(VarType::Secret),
        (false, true) => Some(VarType::Plain),
        (false, false) => None,
        _ => unreachable!("clap enforces mutual exclusivity"),
    };
    service.set_variable(&project, &environment, key, value, var_type, &passphrase)?;
    print_success("Variable saved.")?;
    Ok(ExitCode::SUCCESS)
}
