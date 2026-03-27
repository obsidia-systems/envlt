use std::{collections::BTreeMap, fs, path::Path};

use crate::error::{EnvltError, Result};

pub fn parse_env_file(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path)?;
    parse_env_str(path, &content)
}

pub fn parse_env_str(path: &Path, content: &str) -> Result<BTreeMap<String, String>> {
    let mut variables = BTreeMap::new();

    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = line.split_once('=').ok_or_else(|| EnvltError::EnvParse {
            path: path.to_path_buf(),
            message: format!("line {} is missing '='", index + 1),
        })?;

        let key = key.trim();
        if key.is_empty() {
            return Err(EnvltError::EnvParse {
                path: path.to_path_buf(),
                message: format!("line {} has an empty key", index + 1),
            });
        }

        variables.insert(key.to_owned(), value.to_owned());
    }

    Ok(variables)
}
