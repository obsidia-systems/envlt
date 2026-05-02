use std::process::ExitCode;

use anyhow::Result;
use chrono::Utc;
use envlt_core::{ActivityAction, ActivityEvent, AppService};
use serde_json::to_string_pretty;

use crate::cli::read_passphrase;
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

pub fn run_history(
    service: &AppService,
    project: &Option<String>,
    key: Option<&str>,
    format: OutputFormat,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;

    let events = match key {
        Some(k) => service.variable_history(&project, k, &passphrase)?,
        None => service.project_activity_log(&project, &passphrase)?,
    };

    if events.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Raw | OutputFormat::Table => println!("No history found."),
        }
        return Ok(ExitCode::SUCCESS);
    }

    match key {
        Some(k) => render_variable_history(&project, k, &events, format),
        None => render_project_history(&project, &events, format),
    }

    Ok(ExitCode::SUCCESS)
}

fn render_variable_history(
    project: &str,
    key: &str,
    events: &[ActivityEvent],
    format: OutputFormat,
) {
    let headers = ["timestamp", "action", "old value", "new value"];
    let rows: Vec<Vec<String>> = events
        .iter()
        .map(|event| {
            vec![
                format_timestamp(event.timestamp),
                format_action(event.action).to_owned(),
                format_optional_value(&event.old_value),
                format_optional_value(&event.new_value),
            ]
        })
        .collect();

    match format {
        OutputFormat::Table => {
            println!("Variable history for {key} in project {project}:");
            println!("{}", render_table(&headers, &rows));
        }
        OutputFormat::Raw => println!("{}", render_raw_rows(&rows)),
        OutputFormat::Json => {
            let json = rows_to_json_objects(&headers, &rows);
            println!("{}", to_string_pretty(&json).unwrap_or_default());
        }
    }
}

fn render_project_history(project: &str, events: &[ActivityEvent], format: OutputFormat) {
    let headers = ["timestamp", "action", "variable", "detail"];
    let rows: Vec<Vec<String>> = events
        .iter()
        .map(|event| {
            vec![
                format_timestamp(event.timestamp),
                format_action(event.action).to_owned(),
                event.variable_key.clone(),
                format_detail(event),
            ]
        })
        .collect();

    match format {
        OutputFormat::Table => {
            println!("Project activity log for {project}:");
            println!("{}", render_table(&headers, &rows));
        }
        OutputFormat::Raw => println!("{}", render_raw_rows(&rows)),
        OutputFormat::Json => {
            let json = rows_to_json_objects(&headers, &rows);
            println!("{}", to_string_pretty(&json).unwrap_or_default());
        }
    }
}

fn format_timestamp(timestamp: chrono::DateTime<Utc>) -> String {
    timestamp.format("%m-%d %H:%M").to_string()
}

fn format_action(action: ActivityAction) -> &'static str {
    match action {
        ActivityAction::VariableCreated => "Created",
        ActivityAction::VariableUpdated => "Value updated",
        ActivityAction::VariableDeleted => "Deleted",
        ActivityAction::VariableTypeChanged => "Type changed",
    }
}

fn format_optional_value(value: &Option<String>) -> String {
    match value {
        Some(v) => v.clone(),
        None => "********".to_owned(),
    }
}

fn format_detail(event: &ActivityEvent) -> String {
    match event.action {
        ActivityAction::VariableCreated => match &event.new_value {
            Some(v) => v.clone(),
            None => "********".to_owned(),
        },
        ActivityAction::VariableUpdated => {
            let old = format_optional_value(&event.old_value);
            let new = format_optional_value(&event.new_value);
            format!("{old} → {new}")
        }
        ActivityAction::VariableDeleted => match &event.old_value {
            Some(v) => v.clone(),
            None => "********".to_owned(),
        },
        ActivityAction::VariableTypeChanged => {
            let old = event.old_type.map(format_var_type).unwrap_or("unknown");
            let new = event.new_type.map(format_var_type).unwrap_or("unknown");
            format!("{old} → {new}")
        }
    }
}

fn format_var_type(var_type: envlt_core::VarType) -> &'static str {
    match var_type {
        envlt_core::VarType::Secret => "Secret",
        envlt_core::VarType::Config => "Config",
        envlt_core::VarType::Plain => "Plain",
    }
}
