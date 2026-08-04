use std::{env, process::ExitCode};

use anyhow::Result;
use envlt_core::AppService;
use serde_json::to_string_pretty;

use crate::cli::{confirm_action, print_success, read_passphrase};
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

pub fn run_env_list(
    service: &AppService,
    project: &Option<String>,
    format: OutputFormat,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let environments = service.list_environments(&project, &passphrase)?;

    let headers = ["environment"];
    let rows = environments
        .into_iter()
        .map(|name| vec![name])
        .collect::<Vec<_>>();

    match format {
        OutputFormat::Table => println!("{}", render_table(&headers, &rows)),
        OutputFormat::Raw => println!("{}", render_raw_rows(&rows)),
        OutputFormat::Json => {
            let json = rows_to_json_objects(&headers, &rows);
            println!("{}", to_string_pretty(&json)?);
        }
    }

    Ok(ExitCode::SUCCESS)
}

pub fn run_env_add(
    service: &AppService,
    name: &str,
    project: &Option<String>,
    from: &Option<String>,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    service.add_environment(&project, name, from.as_deref(), &passphrase)?;
    let message = match from {
        Some(source) => {
            format!("Environment '{name}' added to project '{project}' (seeded from '{source}').")
        }
        None => format!("Environment '{name}' added to project '{project}'."),
    };
    print_success(&message)?;
    Ok(ExitCode::SUCCESS)
}

pub fn run_env_remove(
    service: &AppService,
    name: &str,
    project: &Option<String>,
    yes: bool,
) -> Result<ExitCode> {
    let project = service.resolve_project_name(project.as_deref(), None)?;

    if !yes {
        let confirmed = confirm_action(
            Some("ENVLT_ENV_REMOVE_CONFIRM"),
            &format!(
                "Remove environment '{name}' from project '{project}'? This deletes all its \
                 variables and their history. [y/N]: "
            ),
        )?;

        if !confirmed {
            print_success("Removal cancelled.")?;
            return Ok(ExitCode::SUCCESS);
        }
    }

    let passphrase = read_passphrase(service.store(), false)?;
    service.remove_environment(&project, name, &passphrase)?;
    print_success(&format!(
        "Environment '{name}' removed from project '{project}'."
    ))?;
    Ok(ExitCode::SUCCESS)
}

pub fn run_env_switch(
    service: &AppService,
    name: &str,
    project: &Option<String>,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let current_dir = env::current_dir()?;
    service.switch_environment(&project, name, &current_dir, &passphrase)?;
    print_success(&format!(
        "'{}' now defaults to environment '{name}' for project '{project}'.",
        current_dir.display()
    ))?;
    Ok(ExitCode::SUCCESS)
}
