use std::process::ExitCode;

use anyhow::Result;
use chrono::Utc;
use envlt_core::{AppService, VarType};
use serde_json::to_string_pretty;

use crate::cli::{read_passphrase, resolve_environment};
use crate::output::{render_table_styled, rows_to_json_objects, CellStyle, OutputFormat};

pub fn run_vars(
    service: &AppService,
    project: &Option<String>,
    env: &Option<String>,
    format: OutputFormat,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let environment = resolve_environment(env.as_deref(), None)?;
    let variables = service.project_variable_views(&project, &environment, &passphrase)?;

    if variables.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Table => println!("No variables found."),
        }
        return Ok(ExitCode::SUCCESS);
    }

    let headers = ["key", "type", "value", "last modified"];
    let mut styles = Vec::new();
    let rows = variables
        .into_iter()
        .map(|variable| {
            let type_style = match variable.var_type {
                VarType::Secret => CellStyle::Warn,
                VarType::Plain => CellStyle::Plain,
            };
            styles.push(vec![
                CellStyle::Plain,
                type_style,
                CellStyle::Plain,
                CellStyle::Plain,
            ]);
            vec![
                variable.key,
                format_var_type(variable.var_type).to_owned(),
                format_value(&variable.value, variable.var_type),
                format_timestamp(variable.updated_at),
            ]
        })
        .collect::<Vec<_>>();

    match format {
        OutputFormat::Table => println!("{}", render_table_styled(&headers, &rows, &styles)),
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
        VarType::Plain => "plain",
    }
}

fn format_value(value: &str, var_type: VarType) -> String {
    match var_type {
        VarType::Secret => mask_secret(value),
        VarType::Plain => value.to_owned(),
    }
}

fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "[hidden]".to_owned();
    }

    let visible_prefix: String = value.chars().take(2).collect();
    format!("{visible_prefix}***")
}

fn format_timestamp(timestamp: chrono::DateTime<Utc>) -> String {
    timestamp.format("%m-%d %H:%M").to_string()
}
