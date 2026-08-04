use std::{path::Path, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_passphrase, resolve_environment};

pub fn run_pull(
    service: &AppService,
    project: &Option<String>,
    env: &Option<String>,
    out: &Path,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let environment = resolve_environment(env.as_deref(), None)?;
    service.write_env_file(&project, &environment, out, &passphrase)?;

    eprintln!("Warning: generated .env files are plaintext artifacts.");
    eprintln!("         Keep them out of version control and delete them when no longer needed.");
    eprintln!("         Prefer 'envlt run' when a file on disk is not required.");

    print_success("Environment file written.")?;
    Ok(ExitCode::SUCCESS)
}
