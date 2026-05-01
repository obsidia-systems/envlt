use std::{path::Path, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::read_passphrase;

pub fn run_check(
    service: &AppService,
    project: &Option<String>,
    example: &Path,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let diff = service.diff_project_against_example(&project, example, &passphrase)?;

    if diff.missing_in_vault.is_empty() {
        println!("ok\tall required variables present");
        Ok(ExitCode::SUCCESS)
    } else {
        for key in &diff.missing_in_vault {
            println!("missing\t{key}");
        }
        Ok(ExitCode::FAILURE)
    }
}
