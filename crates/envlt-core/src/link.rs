use std::{
    fs,
    path::{Path, PathBuf},
};

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
    /// Which environment this working directory defaults to. `None` for
    /// links written before environments existed, or by commands that
    /// haven't been taught to set it yet -- callers fall back to
    /// [`crate::vault::DEFAULT_ENVIRONMENT`] in that case.
    #[serde(default)]
    environment: Option<String>,
}

/// Write a `.envlt-link` file in `project_root` pointing to `project_name`.
pub fn write_project_link(project_root: &Path, project_name: &str) -> Result<()> {
    let link_path = project_root.join(LINK_FILE_NAME);
    let link = ProjectLink {
        project: project_name.to_owned(),
        envlt_version: format!("{}.0", VAULT_VERSION),
        environment: None,
    };
    let content = toml::to_string(&link)?;
    fs::write(link_path, content)?;
    Ok(())
}

/// Read the linked project name and, if set, environment from
/// `.envlt-link` in `project_root`.
pub fn read_project_link(project_root: &Path) -> Result<Option<(String, Option<String>)>> {
    let link_path = project_root.join(LINK_FILE_NAME);
    if !link_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&link_path)?;
    let link: ProjectLink = toml::from_str(&content).map_err(|err| EnvltError::LinkParse {
        path: link_path.clone(),
        message: err.to_string(),
    })?;
    Ok(Some((link.project, link.environment)))
}

/// Find the nearest `.envlt-link`, starting at `start_dir` and walking up
/// through its ancestors until one is found, similar to how `.git` is
/// resolved from a subdirectory of a repository.
///
/// Returns the directory the link was found in together with the linked
/// project name and environment, so callers that need to remove or
/// otherwise act on the link file operate on its real location rather than
/// `start_dir`.
pub fn find_project_link(start_dir: &Path) -> Result<Option<(PathBuf, String, Option<String>)>> {
    let mut dir = Some(start_dir);
    while let Some(current) = dir {
        if let Some((project, environment)) = read_project_link(current)? {
            return Ok(Some((current.to_path_buf(), project, environment)));
        }
        dir = current.parent();
    }
    Ok(None)
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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn find_project_link_matches_in_the_exact_directory() {
        let temp = TempDir::new().expect("tempdir");
        write_project_link(temp.path(), "same-dir-project").expect("write link");

        let (link_dir, project, environment) = find_project_link(temp.path())
            .expect("find link")
            .expect("link found");

        assert_eq!(link_dir, temp.path());
        assert_eq!(project, "same-dir-project");
        assert_eq!(environment, None);
    }

    #[test]
    fn find_project_link_walks_up_through_parent_directories() {
        let temp = TempDir::new().expect("tempdir");
        write_project_link(temp.path(), "parent-project").expect("write link");

        let nested = temp.path().join("src").join("nested");
        fs::create_dir_all(&nested).expect("create nested dirs");

        let (link_dir, project, _environment) = find_project_link(&nested)
            .expect("find link")
            .expect("link found");

        assert_eq!(link_dir, temp.path());
        assert_eq!(project, "parent-project");
    }

    #[test]
    fn find_project_link_prefers_the_closest_link() {
        let temp = TempDir::new().expect("tempdir");
        write_project_link(temp.path(), "outer-project").expect("write outer link");

        let inner = temp.path().join("inner");
        fs::create_dir_all(&inner).expect("create inner dir");
        write_project_link(&inner, "inner-project").expect("write inner link");

        let (link_dir, project, _environment) = find_project_link(&inner)
            .expect("find link")
            .expect("link found");

        assert_eq!(link_dir, inner);
        assert_eq!(project, "inner-project");
    }

    #[test]
    fn find_project_link_returns_none_when_no_ancestor_has_a_link() {
        let temp = TempDir::new().expect("tempdir");
        let nested = temp.path().join("a").join("b");
        fs::create_dir_all(&nested).expect("create nested dirs");

        assert_eq!(find_project_link(&nested).expect("find link"), None);
    }
}
