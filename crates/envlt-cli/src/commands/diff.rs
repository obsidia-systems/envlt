use std::process::ExitCode;

use anyhow::{anyhow, Result};
use envlt_core::AppService;
use serde_json::{json, to_string_pretty};

use crate::cli::{read_passphrase, resolve_environment};
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

#[allow(clippy::too_many_arguments)]
pub fn run_diff(
    service: &AppService,
    project: &Option<String>,
    env: &Option<String>,
    other_project: &Option<String>,
    other_env: &Option<String>,
    example: &Option<std::path::PathBuf>,
    format: OutputFormat,
) -> Result<ExitCode> {
    if example.is_none() && other_project.is_none() && other_env.is_none() {
        return Err(anyhow!(
            "diff requires either --example <path>, a second project name, or --other-env"
        ));
    }

    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;
    let environment = resolve_environment(env.as_deref(), None)?;

    if let Some(example) = example {
        let diff =
            service.diff_project_against_example(&project, &environment, example, &passphrase)?;

        let metadata_rows = vec![
            vec!["mode".to_owned(), "example".to_owned()],
            vec!["project".to_owned(), diff.project.clone()],
            vec!["environment".to_owned(), environment.clone()],
            vec![
                "example".to_owned(),
                diff.example_path.display().to_string(),
            ],
        ];
        let summary_rows = vec![
            vec!["shared".to_owned(), diff.shared_keys.len().to_string()],
            vec![
                "missing".to_owned(),
                diff.missing_in_vault.len().to_string(),
            ],
            vec!["extra".to_owned(), diff.extra_in_vault.len().to_string()],
        ];

        let mut item_rows = Vec::new();
        for key in &diff.shared_keys {
            item_rows.push(vec!["ok".to_owned(), key.clone()]);
        }
        for key in &diff.missing_in_vault {
            item_rows.push(vec!["missing".to_owned(), key.clone()]);
        }
        for key in &diff.extra_in_vault {
            item_rows.push(vec!["extra".to_owned(), key.clone()]);
        }

        match format {
            OutputFormat::Raw => {
                println!("mode\texample");
                println!("project\t{}", diff.project);
                println!("environment\t{environment}");
                println!("example\t{}", diff.example_path.display());
                println!(
                    "summary\tshared={}\tmissing={}\textra={}",
                    diff.shared_keys.len(),
                    diff.missing_in_vault.len(),
                    diff.extra_in_vault.len()
                );
                println!("{}", render_raw_rows(&summary_rows));
                println!("{}", render_raw_rows(&item_rows));
            }
            OutputFormat::Table => {
                println!("{}", render_table(&["field", "value"], &metadata_rows));
                println!();
                println!("{}", render_table(&["metric", "count"], &summary_rows));
                println!();
                println!("{}", render_table(&["status", "key"], &item_rows));
            }
            OutputFormat::Json => {
                let summary = rows_to_json_objects(&["metric", "count"], &summary_rows);
                let items = rows_to_json_objects(&["status", "key"], &item_rows);
                let payload = json!({
                    "mode": "example",
                    "project": diff.project,
                    "environment": environment,
                    "example": diff.example_path,
                    "summary": summary,
                    "items": items,
                });
                println!("{}", to_string_pretty(&payload)?);
            }
        }
    } else {
        // No --example: compare two environments, which may belong to the
        // same project (--other-env alone) or two different projects
        // (a second project name, optionally paired with --other-env to
        // pick which of its environments to use).
        let right_project = other_project.clone().unwrap_or_else(|| project.clone());
        let right_environment = other_env.clone().unwrap_or_else(|| environment.clone());

        let diff = service.diff_projects(
            &project,
            &environment,
            &right_project,
            &right_environment,
            &passphrase,
        )?;

        let left_label = format!("{}@{environment}", diff.left_project);
        let right_label = format!("{}@{right_environment}", diff.right_project);

        let metadata_rows = vec![
            vec!["mode".to_owned(), "project".to_owned()],
            vec!["left".to_owned(), left_label.clone()],
            vec!["right".to_owned(), right_label.clone()],
        ];
        let summary_rows = vec![
            vec!["shared".to_owned(), diff.shared_keys.len().to_string()],
            vec![
                "changed_values".to_owned(),
                diff.changed_values.len().to_string(),
            ],
            vec![
                "changed_types".to_owned(),
                diff.changed_types.len().to_string(),
            ],
            vec!["only_left".to_owned(), diff.only_in_left.len().to_string()],
            vec![
                "only_right".to_owned(),
                diff.only_in_right.len().to_string(),
            ],
        ];

        let mut item_rows = Vec::new();
        for key in &diff.shared_keys {
            item_rows.push(vec!["ok".to_owned(), key.clone()]);
        }
        for key in &diff.changed_values {
            item_rows.push(vec!["value_changed".to_owned(), key.clone()]);
        }
        for key in &diff.changed_types {
            item_rows.push(vec!["type_changed".to_owned(), key.clone()]);
        }
        for key in &diff.only_in_left {
            item_rows.push(vec!["left_only".to_owned(), key.clone()]);
        }
        for key in &diff.only_in_right {
            item_rows.push(vec!["right_only".to_owned(), key.clone()]);
        }

        match format {
            OutputFormat::Raw => {
                println!("mode\tproject");
                println!("left\t{left_label}");
                println!("right\t{right_label}");
                println!(
                    "summary\tshared={}\tchanged_values={}\tchanged_types={}\tonly_left={}\tonly_right={}",
                    diff.shared_keys.len(),
                    diff.changed_values.len(),
                    diff.changed_types.len(),
                    diff.only_in_left.len(),
                    diff.only_in_right.len()
                );
                println!("{}", render_raw_rows(&summary_rows));
                println!("{}", render_raw_rows(&item_rows));
            }
            OutputFormat::Table => {
                println!("{}", render_table(&["field", "value"], &metadata_rows));
                println!();
                println!("{}", render_table(&["metric", "count"], &summary_rows));
                println!();
                println!("{}", render_table(&["status", "key"], &item_rows));
            }
            OutputFormat::Json => {
                let summary = rows_to_json_objects(&["metric", "count"], &summary_rows);
                let items = rows_to_json_objects(&["status", "key"], &item_rows);
                let payload = json!({
                    "mode": "project",
                    "left": left_label,
                    "right": right_label,
                    "summary": summary,
                    "items": items,
                });
                println!("{}", to_string_pretty(&payload)?);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
