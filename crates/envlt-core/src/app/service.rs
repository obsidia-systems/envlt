use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::{
    bundle::{decrypt_project_bundle, encrypt_project_bundle},
    env::{parse_env_file, parse_env_str, render_env},
    error::{EnvltError, Result},
    gen::{generate_value, GenType},
    link::{
        find_project_link, remove_project_link, write_project_link,
        write_project_link_with_environment,
    },
    vault::{
        infer_var_type, synthesize_variable_events, ActivityEvent, Environment, Project, VarType,
        Variable, VaultStore, DEFAULT_ENVIRONMENT,
    },
};

/// Facade over a [`VaultStore`] exposing project, environment, and variable
/// operations to the CLI.
#[derive(Debug, Clone)]
pub struct AppService {
    store: VaultStore,
}

/// A resolved set of variables ready to inject into a child process.
#[derive(Debug, Clone)]
pub struct RunEnvironment {
    /// Variable name to current value.
    pub variables: BTreeMap<String, String>,
}

/// One variable's current state, as displayed by `envlt vars`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableView {
    /// Variable key.
    pub key: String,
    /// Current value.
    pub value: String,
    /// Current classification.
    pub var_type: VarType,
    /// When the current value was written.
    pub updated_at: DateTime<Utc>,
}

/// Difference between a project's stored variables and an `.env.example`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleDiff {
    /// Project the comparison was run against.
    pub project: String,
    /// Path to the example file.
    pub example_path: PathBuf,
    /// Keys present in the example but missing from the vault.
    pub missing_in_vault: Vec<String>,
    /// Keys present in the vault but not in the example.
    pub extra_in_vault: Vec<String>,
    /// Keys present in both.
    pub shared_keys: Vec<String>,
}

/// Difference between two environments' variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiff {
    /// Left-hand project name.
    pub left_project: String,
    /// Right-hand project name.
    pub right_project: String,
    /// Keys only present on the left.
    pub only_in_left: Vec<String>,
    /// Keys only present on the right.
    pub only_in_right: Vec<String>,
    /// Keys present on both sides.
    pub shared_keys: Vec<String>,
    /// Shared keys whose values differ.
    pub changed_values: Vec<String>,
    /// Shared keys whose classification differs.
    pub changed_types: Vec<String>,
}

/// Severity of a single [`DoctorReport`] check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Everything looks fine.
    Ok,
    /// Worth a look, but not blocking.
    Warn,
    /// Something is broken.
    Error,
}

impl DiagnosticSeverity {
    /// The lowercase string used when rendering this severity.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One diagnostic line in a [`DoctorReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticCheck {
    /// Short machine-readable identifier, e.g. `"vault"` or `"link"`.
    pub code: String,
    /// How serious this check's finding is.
    pub severity: DiagnosticSeverity,
    /// Human-readable detail.
    pub detail: String,
}

/// Full result of `envlt doctor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// All checks that were run, in the order they ran.
    pub checks: Vec<DiagnosticCheck>,
}

/// Result of removing a project, including whether a matching
/// `.envlt-link` was cleaned up alongside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveProjectResult {
    /// Name of the removed project.
    pub project: String,
    /// Whether a `.envlt-link` pointing at it was also removed.
    pub removed_link: bool,
}

impl DoctorReport {
    /// Number of checks that passed.
    pub fn ok_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Ok)
            .count()
    }

    /// Number of checks that warned.
    pub fn warn_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Warn)
            .count()
    }

    /// Number of checks that failed.
    pub fn error_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.severity == DiagnosticSeverity::Error)
            .count()
    }

    /// Whether any check failed.
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

/// Look up an environment by name, or `EnvironmentNotFound`.
fn require_environment<'a>(
    project: &'a Project,
    project_name: &str,
    environment_name: &str,
) -> Result<&'a Environment> {
    project
        .environment(environment_name)
        .ok_or_else(|| EnvltError::EnvironmentNotFound {
            project: project_name.to_owned(),
            name: environment_name.to_owned(),
        })
}

/// Look up an environment by name mutably, or `EnvironmentNotFound`.
fn require_environment_mut<'a>(
    project: &'a mut Project,
    project_name: &str,
    environment_name: &str,
) -> Result<&'a mut Environment> {
    project
        .environment_mut(environment_name)
        .ok_or_else(|| EnvltError::EnvironmentNotFound {
            project: project_name.to_owned(),
            name: environment_name.to_owned(),
        })
}

/// Merge `incoming`'s variables into `target`, recording a new version for
/// each variable that is new, changed, or reviving a deleted one, so the
/// merge behaves like repeated `set_variable` calls rather than a silent
/// overwrite that would discard `target`'s existing history.
fn merge_environment(target: &mut Environment, incoming: &Environment, max_versions: usize) {
    for (key, incoming_variable) in &incoming.variables {
        match target.variables.get_mut(key) {
            Some(existing_variable) => {
                let value_changed = existing_variable.value() != incoming_variable.value();
                let type_changed = existing_variable.var_type() != incoming_variable.var_type();
                if value_changed || type_changed || existing_variable.is_deleted() {
                    existing_variable.record(
                        incoming_variable.value(),
                        incoming_variable.var_type(),
                        max_versions,
                    );
                }
            }
            None => {
                target.variables.insert(
                    key.clone(),
                    Variable::new_with_type(
                        incoming_variable.value().to_owned(),
                        incoming_variable.var_type(),
                    ),
                );
            }
        }
    }
    target.touch();
}

impl AppService {
    /// Wrap a [`VaultStore`] in an [`AppService`].
    pub fn new(store: VaultStore) -> Self {
        Self { store }
    }

    /// The underlying vault store.
    pub fn store(&self) -> &VaultStore {
        &self.store
    }

    /// Initialize a new, empty vault protected by `passphrase`.
    pub fn init_vault(&self, passphrase: &str) -> Result<()> {
        let _lock = self.store.lock()?;
        self.store.initialize(passphrase)
    }

    /// Create a project from the variables in an `.env` file, seeded into
    /// [`DEFAULT_ENVIRONMENT`].
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

    /// Create a project from an in-memory `.env`-formatted string, seeded
    /// into [`DEFAULT_ENVIRONMENT`].
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

    /// Keys in `example_path` left empty, paired with their inferred type,
    /// i.e. the inputs a caller must supply to satisfy the example.
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
        let _lock = self.store.lock()?;
        let mut vault = self.store.load(passphrase)?;

        if vault.projects.contains_key(project_name) {
            return Err(EnvltError::ProjectAlreadyExists {
                name: project_name.to_owned(),
            });
        }

        let mut project = Project::new(project_name, project_path);
        let mut environment = Environment::new(DEFAULT_ENVIRONMENT);
        environment.variables = variables
            .into_iter()
            .map(|(key, value)| {
                let variable = Variable::new(&key, value);
                (key, variable)
            })
            .collect();
        project
            .environments
            .insert(DEFAULT_ENVIRONMENT.to_owned(), environment);
        project.touch();
        vault.projects.insert(project_name.to_owned(), project);
        vault.touch();

        self.store.save(&vault, passphrase)
    }

    /// Create a project from an `.env.example` template, filling blank
    /// entries from `overrides`, seeded into [`DEFAULT_ENVIRONMENT`].
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

        let _lock = self.store.lock()?;
        let mut vault = self.store.load(passphrase)?;

        if vault.projects.contains_key(project_name) {
            return Err(EnvltError::ProjectAlreadyExists {
                name: project_name.to_owned(),
            });
        }

        let mut project = Project::new(project_name, project_path);
        let mut environment = Environment::new(DEFAULT_ENVIRONMENT);
        environment.variables = resolved_variables
            .into_iter()
            .map(|(key, value)| {
                let variable = Variable::new(&key, value);
                (key, variable)
            })
            .collect();
        project
            .environments
            .insert(DEFAULT_ENVIRONMENT.to_owned(), environment);
        project.touch();
        vault.projects.insert(project_name.to_owned(), project);
        vault.touch();

        self.store.save(&vault, passphrase)
    }

    /// Write a `.envlt-link` in `project_root` pointing at `project_name`.
    pub fn write_project_link(&self, project_root: &Path, project_name: &str) -> Result<()> {
        write_project_link(project_root, project_name)
    }

    /// Remove a project from the vault and, if `current_dir` (or the
    /// project's own recorded path) has a matching `.envlt-link`, remove
    /// that too.
    pub fn remove_project(
        &self,
        project_name: &str,
        current_dir: Option<&Path>,
        passphrase: &str,
    ) -> Result<RemoveProjectResult> {
        let _lock = self.store.lock()?;
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

    /// Resolve the project to operate on: an explicit `--project` flag
    /// takes priority, otherwise fall back to the nearest `.envlt-link`.
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

        find_project_link(&current_dir)?
            .map(|(_link_dir, project, _environment)| project)
            .ok_or(EnvltError::ProjectResolutionFailed { path: current_dir })
    }

    /// Resolve the environment to operate on: an explicit `--env` flag
    /// takes priority, then the environment recorded on `.envlt-link`,
    /// then [`DEFAULT_ENVIRONMENT`]. Existence is validated at the point of
    /// use, not here.
    pub fn resolve_environment_name(
        explicit: Option<&str>,
        link_environment: Option<&str>,
    ) -> String {
        explicit
            .or(link_environment)
            .unwrap_or(DEFAULT_ENVIRONMENT)
            .to_owned()
    }

    /// List every project in the vault.
    pub fn list_projects(&self, passphrase: &str) -> Result<Vec<Project>> {
        let vault = self.store.load(passphrase)?;
        Ok(vault.projects.into_values().collect())
    }

    /// Confirm `passphrase` unlocks the vault, without returning its data.
    pub fn verify_vault_access(&self, passphrase: &str) -> Result<()> {
        self.store.load(passphrase).map(|_| ())
    }

    /// A full copy of one project, all environments included.
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

    /// Add a new, empty environment to a project.
    pub fn add_environment(
        &self,
        project_name: &str,
        environment_name: &str,
        passphrase: &str,
    ) -> Result<()> {
        let _lock = self.store.lock()?;
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get_mut(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        if project.environments.contains_key(environment_name) {
            return Err(EnvltError::EnvironmentAlreadyExists {
                project: project_name.to_owned(),
                name: environment_name.to_owned(),
            });
        }

        project.environments.insert(
            environment_name.to_owned(),
            Environment::new(environment_name),
        );
        project.touch();
        vault.touch();
        self.store.save(&vault, passphrase)
    }

    /// List a project's environment names.
    pub fn list_environments(&self, project_name: &str, passphrase: &str) -> Result<Vec<String>> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        Ok(project.environments.keys().cloned().collect())
    }

    /// Remove an environment and everything in it (variables and their
    /// history). Every project must keep at least one environment, so
    /// removing the last one is an error rather than leaving the project
    /// with none.
    pub fn remove_environment(
        &self,
        project_name: &str,
        environment_name: &str,
        passphrase: &str,
    ) -> Result<()> {
        let _lock = self.store.lock()?;
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get_mut(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;

        if !project.environments.contains_key(environment_name) {
            return Err(EnvltError::EnvironmentNotFound {
                project: project_name.to_owned(),
                name: environment_name.to_owned(),
            });
        }
        if project.environments.len() <= 1 {
            return Err(EnvltError::CannotRemoveLastEnvironment {
                project: project_name.to_owned(),
                name: environment_name.to_owned(),
            });
        }

        project.environments.remove(environment_name);
        project.touch();
        vault.touch();
        self.store.save(&vault, passphrase)
    }

    /// Pin `environment_name` as `project_root`'s default environment by
    /// writing it into `.envlt-link`, after confirming the environment
    /// actually exists (so a typo doesn't silently link to nothing).
    pub fn use_environment(
        &self,
        project_name: &str,
        environment_name: &str,
        project_root: &Path,
        passphrase: &str,
    ) -> Result<()> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        require_environment(project, project_name, environment_name)?;

        write_project_link_with_environment(project_root, project_name, Some(environment_name))
    }

    /// Reconstruct the full change history for every variable in one
    /// environment, oldest first.
    pub fn project_activity_log(
        &self,
        project_name: &str,
        environment_name: &str,
        passphrase: &str,
    ) -> Result<Vec<ActivityEvent>> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        let environment = require_environment(project, project_name, environment_name)?;

        let mut events: Vec<ActivityEvent> = environment
            .variables
            .iter()
            .flat_map(|(key, variable)| synthesize_variable_events(key, variable))
            .collect();
        events.sort_by_key(|event| event.timestamp);
        Ok(events)
    }

    /// Reconstruct the change history for a single variable.
    pub fn variable_history(
        &self,
        project_name: &str,
        environment_name: &str,
        key: &str,
        passphrase: &str,
    ) -> Result<Vec<ActivityEvent>> {
        let vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        let environment = require_environment(project, project_name, environment_name)?;

        Ok(environment
            .variables
            .get(key)
            .map(|variable| synthesize_variable_events(key, variable))
            .unwrap_or_default())
    }

    /// Export one environment as an encrypted, portable `.evlt` bundle.
    ///
    /// The bundle carries a flattened copy of the environment (current
    /// values only, no version history, no soft-deleted variables) so that
    /// sharing it can't leak more than the environment's present state.
    pub fn export_project_bundle(
        &self,
        project_name: &str,
        environment_name: &str,
        vault_passphrase: &str,
        bundle_passphrase: &str,
    ) -> Result<Vec<u8>> {
        let vault = self.store.load(vault_passphrase)?;
        let project =
            vault
                .projects
                .get(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        let environment = require_environment(project, project_name, environment_name)?;

        let mut shadow = Project::new(project_name, project.path.clone());
        let mut shadow_environment = Environment::new(environment_name);
        shadow_environment.variables = environment
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .map(|(key, variable)| {
                (
                    key.clone(),
                    Variable::new_with_type(variable.value().to_owned(), variable.var_type()),
                )
            })
            .collect();
        shadow
            .environments
            .insert(environment_name.to_owned(), shadow_environment);

        encrypt_project_bundle(
            &shadow,
            environment_name,
            bundle_passphrase,
            env!("CARGO_PKG_VERSION"),
        )
    }

    /// Import a bundle produced by [`AppService::export_project_bundle`].
    ///
    /// If the project already exists, its matching environment (created if
    /// absent) is merged with the incoming one via [`merge_environment`],
    /// preserving existing version history rather than overwriting it.
    /// Otherwise a new project is created holding just that environment.
    pub fn import_project_bundle(
        &self,
        bundle_bytes: &[u8],
        vault_passphrase: &str,
        bundle_passphrase: &str,
        overwrite_existing: bool,
    ) -> Result<String> {
        let bundle_project = decrypt_project_bundle(bundle_bytes, bundle_passphrase)?;
        let project_name = bundle_project.name.clone();
        let (environment_name, bundle_environment) = bundle_project
            .environments
            .into_iter()
            .next()
            .ok_or(EnvltError::InvalidBundlePayload)?;

        let _lock = self.store.lock()?;
        let max_versions = self.store.config()?.max_versions;
        let mut vault = self.store.load(vault_passphrase)?;

        match vault.projects.get_mut(&project_name) {
            Some(existing) => {
                if !overwrite_existing {
                    return Err(EnvltError::BundleProjectAlreadyExists { name: project_name });
                }

                let environment = existing
                    .environments
                    .entry(environment_name.clone())
                    .or_insert_with(|| Environment::new(environment_name.clone()));
                merge_environment(environment, &bundle_environment, max_versions);
                existing.touch();
            }
            None => {
                let mut project = Project::new(project_name.clone(), None);
                let mut environment = Environment::new(environment_name.clone());
                merge_environment(&mut environment, &bundle_environment, max_versions);
                project.environments.insert(environment_name, environment);
                vault.projects.insert(project_name.clone(), project);
            }
        }

        vault.touch();
        self.store.save(&vault, vault_passphrase)?;
        Ok(project_name)
    }

    /// Set a variable's value (and optionally its type) in one environment,
    /// appending a new version. A no-op call (same value, same type, not
    /// reviving a deleted variable) records nothing.
    pub fn set_variable(
        &self,
        project_name: &str,
        environment_name: &str,
        key: &str,
        value: &str,
        var_type: Option<VarType>,
        passphrase: &str,
    ) -> Result<()> {
        let _lock = self.store.lock()?;
        let max_versions = self.store.config()?.max_versions;
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get_mut(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        let environment = require_environment_mut(project, project_name, environment_name)?;

        match environment.variables.get_mut(key) {
            Some(variable) => {
                let resolved_type = var_type.unwrap_or_else(|| variable.var_type());
                let value_changed = variable.value() != value;
                let type_changed = resolved_type != variable.var_type();
                if value_changed || type_changed || variable.is_deleted() {
                    variable.record(value, resolved_type, max_versions);
                }
            }
            None => {
                let variable = match var_type {
                    Some(var_type) => Variable::new_with_type(value.to_owned(), var_type),
                    None => Variable::new(key, value.to_owned()),
                };
                environment.variables.insert(key.to_owned(), variable);
            }
        }

        environment.touch();
        project.touch();
        vault.touch();
        self.store.save(&vault, passphrase)
    }

    /// Tombstone a variable in one environment. Its version history is
    /// kept for `envlt history`; unsetting an already-deleted (or never
    /// existing) key is an error.
    pub fn unset_variable(
        &self,
        project_name: &str,
        environment_name: &str,
        key: &str,
        passphrase: &str,
    ) -> Result<()> {
        let _lock = self.store.lock()?;
        let mut vault = self.store.load(passphrase)?;
        let project =
            vault
                .projects
                .get_mut(project_name)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: project_name.to_owned(),
                })?;
        let environment = require_environment_mut(project, project_name, environment_name)?;

        let variable =
            environment
                .variables
                .get_mut(key)
                .ok_or_else(|| EnvltError::VariableNotFound {
                    project: project_name.to_owned(),
                    key: key.to_owned(),
                })?;

        if variable.is_deleted() {
            return Err(EnvltError::VariableNotFound {
                project: project_name.to_owned(),
                key: key.to_owned(),
            });
        }

        variable.mark_deleted();
        environment.touch();
        project.touch();
        vault.touch();
        self.store.save(&vault, passphrase)
    }

    /// Generate a value without storing it.
    pub fn generate_value(&self, gen_type: GenType) -> String {
        generate_value(gen_type)
    }

    /// Generate a value and store it as a new variable.
    pub fn generate_and_store(
        &self,
        project_name: &str,
        environment_name: &str,
        key: &str,
        gen_type: GenType,
        passphrase: &str,
    ) -> Result<String> {
        let value = generate_value(gen_type);
        self.set_variable(
            project_name,
            environment_name,
            key,
            &value,
            Some(gen_type.default_var_type()),
            passphrase,
        )?;
        Ok(value)
    }

    /// Current (non-deleted) variables in one environment.
    pub fn project_variables(
        &self,
        project_name: &str,
        environment_name: &str,
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
        let environment = require_environment(project, project_name, environment_name)?;

        Ok(environment
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .map(|(key, variable)| (key.clone(), variable.value().to_owned()))
            .collect())
    }

    /// Current (non-deleted) variables in one environment, with type and
    /// last-updated metadata.
    pub fn project_variable_views(
        &self,
        project_name: &str,
        environment_name: &str,
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
        let environment = require_environment(project, project_name, environment_name)?;

        Ok(environment
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .map(|(key, variable)| VariableView {
                key: key.clone(),
                value: variable.value().to_owned(),
                var_type: variable.var_type(),
                updated_at: variable.updated_at(),
            })
            .collect())
    }

    /// Compare one environment's current variables against an
    /// `.env.example` template.
    pub fn diff_project_against_example(
        &self,
        project_name: &str,
        environment_name: &str,
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
        let environment = require_environment(project, project_name, environment_name)?;
        let example_variables = parse_env_file(example_path)?;

        let is_active = |key: &str| {
            environment
                .variables
                .get(key)
                .is_some_and(|variable| !variable.is_deleted())
        };

        let missing_in_vault = example_variables
            .keys()
            .filter(|key| !is_active(key))
            .cloned()
            .collect();

        let extra_in_vault = environment
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .filter(|(key, _)| !example_variables.contains_key(*key))
            .map(|(key, _)| key.clone())
            .collect();

        let shared_keys = example_variables
            .keys()
            .filter(|key| is_active(key))
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

    /// Compare current variables between two environments, which may
    /// belong to the same or different projects.
    pub fn diff_projects(
        &self,
        left_project: &str,
        left_environment: &str,
        right_project: &str,
        right_environment: &str,
        passphrase: &str,
    ) -> Result<ProjectDiff> {
        let vault = self.store.load(passphrase)?;
        let left_project_data =
            vault
                .projects
                .get(left_project)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: left_project.to_owned(),
                })?;
        let left = require_environment(left_project_data, left_project, left_environment)?;
        let right_project_data =
            vault
                .projects
                .get(right_project)
                .ok_or_else(|| EnvltError::ProjectNotFound {
                    name: right_project.to_owned(),
                })?;
        let right = require_environment(right_project_data, right_project, right_environment)?;

        let left_has = |key: &str| left.variables.get(key).is_some_and(|v| !v.is_deleted());
        let right_has = |key: &str| right.variables.get(key).is_some_and(|v| !v.is_deleted());

        let only_in_left = left
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .filter(|(key, _)| !right_has(key))
            .map(|(key, _)| key.clone())
            .collect();

        let only_in_right = right
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .filter(|(key, _)| !left_has(key))
            .map(|(key, _)| key.clone())
            .collect();

        let shared_keys: Vec<String> = left
            .variables
            .iter()
            .filter(|(_, variable)| !variable.is_deleted())
            .filter(|(key, _)| right_has(key))
            .map(|(key, _)| key.clone())
            .collect();

        let changed_values = shared_keys
            .iter()
            .filter(|key| {
                left.variables[key.as_str()].value() != right.variables[key.as_str()].value()
            })
            .cloned()
            .collect();

        let changed_types = shared_keys
            .iter()
            .filter(|key| {
                left.variables[key.as_str()].var_type() != right.variables[key.as_str()].var_type()
            })
            .cloned()
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

    /// Write one environment's current variables to a `.env` file,
    /// atomically and with `0600` permissions on Unix.
    pub fn write_env_file(
        &self,
        project_name: &str,
        environment_name: &str,
        output_path: &Path,
        passphrase: &str,
    ) -> Result<()> {
        use std::io::Write;

        let content =
            self.render_project_env_content(project_name, environment_name, passphrase)?;

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

    /// Render one environment's current variables as `.env`-formatted text.
    pub fn render_project_env_content(
        &self,
        project_name: &str,
        environment_name: &str,
        passphrase: &str,
    ) -> Result<String> {
        let variables = self.project_variables(project_name, environment_name, passphrase)?;
        Ok(render_env(&variables))
    }

    /// Resolve one environment's current variables for `envlt run`.
    pub fn build_run_environment(
        &self,
        project_name: &str,
        environment_name: &str,
        passphrase: &str,
    ) -> Result<RunEnvironment> {
        let variables = self.project_variables(project_name, environment_name, passphrase)?;
        Ok(RunEnvironment { variables })
    }

    /// Run environment/vault/link health checks for `envlt doctor`.
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

        match self.store.config() {
            Ok(config) => {
                let config_path = root_dir.join("config.toml");
                let source = if config_path.exists() {
                    "config.toml"
                } else {
                    "defaults"
                };
                checks.push(DiagnosticCheck {
                    code: "config".to_owned(),
                    severity: DiagnosticSeverity::Ok,
                    detail: format!(
                        "max_versions={}, lock_timeout_ms={} (from {source}, env vars override)",
                        config.max_versions, config.lock_timeout_ms
                    ),
                });
            }
            Err(error) => checks.push(DiagnosticCheck {
                code: "config".to_owned(),
                severity: DiagnosticSeverity::Error,
                detail: error.to_string(),
            }),
        }

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
                Some(passphrase) => match self.store.load_with_migration_info(passphrase) {
                    Ok((vault, migrated_from)) => {
                        let project_names = vault.projects.keys().cloned().collect::<Vec<_>>();
                        checks.push(DiagnosticCheck {
                            code: "decrypt".to_owned(),
                            severity: DiagnosticSeverity::Ok,
                            detail: format!(
                                "vault decrypted successfully ({} projects)",
                                project_names.len()
                            ),
                        });

                        checks.push(DiagnosticCheck {
                            code: "vault_format".to_owned(),
                            severity: DiagnosticSeverity::Ok,
                            detail: match migrated_from {
                                Some(old_version) => format!(
                                    "vault migrated from format version {old_version} to {} \
                                     (pre-migration backup kept as vault.v{old_version}.pre-migration.age); \
                                     this will be persisted on the next save",
                                    vault.version
                                ),
                                None => format!("vault is at the current format version ({})", vault.version),
                            },
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

        match find_project_link(&current_dir) {
            Ok(Some((link_dir, project, _environment))) => {
                checks.push(DiagnosticCheck {
                    code: "link".to_owned(),
                    severity: DiagnosticSeverity::Ok,
                    detail: format!(
                        ".envlt-link points to project '{project}' in {}",
                        link_dir.display()
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

                let env_file_path = link_dir.join(".env");
                if env_file_path.exists() {
                    checks.push(DiagnosticCheck {
                        code: "stray_env_file".to_owned(),
                        severity: DiagnosticSeverity::Warn,
                        detail: format!(
                            "found {} next to .envlt-link -- anything that reads the working \
                             directory, including AI coding assistants, can read it in \
                             plaintext; prefer `envlt run` to inject variables without writing \
                             this file, and delete it once you no longer need it",
                            env_file_path.display()
                        ),
                    });
                }
            }
            Ok(None) => checks.push(DiagnosticCheck {
                code: "link".to_owned(),
                severity: DiagnosticSeverity::Warn,
                detail: format!(
                    "no .envlt-link found in {} or its parent directories",
                    current_dir.display()
                ),
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
        let Some(search_root) = current_dir
            .map(Path::to_path_buf)
            .or_else(|| project.path.clone())
        else {
            return Ok(false);
        };

        match find_project_link(&search_root)? {
            Some((link_dir, linked_project, _environment)) if linked_project == project_name => {
                remove_project_link(&link_dir)
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
    use crate::vault::{ActivityAction, DEFAULT_ENVIRONMENT};
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
        let environment = &project.environments[DEFAULT_ENVIRONMENT];

        assert_eq!(
            environment
                .variables
                .get("API_KEY")
                .map(|var| var.var_type()),
            Some(VarType::Secret)
        );
        assert_eq!(
            environment.variables.get("PORT").map(|var| var.var_type()),
            Some(VarType::Plain)
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
            .set_variable(
                "typed-project",
                DEFAULT_ENVIRONMENT,
                "DB_PASSWORD",
                "secret",
                None,
                "passphrase",
            )
            .expect("set secret");
        service
            .set_variable(
                "typed-project",
                DEFAULT_ENVIRONMENT,
                "PORT",
                "4000",
                None,
                "passphrase",
            )
            .expect("update config");

        let project = service
            .project_snapshot("typed-project", "passphrase")
            .expect("snapshot");
        let environment = &project.environments[DEFAULT_ENVIRONMENT];

        assert_eq!(
            environment
                .variables
                .get("DB_PASSWORD")
                .map(|var| var.var_type()),
            Some(VarType::Secret)
        );
        assert_eq!(
            environment.variables.get("PORT").map(|var| var.var_type()),
            Some(VarType::Plain)
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
                DEFAULT_ENVIRONMENT,
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
            project.environments[DEFAULT_ENVIRONMENT]
                .variables
                .get("PORT")
                .map(|var| var.var_type()),
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
        let environment = &project.environments[DEFAULT_ENVIRONMENT];

        assert_eq!(
            environment.variables.get("PORT").map(|var| var.value()),
            Some("3000")
        );
        assert_eq!(
            environment.variables.get("API_KEY").map(|var| var.value()),
            Some("abc123")
        );
        assert_eq!(
            environment
                .variables
                .get("API_KEY")
                .map(|var| var.var_type()),
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

        let project = service
            .project_snapshot("typed-project", "passphrase")
            .expect("snapshot");
        let environment = &project.environments[DEFAULT_ENVIRONMENT];
        let views = service
            .project_variable_views("typed-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("variable views");

        assert_eq!(
            views,
            vec![
                VariableView {
                    key: "API_KEY".to_owned(),
                    value: "abc123".to_owned(),
                    var_type: VarType::Secret,
                    updated_at: environment.variables["API_KEY"].updated_at(),
                },
                VariableView {
                    key: "PORT".to_owned(),
                    value: "3000".to_owned(),
                    var_type: VarType::Plain,
                    updated_at: environment.variables["PORT"].updated_at(),
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
            .diff_project_against_example(
                "diff-project",
                DEFAULT_ENVIRONMENT,
                &example_path,
                "passphrase",
            )
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
                DEFAULT_ENVIRONMENT,
                "JWT_SECRET",
                GenType::JwtSecret,
                "passphrase",
            )
            .expect("generate and store");

        assert_eq!(value.len(), 64);

        let project = service
            .project_snapshot("gen-project", "passphrase")
            .expect("snapshot");
        let variable = project.environments[DEFAULT_ENVIRONMENT]
            .variables
            .get("JWT_SECRET")
            .expect("generated variable");

        assert_eq!(variable.value(), value);
        assert_eq!(variable.var_type(), VarType::Secret);
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
            .diff_projects(
                "left-project",
                DEFAULT_ENVIRONMENT,
                "right-project",
                DEFAULT_ENVIRONMENT,
                "passphrase",
            )
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
                DEFAULT_ENVIRONMENT,
                "API_TOKEN",
                "same",
                Some(VarType::Plain),
                "passphrase",
            )
            .expect("retag variable");

        let diff = service
            .diff_projects(
                "left-project",
                DEFAULT_ENVIRONMENT,
                "right-project",
                DEFAULT_ENVIRONMENT,
                "passphrase",
            )
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
    fn doctor_warns_about_a_stray_env_file_next_to_the_link() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let env_path = project_dir.path().join(".env");
        std::fs::write(&env_path, "PORT=3000\n").expect("write env");

        let service = AppService::new(VaultStore::new(home.path().to_path_buf()));
        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_file(
                "stray-env-project",
                &env_path,
                Some(project_dir.path().to_path_buf()),
                "passphrase",
            )
            .expect("add project");
        service
            .write_project_link(project_dir.path(), "stray-env-project")
            .expect("write project link");

        let report = service.doctor(Some(project_dir.path()), Some("passphrase"));

        assert!(!report.has_errors());
        assert!(report.checks.iter().any(|check| {
            check.code == "stray_env_file" && check.severity == DiagnosticSeverity::Warn
        }));
    }

    #[test]
    fn doctor_does_not_warn_when_no_env_file_is_present() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");

        let service = AppService::new(VaultStore::new(home.path().to_path_buf()));
        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("clean-project", "PORT=3000\n", None, "passphrase")
            .expect("add project");
        service
            .write_project_link(project_dir.path(), "clean-project")
            .expect("write project link");

        let report = service.doctor(Some(project_dir.path()), Some("passphrase"));

        assert!(!report
            .checks
            .iter()
            .any(|check| check.code == "stray_env_file"));
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
            .write_env_file(
                "perm-project",
                DEFAULT_ENVIRONMENT,
                &output_env,
                "passphrase",
            )
            .expect("write env file");

        let metadata = std::fs::metadata(&output_env).expect("metadata");
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }

    #[test]
    fn set_variable_generates_created_event() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "GREETING",
                "hello",
                Some(VarType::Plain),
                "passphrase",
            )
            .expect("set variable");

        let log = service
            .project_activity_log("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("activity log");

        let created = log
            .iter()
            .find(|e| e.action == ActivityAction::VariableCreated && e.variable_key == "GREETING");
        assert!(created.is_some(), "expected VariableCreated event");
        assert_eq!(created.unwrap().new_value, Some("hello".to_owned()));
    }

    #[test]
    fn set_variable_generates_updated_event() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "PORT",
                "4000",
                None,
                "passphrase",
            )
            .expect("set variable");

        let log = service
            .project_activity_log("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("activity log");

        let updated = log
            .iter()
            .find(|e| e.action == ActivityAction::VariableUpdated && e.variable_key == "PORT");
        assert!(updated.is_some(), "expected VariableUpdated event");
        assert_eq!(updated.unwrap().old_value, Some("3000".to_owned()));
        assert_eq!(updated.unwrap().new_value, Some("4000".to_owned()));
    }

    #[test]
    fn set_variable_generates_type_changed_event() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "PORT",
                "3000",
                Some(VarType::Secret),
                "passphrase",
            )
            .expect("set variable");

        let log = service
            .project_activity_log("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("activity log");

        let type_changed = log
            .iter()
            .find(|e| e.action == ActivityAction::VariableTypeChanged && e.variable_key == "PORT");
        assert!(type_changed.is_some(), "expected VariableTypeChanged event");
        assert_eq!(type_changed.unwrap().old_type, Some(VarType::Plain));
        assert_eq!(type_changed.unwrap().new_type, Some(VarType::Secret));
    }

    #[test]
    fn unset_variable_generates_deleted_event() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        service
            .unset_variable("test-project", DEFAULT_ENVIRONMENT, "PORT", "passphrase")
            .expect("unset variable");

        let log = service
            .project_activity_log("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("activity log");

        let deleted = log
            .iter()
            .find(|e| e.action == ActivityAction::VariableDeleted && e.variable_key == "PORT");
        assert!(deleted.is_some(), "expected VariableDeleted event");
        assert_eq!(deleted.unwrap().old_value, Some("3000".to_owned()));
    }

    #[test]
    fn unset_variable_twice_is_an_error() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        service
            .unset_variable("test-project", DEFAULT_ENVIRONMENT, "PORT", "passphrase")
            .expect("first unset");

        assert!(service
            .unset_variable("test-project", DEFAULT_ENVIRONMENT, "PORT", "passphrase")
            .is_err());
    }

    #[test]
    fn unset_variable_hides_it_from_project_variables_but_keeps_its_history() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");
        service
            .unset_variable("test-project", DEFAULT_ENVIRONMENT, "PORT", "passphrase")
            .expect("unset");

        let variables = service
            .project_variables("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("variables");
        assert!(!variables.contains_key("PORT"));

        let history = service
            .variable_history("test-project", DEFAULT_ENVIRONMENT, "PORT", "passphrase")
            .expect("history");
        assert!(history
            .iter()
            .any(|event| event.action == ActivityAction::VariableDeleted));
    }

    #[test]
    fn activity_log_masks_secret_values() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "API_KEY=secret123", None, "passphrase")
            .expect("add project");

        let log = service
            .project_activity_log("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("activity log");

        let created = log.iter().find(|e| e.variable_key == "API_KEY");
        assert!(created.is_some());
        let event = created.unwrap();
        assert_eq!(event.old_value, None);
        assert_eq!(event.new_value, None);
    }

    #[test]
    fn activity_log_respects_max_versions_env_var() {
        let _env_lock = crate::test_support::ENV_VAR_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        std::env::set_var("ENVLT_MAX_VERSIONS", "3");
        let _guard = CleanupEnvVar("ENVLT_MAX_VERSIONS");

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "A=1", None, "passphrase")
            .expect("add project");

        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "A",
                "2",
                None,
                "passphrase",
            )
            .expect("set");
        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "A",
                "3",
                None,
                "passphrase",
            )
            .expect("set");
        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "A",
                "4",
                None,
                "passphrase",
            )
            .expect("set");

        let log = service
            .project_activity_log("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("activity log");

        // max_versions=3 keeps only the last 3 of the 4 values A ever held
        // (1, 2, 3, 4), so the oldest (1) is trimmed away entirely.
        assert_eq!(log.len(), 3);
        assert!(!log
            .iter()
            .any(|e| e.new_value.as_deref() == Some("1") || e.old_value.as_deref() == Some("1")));
    }

    #[test]
    fn add_environment_creates_a_new_empty_environment() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");
        service
            .add_environment("test-project", "staging", "passphrase")
            .expect("add environment");

        let environments = service
            .list_environments("test-project", "passphrase")
            .expect("list environments");
        assert_eq!(
            environments,
            vec![DEFAULT_ENVIRONMENT.to_owned(), "staging".to_owned()]
        );

        let variables = service
            .project_variables("test-project", "staging", "passphrase")
            .expect("staging variables");
        assert!(variables.is_empty());
    }

    #[test]
    fn add_environment_rejects_a_duplicate_name() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        assert!(service
            .add_environment("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .is_err());
    }

    #[test]
    fn remove_environment_deletes_it_and_its_variables() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");
        service
            .add_environment("test-project", "staging", "passphrase")
            .expect("add environment");

        service
            .remove_environment("test-project", "staging", "passphrase")
            .expect("remove environment");

        let environments = service
            .list_environments("test-project", "passphrase")
            .expect("list environments");
        assert_eq!(environments, vec![DEFAULT_ENVIRONMENT.to_owned()]);
        assert!(service
            .project_variables("test-project", "staging", "passphrase")
            .is_err());
    }

    #[test]
    fn remove_environment_rejects_removing_the_last_one() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        assert!(service
            .remove_environment("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .is_err());
    }

    #[test]
    fn remove_environment_rejects_an_unknown_name() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");
        service
            .add_environment("test-project", "staging", "passphrase")
            .expect("add environment");

        assert!(service
            .remove_environment("test-project", "ghost", "passphrase")
            .is_err());
    }

    #[test]
    fn use_environment_pins_the_environment_in_envlt_link() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");
        service
            .add_environment("test-project", "staging", "passphrase")
            .expect("add environment");

        service
            .use_environment("test-project", "staging", project_dir.path(), "passphrase")
            .expect("use environment");

        let link_content =
            fs::read_to_string(project_dir.path().join(".envlt-link")).expect("read link");
        assert!(link_content.contains("staging"));
    }

    #[test]
    fn use_environment_rejects_an_unknown_environment() {
        let home = TempDir::new().expect("tempdir");
        let project_dir = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        assert!(service
            .use_environment("test-project", "ghost", project_dir.path(), "passphrase")
            .is_err());
        assert!(!project_dir.path().join(".envlt-link").exists());
    }

    #[test]
    fn set_variable_is_scoped_to_its_own_environment() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("test-project", "PORT=3000", None, "passphrase")
            .expect("add project");
        service
            .add_environment("test-project", "staging", "passphrase")
            .expect("add environment");
        service
            .set_variable(
                "test-project",
                "staging",
                "PORT",
                "9000",
                None,
                "passphrase",
            )
            .expect("set staging variable");

        let local_variables = service
            .project_variables("test-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("local variables");
        let staging_variables = service
            .project_variables("test-project", "staging", "passphrase")
            .expect("staging variables");

        assert_eq!(
            local_variables.get("PORT").map(String::as_str),
            Some("3000")
        );
        assert_eq!(
            staging_variables.get("PORT").map(String::as_str),
            Some("9000")
        );
    }

    #[test]
    fn resolve_environment_name_prefers_explicit_then_link_then_default() {
        assert_eq!(
            AppService::resolve_environment_name(Some("staging"), Some("prod")),
            "staging"
        );
        assert_eq!(
            AppService::resolve_environment_name(None, Some("prod")),
            "prod"
        );
        assert_eq!(
            AppService::resolve_environment_name(None, None),
            DEFAULT_ENVIRONMENT
        );
    }

    #[test]
    fn export_then_import_bundle_round_trips_a_single_environment() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str(
                "bundle-project",
                "PORT=3000\nAPI_KEY=abc123",
                None,
                "passphrase",
            )
            .expect("add project");

        let bundle = service
            .export_project_bundle(
                "bundle-project",
                DEFAULT_ENVIRONMENT,
                "passphrase",
                "bundle-pass",
            )
            .expect("export");

        service
            .remove_project("bundle-project", None, "passphrase")
            .expect("remove before import");

        let imported_name = service
            .import_project_bundle(&bundle, "passphrase", "bundle-pass", false)
            .expect("import");
        assert_eq!(imported_name, "bundle-project");

        let variables = service
            .project_variables("bundle-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("variables");
        assert_eq!(variables.get("PORT").map(String::as_str), Some("3000"));
        assert_eq!(variables.get("API_KEY").map(String::as_str), Some("abc123"));
    }

    #[test]
    fn import_bundle_merges_into_an_existing_project_and_preserves_history() {
        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());
        let service = AppService::new(store);

        service.init_vault("passphrase").expect("init");
        service
            .add_project_from_env_str("merge-project", "PORT=3000", None, "passphrase")
            .expect("add project");

        let bundle = service
            .export_project_bundle(
                "merge-project",
                DEFAULT_ENVIRONMENT,
                "passphrase",
                "bundle-pass",
            )
            .expect("export");

        service
            .set_variable(
                "merge-project",
                DEFAULT_ENVIRONMENT,
                "PORT",
                "4000",
                None,
                "passphrase",
            )
            .expect("change value locally after export");

        let imported_name = service
            .import_project_bundle(&bundle, "passphrase", "bundle-pass", true)
            .expect("import with overwrite");
        assert_eq!(imported_name, "merge-project");

        let history = service
            .variable_history("merge-project", DEFAULT_ENVIRONMENT, "PORT", "passphrase")
            .expect("history");
        // Created(3000) -> Updated(3000->4000) -> Updated(4000->3000 from the bundle).
        assert_eq!(history.len(), 3);
        let variables = service
            .project_variables("merge-project", DEFAULT_ENVIRONMENT, "passphrase")
            .expect("variables");
        assert_eq!(variables.get("PORT").map(String::as_str), Some("3000"));
    }

    #[test]
    fn vault_v1_migration_loads_variables_into_local_environment() {
        use crate::vault::crypto;

        let home = TempDir::new().expect("tempdir");
        let store = VaultStore::new(home.path().to_path_buf());

        // Craft a v1 vault manually (no activity_log, no environments).
        let v1_toml = r#"
version = 1
created_at = "2024-01-01T00:00:00Z"
updated_at = "2024-01-01T00:00:00Z"

[projects.test-project]
name = "test-project"
created_at = "2024-01-01T00:00:00Z"
updated_at = "2024-01-01T00:00:00Z"

[projects.test-project.variables.PORT]
value = "3000"
var_type = "Config"
created_at = "2024-01-01T00:00:00Z"
updated_at = "2024-01-01T00:00:00Z"
"#;

        let ciphertext = crypto::encrypt(v1_toml.as_bytes(), "passphrase").expect("encrypt");
        fs::create_dir_all(home.path()).expect("mkdir");
        fs::write(store.vault_path(), ciphertext).expect("write vault");

        let service = AppService::new(store);
        let project = service
            .project_snapshot("test-project", "passphrase")
            .expect("load migrated vault");

        let environment = &project.environments[DEFAULT_ENVIRONMENT];
        assert_eq!(
            environment.variables.get("PORT").map(|v| v.value()),
            Some("3000")
        );
        assert_eq!(
            environment.variables.get("PORT").map(|v| v.var_type()),
            Some(VarType::Plain)
        );

        // Trigger a save to verify the migrated version is persisted as 3.
        service
            .set_variable(
                "test-project",
                DEFAULT_ENVIRONMENT,
                "PORT",
                "4000",
                None,
                "passphrase",
            )
            .expect("set");

        let vault_text = {
            let ciphertext = fs::read(service.store().vault_path()).expect("read vault");
            let plaintext = crypto::decrypt(&ciphertext, "passphrase").expect("decrypt");
            String::from_utf8(plaintext.to_vec()).expect("utf8")
        };

        assert!(vault_text.contains("version = 3"));
        assert!(!vault_text.contains("activity_log"));
    }

    /// RAII guard to unset an environment variable when dropped.
    struct CleanupEnvVar(&'static str);

    impl Drop for CleanupEnvVar {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }
}
