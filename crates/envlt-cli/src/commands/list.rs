use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;
use serde_json::to_string_pretty;

use crate::cli::read_passphrase;
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

pub fn run_list(service: &AppService, format: OutputFormat) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let projects = service.list_projects(&passphrase)?;

    if projects.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Raw | OutputFormat::Table => println!("No projects found."),
        }

        return Ok(ExitCode::SUCCESS);
    }

    let headers = ["project"];
    let rows = projects
        .into_iter()
        .map(|project| vec![project.name])
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
