use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{AppService, VarType};
use serde_json::to_string_pretty;

use crate::cli::read_passphrase;
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

pub fn run_vars(
    service: &AppService,
    project: &Option<String>,
    format: OutputFormat,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let variables = service.project_variable_views(&project, &passphrase)?;

    if variables.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Raw | OutputFormat::Table => println!("No variables found."),
        }
        return Ok(ExitCode::SUCCESS);
    }

    let headers = ["key", "type", "value"];
    let rows = variables
        .into_iter()
        .map(|variable| {
            vec![
                variable.key,
                format_var_type(variable.var_type).to_owned(),
                format_value(&variable.value, variable.var_type),
            ]
        })
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

fn format_var_type(var_type: VarType) -> &'static str {
    match var_type {
        VarType::Secret => "secret",
        VarType::Config => "config",
        VarType::Plain => "plain",
    }
}

fn format_value(value: &str, var_type: VarType) -> String {
    match var_type {
        VarType::Secret => mask_secret(value),
        VarType::Config | VarType::Plain => value.to_owned(),
    }
}

fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "[hidden]".to_owned();
    }

    let visible_prefix: String = value.chars().take(2).collect();
    format!("{visible_prefix}***")
}
