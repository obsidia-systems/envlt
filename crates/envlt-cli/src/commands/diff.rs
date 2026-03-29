use std::process::ExitCode;

use anyhow::{anyhow, Result};
use envlt_core::AppService;

use crate::cli::read_passphrase;

pub fn run_diff(
    service: &AppService,
    project: &Option<String>,
    other_project: &Option<String>,
    example: &Option<std::path::PathBuf>,
) -> Result<ExitCode> {
    if example.is_none() && other_project.is_none() {
        return Err(anyhow!(
            "diff requires either --example <path> or a second project name"
        ));
    }

    let passphrase = read_passphrase(false)?;
    let project = service.resolve_project_name(project.as_deref(), None)?;

    if let Some(example) = example {
        let diff = service.diff_project_against_example(&project, example, &passphrase)?;

        println!("mode\texample");
        println!("project\t{}", diff.project);
        println!("example\t{}", diff.example_path.display());
        println!(
            "summary\tshared={}\tmissing={}\textra={}",
            diff.shared_keys.len(),
            diff.missing_in_vault.len(),
            diff.extra_in_vault.len()
        );
        println!("shared\t{}", diff.shared_keys.len());
        println!("missing\t{}", diff.missing_in_vault.len());
        println!("extra\t{}", diff.extra_in_vault.len());

        for key in diff.shared_keys {
            println!("ok\t{key}");
        }
        for key in diff.missing_in_vault {
            println!("missing\t{key}");
        }
        for key in diff.extra_in_vault {
            println!("extra\t{key}");
        }
    } else if let Some(other_project) = other_project {
        let diff = service.diff_projects(&project, other_project, &passphrase)?;

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
        println!("shared\t{}", diff.shared_keys.len());
        println!("changed_values\t{}", diff.changed_values.len());
        println!("changed_types\t{}", diff.changed_types.len());
        println!("only_left\t{}", diff.only_in_left.len());
        println!("only_right\t{}", diff.only_in_right.len());

        for key in diff.shared_keys {
            println!("ok\t{key}");
        }
        for key in diff.changed_values {
            println!("value_changed\t{key}");
        }
        for key in diff.changed_types {
            println!("type_changed\t{key}");
        }
        for key in diff.only_in_left {
            println!("left_only\t{key}");
        }
        for key in diff.only_in_right {
            println!("right_only\t{key}");
        }
    } else {
        unreachable!("validated above");
    }

    Ok(ExitCode::SUCCESS)
}
