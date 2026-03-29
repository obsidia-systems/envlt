use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    bundle::{decrypt_project_bundle, encrypt_project_bundle},
    env::{parse_env_file, render_env},
    error::{EnvltError, Result},
    gen::{generate_value, GenType},
    link::{read_project_link, write_project_link},
    vault::{Project, VarType, Variable, VaultStore},
};

#[derive(Debug, Clone)]
pub struct AppService {
    store: VaultStore,
}

#[derive(Debug, Clone)]
pub struct RunEnvironment {
    pub variables: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableView {
    pub key: String,
    pub value: String,
    pub var_type: VarType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleDiff {
    pub project: String,
    pub example_path: PathBuf,
    pub missing_in_vault: Vec<String>,
    pub extra_in_vault: Vec<String>,
    pub shared_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiff {
    pub left_project: String,
    pub right_project: String,
    pub only_in_left: Vec<String>,
    pub only_in_right: Vec<String>,
    pub shared_keys: Vec<String>,
    pub changed_values: Vec<String>,
    pub changed_types: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Ok,
    Warn,
    Error,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticCheck {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DiagnosticCheck>,
}

impl DoctorReport {
    pub fn ok_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Ok)
            .count()
    }

    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Warn)
            .count()
    }

    pub fn error_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Error)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

impl AppService {
    pub fn new(store: VaultStore) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &VaultStore {
        &self.store
    }

    pub fn init_vault(&self, passphrase: &str) -> Result<()> {
        self.store.initialize(passphrase)
    }

    pub fn add_project_from_env_file(
        &self,
        project_name: &str,
        env_file_path: &Path,
        project_path: Option<PathBuf>,
        passphrase: &str,
    ) -> Result<()> {
        let variables = parse_env_file(env_file_path)?;
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

    pub fn write_project_link(&self, project_root: &Path, project_name: &str) -> Result<()> {
        write_project_link(project_root, project_name)
    }

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

    pub fn list_projects(&self, passphrase: &str) -> Result<Vec<Project>> {
        let vault = self.store.load(passphrase)?;
        Ok(vault.projects.into_values().collect())
    }

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

    pub fn export_project_bundle(
        &self,
        project_name: &str,
        vault_passphrase: &str,
        bundle_passphrase: &str,
    ) -> Result<Vec<u8>> {
        let project = self.project_snapshot(project_name, vault_passphrase)?;
        encrypt_project_bundle(&project, bundle_passphrase, env!("CARGO_PKG_VERSION"))
    }

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
            return Err(EnvltError::BundleProjectAlreadyExists {
                name: project.name.clone(),
            });
        }

        let project_name = project.name.clone();
        vault.projects.insert(project_name.clone(), project);
        vault.touch();
        self.store.save(&vault, vault_passphrase)?;
        Ok(project_name)
    }

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

    pub fn generate_value(&self, gen_type: GenType) -> String {
        generate_value(gen_type)
    }

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

    pub fn write_env_file(
        &self,
        project_name: &str,
        output_path: &Path,
        passphrase: &str,
    ) -> Result<()> {
        let variables = self.project_variables(project_name, passphrase)?;
        let content = render_env(&variables);
        fs::write(output_path, content)?;
        Ok(())
    }

    pub fn build_run_environment(
        &self,
        project_name: &str,
        passphrase: &str,
    ) -> Result<RunEnvironment> {
        let variables = self.project_variables(project_name, passphrase)?;
        Ok(RunEnvironment { variables })
    }

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
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::TempDir;

    use super::{AppService, DiagnosticSeverity, ExampleDiff, ProjectDiff, VariableView};
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
}
