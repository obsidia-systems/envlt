use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

use crate::{
    bundle::{decrypt_project_bundle, encrypt_project_bundle},
    env::{parse_env_file, parse_env_str, render_env},
    error::{EnvltError, Result},
    gen::{generate_value, GenType},
    link::{read_project_link, remove_project_link, write_project_link},
    vault::{infer_var_type, Project, VarType, Variable, VaultStore},
};

#[derive(Debug, Clone)]
/// AppService.
pub struct AppService {
    store: VaultStore,
}

#[derive(Debug, Clone)]
/// RunEnvironment.
pub struct RunEnvironment {
    /// variables.
    pub variables: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// VariableView.
pub struct VariableView {
    /// key.
    pub key: String,
    /// value.
    pub value: String,
    /// var_type.
    pub var_type: VarType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// ExampleDiff.
pub struct ExampleDiff {
    /// project.
    pub project: String,
    /// example_path.
    pub example_path: PathBuf,
    /// missing_in_vault.
    pub missing_in_vault: Vec<String>,
    /// extra_in_vault.
    pub extra_in_vault: Vec<String>,
    /// shared_keys.
    pub shared_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// ProjectDiff.
pub struct ProjectDiff {
    /// left_project.
    pub left_project: String,
    /// right_project.
    pub right_project: String,
    /// only_in_left.
    pub only_in_left: Vec<String>,
    /// only_in_right.
    pub only_in_right: Vec<String>,
    /// shared_keys.
    pub shared_keys: Vec<String>,
    /// changed_values.
    pub changed_values: Vec<String>,
    /// changed_types.
    pub changed_types: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// DiagnosticSeverity.
pub enum DiagnosticSeverity {
    /// Ok.
    Ok,
    /// Warn.
    Warn,
    /// Error.
    Error,
}

impl DiagnosticSeverity {
    /// fn as_str(self) -> &'static str {.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// DiagnosticCheck.
pub struct DiagnosticCheck {
    /// code.
    pub code: String,
    /// severity.
    pub severity: DiagnosticSeverity,
    /// detail.
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// DoctorReport.
pub struct DoctorReport {
    /// checks.
    pub checks: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// RemoveProjectResult.
pub struct RemoveProjectResult {
    /// project.
    pub project: String,
    /// removed_link.
    pub removed_link: bool,
}

impl DoctorReport {
    /// fn ok_count(&self) -> usize {.
    pub fn ok_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Ok)
            .count()
    }

    /// fn warn_count(&self) -> usize {.
    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Warn)
            .count()
    }

    /// fn error_count(&self) -> usize {.
    pub fn error_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Error)
            .count()
    }

    /// fn has_errors(&self) -> bool {.
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

impl AppService {
    /// fn new(store.
    pub fn new(store: VaultStore) -> Self {
        Self { store }
    }

    /// fn store(&self) -> &VaultStore {.
    pub fn store(&self) -> &VaultStore {
        &self.store
    }

    /// fn init_vault(&self, passphrase.
    pub fn init_vault(&self, passphrase: &str) -> Result<()> {
        self.store.initialize(passphrase)
    }

    /// fn add_project_from_env_file(.
    pub fn add_project_from_env_file(
        &self,
        project_name: &str,
        env_file_path: &Path,
        project_path: Option<PathBuf>,
        passphrase: &str,
    ) -> Result<()> {
        let variables = parse_env_file(env_file_path)?;
        self.add_project_from_variables(project_name, variables, project_path, passphrase)
    }

    /// fn add_project_from_env_str(.
    pub fn add_project_from_env_str(
        &self,
        project_name: &str,
        env_content: &str,
        project_path: Option<PathBuf>,
        passphrase: &str,
    ) -> Result<()> {
        let virtual_path = Path::new("<inline-env>");
        let variables = parse_env_str(virtual_path, env_content)?;
        self.add_project_from_variables(project_name, variables, project_path, passphrase)
    }

    /// fn missing_example_inputs(&self, example_path.
    pub fn missing_example_inputs(&self, example_path: &Path) -> Result<Vec<(String, VarType)>> {
        let variables = parse_env_file(example_path)?;
        Ok(variables
            .into_iter()
            .filter_map(|(key, value)| {
                if value.is_empty() {
                    let var_type = infer_var_type(&key);
                    Some((key, var_type))
                } else {
                    None
                }
            })
            .collect())
    }

    fn add_project_from_variables(
        &self,
        project_name: &str,
        variables: BTreeMap<String, String>,
        project_path: Option<PathBuf>,
        passphrase: &str,
    ) -> Result<()> {
        let mut vault = self.store.load(passphrase)?;

        if vault.projects.contains_key(project_name) {
            return Err(EnvltError::ProjectAlreadyExists {
                name: project_name.to_owned(),
            });
        }

        let mut project = Project::new(project_name, project_path);
        project.variables = variables
            .into_iter()
            .map(|(key, value)| {
                let variable = Variable::new(&key, value);
                (key, variable)
            })
            .collect();
        project.touch();
        vault.projects.insert(project_name.to_owned(), project);
        vault.touch();

        self.store.save(&vault, passphrase)
    }

    /// fn add_project_from_example(.
    pub fn add_project_from_example(
        &self,
        project_name: &str,
        example_path: &Path,
        project_path: Option<PathBuf>,
        overrides: &BTreeMap<String, String>,
        passphrase: &str,
    ) -> Result<()> {
        let variables = parse_env_file(example_path)?;
        let resolved_variables = variables
            .into_iter()
            .map(|(key, value)| {
                let resolved_value = if value.is_empty() {
                    overrides
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| EnvltError::MissingExampleValue { key: key.clone() })?
                } else {
                    value
                };

                Ok((key, resolved_value))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        let mut vault = self.store.load(passphrase)?;

        if vault.projects.contains_key(project_name) {
            return Err(EnvltError::ProjectAlreadyExists {
                name: project_name.to_owned(),
            });
        }

        let mut project = Project::new(project_name, project_path);
        project.variables = resolved_variables
            .into_iter()
            .map(|(key, value)| {
                let variable = Variable::new(&key, value);
                (key, variable)
            })
            .collect();
        project.touch();
        vault.projects.insert(project_name.to_owned(), project);
        vault.touch();

        self.store.save(&vault, passphrase)
    }

    /// fn write_project_link(&self, project_root.
    pub fn write_project_link(&self, project_root: &Path, project_name: &str) -> Result<()> {
        write_project_link(project_root, project_name)
    }

    /// fn remove_project(.
    pub fn remove_project(
        &self,
        project_name: &str,
        current_dir: Option<&Path>,
        passphrase: &str,
    ) -> Result<RemoveProjectResult> {
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .remove(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        vault.touch();
        self.store.save(&vault, passphrase)?;

        let removed_link = self.remove_link_if_matches(project_name, current_dir, &project)?;

        Ok(RemoveProjectResult {
            project: project_name.to_owned(),
            removed_link,
        })
    }

    /// fn resolve_project_name(.
    pub fn resolve_project_name(
        &self,
        explicit_project: Option<&str>,
        current_dir: Option<&Path>,
    ) -> Result<String> {
        if let Some(project) = explicit_project {
            return Ok(project.to_owned());
        }

        let current_dir = match current_dir {
            Some(dir) => dir.to_path_buf(),
            None => env::current_dir()?,
        };

        read_project_link(&current_dir)?
            .ok_or(EnvltError::ProjectResolutionFailed { path: current_dir })
    }

    /// fn list_projects(&self, passphrase.
    pub fn list_projects(&self, passphrase: &str) -> Result<Vec<Project>> {
        let vault = self.store.load(passphrase)?;
        Ok(vault.projects.into_values().collect())
    }

    /// fn verify_vault_access(&self, passphrase.
    pub fn verify_vault_access(&self, passphrase: &str) -> Result<()> {
        self.store.load(passphrase).map(|_| ())
    }

    /// fn project_snapshot(&self, project_name.
    pub fn project_snapshot(&self, project_name: &str, passphrase: &str) -> Result<Project> {
        let vault = self.store.load(passphrase)?;
        vault
            .projects
            .get(project_name)
            .cloned()
            .ok_or_else(|| EnvltError::ProjectNotFound {
                name: project_name.to_owned(),
            })
    }

    /// fn export_project_bundle(.
    pub fn export_project_bundle(
        &self,
        project_name: &str,
        vault_passphrase: &str,
        bundle_passphrase: &str,
    ) -> Result<Vec<u8>> {
        let project = self.project_snapshot(project_name, vault_passphrase)?;
        encrypt_project_bundle(&project, bundle_passphrase, env!("CARGO_PKG_VERSION"))
    }

    /// fn import_project_bundle(.
    pub fn import_project_bundle(
        &self,
        bundle_bytes: &[u8],
        vault_passphrase: &str,
        bundle_passphrase: &str,
        overwrite_existing: bool,
    ) -> Result<String> {
        let project = decrypt_project_bundle(bundle_bytes, bundle_passphrase)?;
        let mut vault = self.store.load(vault_passphrase)?;

        if vault.projects.contains_key(&project.name) && !overwrite_existing {
            return Err(EnvltError::BundleProjectAlreadyExists { name: project.name });
        }

        let project_name = project.name.clone();
        vault.projects.insert(project_name.clone(), project);
        vault.touch();
        self.store.save(&vault, vault_passphrase)?;
        Ok(project_name)
    }

    /// fn set_variable(.
    pub fn set_variable(
        &self,
        project_name: &str,
        key: &str,
        value: &str,
        var_type: Option<VarType>,
        passphrase: &str,
    ) -> Result<()> {
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get_mut(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        match project.variables.get_mut(key) {
            Some(variable) => {
                variable.set(value);
                if let Some(var_type) = var_type {
                    variable.set_type(var_type);
                }
            }
            None => {
                let variable = match var_type {
                    Some(var_type) => Variable::new_with_type(value.to_owned(), var_type),
                    None => Variable::new(key, value.to_owned()),
                };
                project.variables.insert(key.to_owned(), variable);
            }
        }

        project.touch();
        vault.touch();
        self.store.save(&vault, passphrase)
    }

    /// fn unset_variable(&self, project_name.
    pub fn unset_variable(&self, project_name: &str, key: &str, passphrase: &str) -> Result<()> {
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get_mut(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        let removed = project.variables.remove(key).is_some();
        if !removed {
            return Err(EnvltError::VariableNotFound {
                project: project_name.to_owned(),
                key: key.to_owned(),
            });
        }

        project.touch();
        vault.touch();
        self.store.save(&vault, passphrase)
    }

    /// fn generate_value(&self, gen_type.
    pub fn generate_value(&self, gen_type: GenType) -> String {
        generate_value(gen_type)
    }

    /// fn generate_and_store(.
    pub fn generate_and_store(
        &self,
        project_name: &str,
        key: &str,
        gen_type: GenType,
        passphrase: &str,
    ) -> Result<String> {
        let value = generate_value(gen_type);
        self.set_variable(
            project_name,
            key,
            &value,
            Some(gen_type.default_var_type()),
            passphrase,
        )?;
        Ok(value)
    }

    /// fn project_variables(.
    pub fn project_variables(
        &self,
        project_name: &str,
        passphrase: &str,
    ) -> Result<BTreeMap<String, String>> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        Ok(project
            .variables
            .iter()
            .map(|(key, variable)| (key.clone(), variable.value.clone()))
            .collect())
    }

    /// fn project_variable_views(.
    pub fn project_variable_views(
        &self,
        project_name: &str,
        passphrase: &str,
    ) -> Result<Vec<VariableView>> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        Ok(project
            .variables
            .iter()
            .map(|(key, variable)| VariableView {
                key: key.clone(),
                value: variable.value.clone(),
                var_type: variable.var_type,
            })
            .collect())
    }

    /// fn diff_project_against_example(.
    pub fn diff_project_against_example(
        &self,
        project_name: &str,
        example_path: &Path,
        passphrase: &str,
    ) -> Result<ExampleDiff> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        let example_variables = parse_env_file(example_path)?;

        let missing_in_vault = example_variables
            .keys()
            .filter(|key| !project.variables.contains_key(*key))
            .cloned()
            .collect();

        let extra_in_vault = project
            .variables
            .keys()
            .filter(|key| !example_variables.contains_key(*key))
            .cloned()
            .collect();

        let shared_keys = example_variables
            .keys()
            .filter(|key| project.variables.contains_key(*key))
            .cloned()
            .collect();

        Ok(ExampleDiff {
            project: project_name.to_owned(),
            example_path: example_path.to_path_buf(),
            missing_in_vault,
            extra_in_vault,
            shared_keys,
        })
    }

    /// fn diff_projects(.
    pub fn diff_projects(
        &self,
        left_project: &str,
        right_project: &str,
        passphrase: &str,
    ) -> Result<ProjectDiff> {
        let vault = self.store.load(passphrase)?;
        let left = vault
            .projects
            .get(left_project)
            .ok_or_else(|| EnvltError::ProjectNotFound {
                name: left_project.to_owned(),
            })?;
        let right =
            vault
                .projects
                .get(right_project)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: right_project.to_owned(),
                })?;

        let only_in_left = left
            .variables
            .keys()
            .filter(|key| !right.variables.contains_key(*key))
            .cloned()
            .collect();

        let only_in_right = right
            .variables
            .keys()
            .filter(|key| !left.variables.contains_key(*key))
            .cloned()
            .collect();

        let shared_keys = left
            .variables
            .keys()
            .filter(|key| right.variables.contains_key(*key))
            .cloned()
            .collect();

        let changed_values = left
            .variables
            .iter()
            .filter_map(|(key, left_variable)| {
                right.variables.get(key).and_then(|right_variable| {
                    if left_variable.value != right_variable.value {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        let changed_types = left
            .variables
            .iter()
            .filter_map(|(key, left_variable)| {
                right.variables.get(key).and_then(|right_variable| {
                    if left_variable.var_type != right_variable.var_type {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(ProjectDiff {
            left_project: left_project.to_owned(),
            right_project: right_project.to_owned(),
            only_in_left,
            only_in_right,
            shared_keys,
            changed_values,
            changed_types,
        })
    }

    /// fn write_env_file(.
    pub fn write_env_file(
        &self,
        project_name: &str,
        output_path: &Path,
        passphrase: &str,
    ) -> Result<()> {
        use std::io::Write;

        let content = self.render_project_env_content(project_name, passphrase)?;

        let parent = output_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(content.as_bytes())?;
        temp.flush()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = temp.as_file().metadata()?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            temp.as_file().set_permissions(permissions)?;
        }

        temp.persist(output_path).map_err(|err| err.error)?;

        Ok(())
    }

    /// fn render_project_env_content(.
    pub fn render_project_env_content(
        &self,
        project_name: &str,
        passphrase: &str,
    ) -> Result<String> {
        let variables = self.project_variables(project_name, passphrase)?;
        Ok(render_env(&variables))
    }

    /// fn build_run_environment(.
    pub fn build_run_environment(
        &self,
        project_name: &str,
        passphrase: &str,
    ) -> Result<RunEnvironment> {
        let variables = self.project_variables(project_name, passphrase)?;
        Ok(RunEnvironment { variables })
    }

    /// fn doctor(&self, current_dir.
    pub fn doctor(&self, current_dir: Option<&Path>, passphrase: Option<&str>) -> DoctorReport {
        let mut checks = Vec::new();
        let root_dir = self.store.root_dir();
        let vault_path = self.store.vault_path();
        let backup_path = self.store.backup_path();

        checks.push(DiagnosticCheck {
            code: "home".to_owned(),
            severity: if root_dir.exists() {
                DiagnosticSeverity::Ok
            } else {
                DiagnosticSeverity::Warn
            },
            detail: format!("envlt home: {}", root_dir.display()),
        });

        let vault_exists = vault_path.exists();
        checks.push(DiagnosticCheck {
            code: "vault".to_owned(),
            severity: if vault_exists {
                DiagnosticSeverity::Ok
            } else {
                DiagnosticSeverity::Warn
            },
            detail: if vault_exists {
                format!("vault found at {}", vault_path.display())
            } else {
                format!("vault not found at {}", vault_path.display())
            },
        });

        checks.push(DiagnosticCheck {
            code: "backup".to_owned(),
            severity: if backup_path.exists() {
                DiagnosticSeverity::Ok
            } else {
                DiagnosticSeverity::Warn
            },
            detail: if backup_path.exists() {
                format!("backup found at {}", backup_path.display())
            } else {
                format!("backup not found at {}", backup_path.display())
            },
        });

        let mut loaded_project_names = None;
        if vault_exists {
            match passphrase {
                Some(passphrase) => match self.store.load(passphrase) {
                    Ok(vault) => {
                        let project_names = vault.projects.keys().cloned().collect::<Vec<_>>();
                        checks.push(DiagnosticCheck {
                            code: "decrypt".to_owned(),
                            severity: DiagnosticSeverity::Ok,
                            detail: format!(
                                "vault decrypted successfully ({} projects)",
                                project_names.len()
                            ),
                        });
                        loaded_project_names = Some(project_names);
                    }
                    Err(error) => checks.push(DiagnosticCheck {
                        code: "decrypt".to_owned(),
                        severity: DiagnosticSeverity::Error,
                        detail: error.to_string(),
                    }),
                },
                None => checks.push(DiagnosticCheck {
                    code: "decrypt".to_owned(),
                    severity: DiagnosticSeverity::Warn,
                    detail: "vault exists but no passphrase was provided".to_owned(),
                }),
            }
        }

        let current_dir = match current_dir {
            Some(path) => path.to_path_buf(),
            None => match env::current_dir() {
                Ok(path) => path,
                Err(error) => {
                    checks.push(DiagnosticCheck {
                        code: "cwd".to_owned(),
                        severity: DiagnosticSeverity::Error,
                        detail: error.to_string(),
                    });
                    return DoctorReport { checks };
                }
            },
        };

        match read_project_link(&current_dir) {
            Ok(Some(project)) => {
                checks.push(DiagnosticCheck {
                    code: "link".to_owned(),
                    severity: DiagnosticSeverity::Ok,
                    detail: format!(
                        ".envlt-link points to project '{project}' in {}",
                        current_dir.display()
                    ),
                });

                if let Some(project_names) = loaded_project_names.as_ref() {
                    let severity = if project_names.iter().any(|name| name == &project) {
                        DiagnosticSeverity::Ok
                    } else {
                        DiagnosticSeverity::Error
                    };
                    let detail = if severity == DiagnosticSeverity::Ok {
                        format!("linked project '{project}' exists in the vault")
                    } else {
                        format!("linked project '{project}' was not found in the vault")
                    };
                    checks.push(DiagnosticCheck {
                        code: "link_target".to_owned(),
                        severity,
                        detail,
                    });
                }
            }
            Ok(None) => checks.push(DiagnosticCheck {
                code: "link".to_owned(),
                severity: DiagnosticSeverity::Warn,
                detail: format!("no .envlt-link found in {}", current_dir.display()),
            }),
            Err(error) => checks.push(DiagnosticCheck {
                code: "link".to_owned(),
                severity: DiagnosticSeverity::Error,
                detail: error.to_string(),
            }),
        }

        DoctorReport { checks }
    }

    fn remove_link_if_matches(
        &self,
        project_name: &str,
        current_dir: Option<&Path>,
        project: &Project,
    ) -> Result<bool> {
        let Some(link_root) = current_dir
            .map(Path::to_path_buf)
            .or_else(|| project.path.clone())
        else {
            return Ok(false);
        };

        match read_project_link(&link_root)? {
            Some(linked_project) if linked_project == project_name => {
                remove_project_link(&link_root)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use tempfile::TempDir;

    use super::{
        AppService, DiagnosticSeverity, ExampleDiff, ProjectDiff, RemoveProjectResult, VariableView,
    };
    use crate::{GenType, VarType, VaultStore};

    #[test]
    fn add_project_infers_variable_types_from_env_file() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        std::fs::write(&env_path, "API_KEY=abc123\nPORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "typed-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");

        let project = service
            .project_snapshot("typed-project", "passphrase")
            .expect("snapshot");

        assert_eq!(
            project.variables.get("API_KEY").map(|var| var.var_type),
            Some(VarType::Secret)
        );
        assert_eq!(
            project.variables.get("PORT").map(|var| var.var_type),
            Some(VarType::Config)
        );
    }

    #[test]
    fn set_variable_infers_type_for_new_entries_and_preserves_existing_type() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "typed-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");

        service
            .set_variable("typed-project", "DB_PASSWORD", "secret", None, "passphrase")
            .expect("set secret");
        service
            .set_variable("typed-project", "PORT", "4000", None, "passphrase")
            .expect("update config");

        let project = service
            .project_snapshot("typed-project", "passphrase")
            .expect("snapshot");

        assert_eq!(
            project.variables.get("DB_PASSWORD").map(|var| var.var_type),
            Some(VarType::Secret)
        );
        assert_eq!(
            project.variables.get("PORT").map(|var| var.var_type),
            Some(VarType::Config)
        );
    }

    #[test]
    fn set_variable_can_override_existing_type_explicitly() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "typed-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");

        service
            .set_variable(
                "typed-project",
                "PORT",
                "4000",
                Some(VarType::Secret),
                "passphrase",
            )
            .expect("override type");

        let project = service
            .project_snapshot("typed-project", "passphrase")
            .expect("snapshot");

        assert_eq!(
            project.variables.get("PORT").map(|var| var.var_type),
            Some(VarType::Secret)
        );
    }

    #[test]
    fn add_project_from_example_uses_defaults_and_overrides_missing_values() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let example_path = project_dir.path().join(".env.example");

        std::fs::write(&example_path, "PORT=3000\nAPI_KEY=\n").expect("write example");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);
        let overrides = BTreeMap::from([("API_KEY".to_owned(), "abc123".to_owned())]);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_example(
                "example-project",
                &example_path,
                Some(project_dir.path().to_path_buf()),
                &overrides,
                "passphrase",
            )
            .expect("add project from example");

        let project = service
            .project_snapshot("example-project", "passphrase")
            .expect("snapshot");

        assert_eq!(
            project.variables.get("PORT").map(|var| var.value.as_str()),
            Some("3000")
        );
        assert_eq!(
            project
                .variables
                .get("API_KEY")
                .map(|var| var.value.as_str()),
            Some("abc123")
        );
        assert_eq!(
            project.variables.get("API_KEY").map(|var| var.var_type),
            Some(VarType::Secret)
        );
    }

    #[test]
    fn project_variable_views_returns_sorted_values_with_types() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        std::fs::write(&env_path, "API_KEY=abc123\nPORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "typed-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");

        let views = service
            .project_variable_views("typed-project", "passphrase")
            .expect("variable views");

        assert_eq!(
            views,
            vec![
                VariableView {
                    key: "API_KEY".to_owned(),
                    value: "abc123".to_owned(),
                    var_type: VarType::Secret,
                },
                VariableView {
                    key: "PORT".to_owned(),
                    value: "3000".to_owned(),
                    var_type: VarType::Config,
                },
            ]
        );
    }

    #[test]
    fn diff_project_against_example_reports_missing_extra_and_shared_keys() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");
        let example_path = project_dir.path().join(".env.example");

        std::fs::write(&env_path, "PORT=3000\nAPI_KEY=abc123\nLOCAL_ONLY=1\n").expect("write env");
        std::fs::write(&example_path, "PORT=\nAPI_KEY=\nREQUIRED_KEY=\n").expect("write example");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "diff-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");

        let diff = service
            .diff_project_against_example("diff-project", &example_path, "passphrase")
            .expect("diff");

        assert_eq!(
            diff,
            ExampleDiff {
                project: "diff-project".to_owned(),
                example_path,
                missing_in_vault: vec!["REQUIRED_KEY".to_owned()],
                extra_in_vault: vec!["LOCAL_ONLY".to_owned()],
                shared_keys: vec!["API_KEY".to_owned(), "PORT".to_owned()],
            }
        );
    }

    #[test]
    fn generate_and_store_writes_secret_variable_to_project() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "gen-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");

        let value = service
            .generate_and_store(
                "gen-project",
                "JWT_SECRET",
                GenType::JwtSecret,
                "passphrase",
            )
            .expect("generate and store");

        assert_eq!(value.len(), 64);

        let project = service
            .project_snapshot("gen-project", "passphrase")
            .expect("snapshot");
        let variable = project
            .variables
            .get("JWT_SECRET")
            .expect("generated variable");

        assert_eq!(variable.value, value);
        assert_eq!(variable.var_type, VarType::Secret);
    }

    #[test]
    fn diff_projects_reports_shared_and_unique_keys() {
        let home = TempDir::new().expect("tempdir");
        let left_dir = TempDir::new().expect("tempdir");
        let right_dir = TempDir::new().expect("tempdir");
        let left_env_path = left_dir.path().join(".env");
        let right_env_path = right_dir.path().join(".env");

        std::fs::write(&left_env_path, "PORT=3000\nLEFT_ONLY=1\nSHARED=yes\n")
            .expect("write left env");
        std::fs::write(&right_env_path, "PORT=4000\nRIGHT_ONLY=1\nSHARED=yes\n")
            .expect("write right env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "left-project",
                &left_env_path,
                Some(left_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add left project");
        service
            .add_project_from_env_file(
                "right-project",
                &right_env_path,
                Some(right_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add right project");

        let diff = service
            .diff_projects("left-project", "right-project", "passphrase")
            .expect("project diff");

        assert_eq!(
            diff,
            ProjectDiff {
                left_project: "left-project".to_owned(),
                right_project: "right-project".to_owned(),
                only_in_left: vec!["LEFT_ONLY".to_owned()],
                only_in_right: vec!["RIGHT_ONLY".to_owned()],
                shared_keys: vec!["PORT".to_owned(), "SHARED".to_owned()],
                changed_values: vec!["PORT".to_owned()],
                changed_types: vec![],
            }
        );
    }

    #[test]
    fn diff_projects_reports_changed_variable_types() {
        let home = TempDir::new().expect("tempdir");
        let left_dir = TempDir::new().expect("tempdir");
        let right_dir = TempDir::new().expect("tempdir");
        let left_env_path = left_dir.path().join(".env");
        let right_env_path = right_dir.path().join(".env");

        std::fs::write(&left_env_path, "API_TOKEN=same\n").expect("write left env");
        std::fs::write(&right_env_path, "API_TOKEN=same\n").expect("write right env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "left-project",
                &left_env_path,
                Some(left_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add left project");
        service
            .add_project_from_env_file(
                "right-project",
                &right_env_path,
                Some(right_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add right project");

        service
            .set_variable(
                "right-project",
                "API_TOKEN",
                "same",
                Some(VarType::Plain),
                "passphrase",
            )
            .expect("retag variable");

        let diff = service
            .diff_projects("left-project", "right-project", "passphrase")
            .expect("project diff");

        assert_eq!(
            diff,
            ProjectDiff {
                left_project: "left-project".to_owned(),
                right_project: "right-project".to_owned(),
                only_in_left: vec![],
                only_in_right: vec![],
                shared_keys: vec!["API_TOKEN".to_owned()],
                changed_values: vec![],
                changed_types: vec!["API_TOKEN".to_owned()],
            }
        );
    }

    #[test]
    fn remove_project_deletes_project_and_matching_link() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "remove-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");
        service
            .write_project_link(project_dir.path(), "remove-project")
            .expect("write link");

        let result = service
            .remove_project("remove-project", Some(project_dir.path()), "passphrase")
            .expect("remove project");

        assert_eq!(
            result,
            RemoveProjectResult {
                project: "remove-project".to_owned(),
                removed_link: true,
            }
        );
        assert!(service
            .project_snapshot("remove-project", "passphrase")
            .is_err());
        assert!(!project_dir.path().join(".envlt-link").exists());
    }

    #[test]
    fn remove_project_keeps_unrelated_link() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");

        fs::write(&env_path, "PORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "remove-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");
        service
            .write_project_link(project_dir.path(), "other-project")
            .expect("write link");

        let result = service
            .remove_project("remove-project", Some(project_dir.path()), "passphrase")
            .expect("remove project");

        assert_eq!(
            result,
            RemoveProjectResult {
                project: "remove-project".to_owned(),
                removed_link: false,
            }
        );
        assert!(project_dir.path().join(".envlt-link").exists());
    }

    #[test]
    fn doctor_reports_missing_vault_as_warning_without_errors() {
        let home = TempDir::new().expect("tempdir");
        let service = AppService::new(VaultStore::new(home.path().to_path_buf()));

        let report = service.doctor(Some(home.path()), None);

        assert_eq!(report.error_count(), 0);
        assert!(report.warn_count() >= 2);
        assert!(!report.has_errors());
    }

    #[test]
    fn doctor_reports_link_target_error_when_project_is_missing() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let service = AppService::new(VaultStore::new(home.path().to_path_buf()));

        service.init_vault("passphrase").expect("init");
        service
            .write_project_link(project_dir.path(), "ghost-project")
            .expect("write link");

        let report = service.doctor(Some(project_dir.path()), Some("passphrase"));

        assert!(report.has_errors());
        assert!(report.checks.iter().any(|check| {
            check.code == "link_target" && check.severity == DiagnosticSeverity::Error
        }));
    }

    #[test]
    fn doctor_reports_successful_decrypt_and_existing_link_target() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");
        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let service = AppService::new(VaultStore::new(home.path().to_path_buf()));
        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "doctor-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");
        service
            .write_project_link(project_dir.path(), "doctor-project")
            .expect("write project link");

        let report = service.doctor(Some(project_dir.path()), Some("passphrase"));

        assert_eq!(report.error_count(), 0);
        assert!(!report.has_errors());
        assert!(report.ok_count() >= 4);
        assert!(report
            .checks
            .iter()
            .any(|check| check.code == "decrypt" && check.severity == DiagnosticSeverity::Ok));
        assert!(report.checks.iter().any(|check| {
            check.code == "link_target"
                && check.severity == DiagnosticSeverity::Ok
                && check.detail.contains("doctor-project")
        }));
    }

    #[test]
    #[cfg(unix)]
    fn write_env_file_uses_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let home = TempDir::new().expect("tempdir");
        let output_dir = TempDir::new().expect("tempdir");
        let env_path = output_dir.path().join(".env");

        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file("perm-project", &env_path, None, "passphrase")
            .expect("add project");

        let output_env = output_dir.path().join("output.env");
        service
            .write_env_file("perm-project", &output_env, "passphrase")
            .expect("write env file");

        let metadata = std::fs::metadata(&output_env).expect("metadata");
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
}
