use std::{env, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::{confirm_action, print_success, read_passphrase};

pub fn run_remove(service: &AppService, project: &str, yes: bool) -> Result<ExitCode> {
    if !yes {
        let confirmed = confirm_action(
            Some("ENVLT_REMOVE_CONFIRM"),
            &format!("Remove project '{project}' from the vault? [y/N]: "),
        )?;

        if !confirmed {
            print_success("Removal cancelled.")?;
            return Ok(ExitCode::SUCCESS);
        }
    }

    let passphrase = read_passphrase(service.store(), false)?;
    let current_dir = env::current_dir()?;
    let result = service.remove_project(project, Some(&current_dir), &passphrase)?;

    if result.removed_link {
        print_success(&format!(
            "Project '{}' removed from vault and .envlt-link cleared.",
            result.project
        ))?;
    } else {
        print_success(&format!("Project '{}' removed from vault.", result.project))?;
    }

    Ok(ExitCode::SUCCESS)
}
