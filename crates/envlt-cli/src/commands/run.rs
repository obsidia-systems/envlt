use std::process::{Command, ExitCode};

use anyhow::Result;
use envlt_core::{AppService, EnvltError};

use crate::cli::read_passphrase;

pub fn run_run(
    service: &AppService,
    project: &Option<String>,
    command: &[String],
) -> Result<ExitCode> {
    let program = command.first().ok_or(EnvltError::MissingCommand)?;
    let passphrase = read_passphrase(false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let env = service.build_run_environment(&project, &passphrase)?;

    let mut child = Command::new(program);
    child.args(&command[1..]);
    child.envs(env.variables);

    let status = child.status()?;
    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}
