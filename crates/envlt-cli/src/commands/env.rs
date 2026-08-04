use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;
use serde_json::to_string_pretty;

use crate::cli::{print_success, read_passphrase};
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

pub fn run_env_add(service: &AppService, name: &str, project: &Option<String>) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    service.add_environment(&project, name, &passphrase)?;
    print_success(&format!(
        "Environment '{name}' added to project '{project}'."
    ))?;
    Ok(ExitCode::SUCCESS)
}
