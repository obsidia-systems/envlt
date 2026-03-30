use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::read_passphrase;

pub fn run_list(service: &AppService) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let projects = service.list_projects(&passphrase)?;

    if projects.is_empty() {
        println!("No projects found.");
    } else {
        for project in projects {
            println!("{}", project.name);
        }
    }

    Ok(ExitCode::SUCCESS)
}
