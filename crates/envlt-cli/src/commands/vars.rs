use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{AppService, VarType};

use crate::cli::read_passphrase;

pub fn run_vars(service: &AppService, project: &Option<String>) -> Result<ExitCode> {
    let passphrase = read_passphrase(false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let variables = service.project_variable_views(&project, &passphrase)?;

    if variables.is_empty() {
        println!("No variables found.");
        return Ok(ExitCode::SUCCESS);
    }

    for variable in variables {
        println!(
            "{}\t{}\t{}",
            variable.key,
            format_var_type(variable.var_type),
            format_value(&variable.value, variable.var_type)
        );
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
