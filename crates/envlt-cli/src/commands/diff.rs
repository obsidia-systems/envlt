use std::process::ExitCode;

use anyhow::{anyhow, Result};
use envlt_core::AppService;
use serde_json::{json, to_string_pretty};

use crate::cli::read_passphrase;
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

pub fn run_diff(
    service: &AppService,
    project: &Option<String>,
    other_project: &Option<String>,
    example: &Option<std::path::PathBuf>,
    format: OutputFormat,
) -> Result<ExitCode> {
    if example.is_none() && other_project.is_none() {
        return Err(anyhow!(
            "diff requires either --example <path> or a second project name"
        ));
    }

    let passphrase = read_passphrase(service.store(), false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;

    if let Some(example) = example {
        let diff = service.diff_project_against_example(&project, example, &passphrase)?;

        let metadata_rows = vec![
            vec!["mode".to_owned(), "example".to_owned()],
            vec!["project".to_owned(), diff.project.clone()],
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
                    "example": diff.example_path,
                    "summary": summary,
                    "items": items,
                });
                println!("{}", to_string_pretty(&payload)?);
            }
        }
    } else if let Some(other_project) = other_project {
        let diff = service.diff_projects(&project, other_project, &passphrase)?;

        let metadata_rows = vec![
            vec!["mode".to_owned(), "project".to_owned()],
            vec!["left".to_owned(), diff.left_project.clone()],
            vec!["right".to_owned(), diff.right_project.clone()],
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
                println!("left\t{}", diff.left_project);
                println!("right\t{}", diff.right_project);
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
                    "left": diff.left_project,
                    "right": diff.right_project,
                    "summary": summary,
                    "items": items,
                });
                println!("{}", to_string_pretty(&payload)?);
            }
        }
    } else {
        unreachable!("validated above");
    }

    Ok(ExitCode::SUCCESS)
}
