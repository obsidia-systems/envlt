use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    error::{EnvltError, Result},
    vault::VAULT_VERSION,
};

const LINK_FILE_NAME: &str = ".envlt-link";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectLink {
    project: String,
    envlt_version: String,
}

/// Write a `.envlt-link` file in `project_root` pointing to `project_name`.
pub fn write_project_link(project_root: &Path, project_name: &str) -> Result<()> {
    let link_path = project_root.join(LINK_FILE_NAME);
    let link = ProjectLink {
        project: project_name.to_owned(),
        envlt_version: format!("{}.0", VAULT_VERSION),
    };
    let content = toml::to_string(&link)?;
    fs::write(link_path, content)?;
    Ok(())
}

/// Read the linked project name from `.envlt-link` in `project_root`, if present.
pub fn read_project_link(project_root: &Path) -> Result<Option<String>> {
    let link_path = project_root.join(LINK_FILE_NAME);
    if !link_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&link_path)?;
    let link: ProjectLink = toml::from_str(&content).map_err(|err| EnvltError::LinkParse {
        path: link_path.clone(),
        message: err.to_string(),
    })?;
    Ok(Some(link.project))
}

/// Remove `.envlt-link` from `project_root` if it exists.
pub fn remove_project_link(project_root: &Path) -> Result<bool> {
    let link_path = project_root.join(LINK_FILE_NAME);
    if !link_path.exists() {
        return Ok(false);
    }

    fs::remove_file(link_path)?;
    Ok(true)
}
