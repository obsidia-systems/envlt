use std::{
    env,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::Result;
use envlt_core::{infer_var_type, parse_env_file, AppService};

use crate::cli::{print_success, read_example_value, read_passphrase};

pub fn run_add(
    service: &AppService,
    project: &str,
    file: &Path,
    from_example: &Option<PathBuf>,
    project_path: Option<PathBuf>,
) -> Result<ExitCode> {
    let passphrase = read_passphrase(false)?;
    let cwd = env::current_dir()?;
    let project_root = project_path.unwrap_or(cwd);

    match from_example {
        Some(example_path) => {
            let example_variables = parse_env_file(example_path)?;
            let mut overrides = std::collections::BTreeMap::new();

            for (key, value) in example_variables {
                if value.is_empty() {
                    let inferred_type = infer_var_type(&key);
                    let resolved = read_example_value(&key, inferred_type)?;
                    overrides.insert(key, resolved);
                }
            }

            service.add_project_from_example(
                project,
                example_path,
                Some(project_root.clone()),
                &overrides,
                &passphrase,
            )?;
        }
        None => service.add_project_from_env_file(
            project,
            file,
            Some(project_root.clone()),
            &passphrase,
        )?,
    }

    service.write_project_link(&project_root, project)?;
    print_success("Project imported into vault.")?;
    Ok(ExitCode::SUCCESS)
}
