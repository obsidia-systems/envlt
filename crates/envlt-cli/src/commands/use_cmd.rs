use std::{path::Path, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{print_success, read_passphrase};

pub fn run_use(service: &AppService, project: &Option<String>, out: &Path) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    service.write_env_file(&project, out, &passphrase)?;

    eprintln!("Warning: generated .env files are plaintext artifacts.");
    eprintln!("         Keep them out of version control and delete them when no longer needed.");
    eprintln!("         Prefer 'envlt run' when a file on disk is not required.");

    print_success("Environment file written.")?;
    Ok(ExitCode::SUCCESS)
}
