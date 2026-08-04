use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current vault format version.
pub const VAULT_VERSION: u32 = 3;

/// Environment every new project starts with, and the one pre-v3 projects'
/// variables land in when migrated. Load-bearing beyond just a default: it
/// is the name `vault/migration.rs`'s v2-to-v3 step writes to disk, so
/// changing it is a vault-format change, not just a UX default.
pub const DEFAULT_ENVIRONMENT: &str = "local";

const SECRET_HINTS: [&str; 9] = [
    "KEY",
    "SECRET",
    "PASSWORD",
    "PASS",
    "TOKEN",
    "CREDENTIAL",
    "PRIVATE",
    "API_KEY",
    "AUTH",
];

/// Top-level encrypted vault containing all projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    /// Format version of this vault.
    pub version: u32,
    /// UTC timestamp when the vault was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
    /// Map of project names to their data.
    pub projects: BTreeMap<String, Project>,
}

impl VaultData {
    /// Create a new empty vault with the current version and timestamp.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            version: VAULT_VERSION,
            created_at: now,
            updated_at: now,
            projects: BTreeMap::new(),
        }
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl Default for VaultData {
    fn default() -> Self {
        Self::new()
    }
}

/// A named project. Variables live under one of its [`Environment`]s, not
/// directly on the project -- every variable belongs to exactly one
/// environment, even for projects that only ever have `"local"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project name (used as the lookup key in the vault).
    pub name: String,
    /// Optional filesystem path associated with the project.
    pub path: Option<PathBuf>,
    /// UTC timestamp when the project was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
    /// Sorted map of environment names (e.g. "local", "staging", "prod")
    /// to their variables.
    pub environments: BTreeMap<String, Environment>,
}

impl Project {
    /// Create a new project with the given name and optional path, and no
    /// environments yet. Callers that want the usual single-environment
    /// project should insert [`DEFAULT_ENVIRONMENT`] themselves; this
    /// constructor stays minimal so it can also build the single-
    /// environment "shadow" projects used for bundle export.
    pub fn new(name: impl Into<String>, path: Option<PathBuf>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            path,
            created_at: now,
            updated_at: now,
            environments: BTreeMap::new(),
        }
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Look up an environment by name.
    pub fn environment(&self, name: &str) -> Option<&Environment> {
        self.environments.get(name)
    }

    /// Look up an environment by name, mutably.
    pub fn environment_mut(&mut self, name: &str) -> Option<&mut Environment> {
        self.environments.get_mut(name)
    }
}

/// One deployment context (e.g. `local`, `staging`, `prod`) within a
/// project. Variables are fully duplicated per environment -- there is no
/// project-level default that environments inherit from. A variable's
/// meaning never depends on where in a lookup chain it was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Environment name (used as the lookup key on the project).
    pub name: String,
    /// UTC timestamp when the environment was created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the last modification.
    pub updated_at: DateTime<Utc>,
    /// Sorted map of variable keys to their values and history.
    pub variables: BTreeMap<String, Variable>,
}

impl Environment {
    /// Create a new, empty environment.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            created_at: now,
            updated_at: now,
            variables: BTreeMap::new(),
        }
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// One historical value a variable held, analogous to a version in
/// HashiCorp Vault's KV v2 secrets engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableVersion {
    /// The value at this point in the variable's history.
    pub value: String,
    /// The variable's classification at this point in its history.
    pub var_type: VarType,
    /// When this version was written.
    pub created_at: DateTime<Utc>,
}

/// A single environment variable, keeping its full value history rather
/// than just the current value.
///
/// History is kept for `Secret` values too, not only `Plain` ones -- a
/// deliberate trade-off (see `docs/security.md`): whoever can decrypt the
/// vault can already see the *current* secret, and this only extends that
/// same trust boundary to *past* values, bounded by
/// [`crate::config::Config::max_versions`] so it cannot grow without
/// limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Oldest to newest. Never empty for a `Variable` reachable through a
    /// project's variable map -- [`Variable::new`] and
    /// [`Variable::record`] are the only ways to produce or extend one,
    /// and both always leave at least one entry.
    pub versions: Vec<VariableVersion>,
    /// Free-text annotation (e.g. "prod DB password, rotate quarterly").
    /// Not versioned -- it describes the key, not a point-in-time value.
    #[serde(default)]
    pub description: Option<String>,
    /// Set when the variable is unset. The `Variable` stays in the map
    /// (it is not removed) so its version history survives deletion for
    /// `envlt history`; every "current state" read (`.env` generation,
    /// `vars`, `diff`, `run`, `check`) must filter these out.
    #[serde(default)]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Variable {
    /// Create a new variable with a type inferred from `name`.
    pub fn new(name: &str, value: impl Into<String>) -> Self {
        Self::new_with_type(value, infer_var_type(name))
    }

    /// Create a new variable with an explicit type.
    pub fn new_with_type(value: impl Into<String>, var_type: VarType) -> Self {
        Self {
            versions: vec![VariableVersion {
                value: value.into(),
                var_type,
                created_at: Utc::now(),
            }],
            description: None,
            deleted_at: None,
        }
    }

    /// The current (most recent) version.
    pub fn current(&self) -> &VariableVersion {
        self.versions
            .last()
            .expect("Variable::versions is never empty")
    }

    /// The current value.
    pub fn value(&self) -> &str {
        &self.current().value
    }

    /// The current classification.
    pub fn var_type(&self) -> VarType {
        self.current().var_type
    }

    /// When the variable was first created (its oldest version).
    pub fn created_at(&self) -> DateTime<Utc> {
        self.versions
            .first()
            .expect("Variable::versions is never empty")
            .created_at
    }

    /// When the variable was last changed (its current version).
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.current().created_at
    }

    /// Whether the variable has been unset.
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Append a new version, trimmed down to `max_versions` (oldest
    /// dropped first). Also clears any tombstone: re-setting a deleted
    /// key revives it and continues its version history rather than
    /// starting a new one.
    pub fn record(&mut self, value: impl Into<String>, var_type: VarType, max_versions: usize) {
        self.deleted_at = None;
        self.versions.push(VariableVersion {
            value: value.into(),
            var_type,
            created_at: Utc::now(),
        });

        let max_versions = max_versions.max(1);
        if self.versions.len() > max_versions {
            let overflow = self.versions.len() - max_versions;
            self.versions.drain(..overflow);
        }
    }

    /// Tombstone the variable without touching its version history.
    pub fn mark_deleted(&mut self) {
        self.deleted_at = Some(Utc::now());
    }
}

/// Classification for a variable based on sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VarType {
    /// Sensitive value that should be masked in output.
    Secret,
    /// Non-sensitive value, shown in full in output.
    ///
    /// Deserializes the legacy `Config` value too: `envlt` used to
    /// distinguish `Config` from `Plain`, but nothing in the codebase ever
    /// treated them differently, so they were merged. Old vaults and
    /// bundles keep loading correctly; they are just written back out as
    /// `Plain` the next time they're saved.
    #[serde(alias = "Config")]
    #[default]
    Plain,
}

/// The kind of change a synthesized history entry represents. Never
/// stored in the vault -- see [`synthesize_variable_events`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityAction {
    /// A variable was created.
    VariableCreated,
    /// A variable value was updated.
    VariableUpdated,
    /// A variable was deleted.
    VariableDeleted,
    /// A variable type was changed.
    VariableTypeChanged,
}

/// A single entry in a variable's or project's history, as displayed by
/// `envlt history`. Computed on demand by [`synthesize_variable_events`]
/// from [`Variable::versions`] and `deleted_at` -- never stored in the
/// vault. Kept as its own type (rather than having callers walk
/// `VariableVersion`s directly) so the CLI's rendering code didn't need
/// to change when the underlying storage moved from a hand-maintained log
/// to on-demand reconstruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// What happened.
    pub action: ActivityAction,
    /// The variable key involved.
    pub variable_key: String,
    /// Previous value, if applicable and non-secret.
    pub old_value: Option<String>,
    /// New value, if applicable and non-secret.
    pub new_value: Option<String>,
    /// Previous type, if applicable.
    pub old_type: Option<VarType>,
    /// New type, if applicable.
    pub new_type: Option<VarType>,
}

impl ActivityEvent {
    /// Helper to mask a value based on the variable type.
    pub fn masked_value(value: &str, var_type: VarType) -> Option<String> {
        match var_type {
            VarType::Secret => None,
            VarType::Plain => Some(value.to_owned()),
        }
    }
}

/// Reconstruct a variable's ordered history by diffing adjacent
/// [`VariableVersion`]s, appending a synthetic `VariableDeleted` entry if
/// the variable has been unset. This is the sole place that maps version
/// data onto the `Created`/`Updated`/`TypeChanged`/`Deleted` vocabulary,
/// so mutating code (`AppService::set_variable` and friends) no longer
/// needs to construct matching events by hand alongside every change --
/// the version list is now the only thing that can drift out of sync with
/// itself.
pub fn synthesize_variable_events(key: &str, variable: &Variable) -> Vec<ActivityEvent> {
    let mut events = Vec::new();
    let mut versions = variable.versions.iter();

    let Some(first) = versions.next() else {
        return events;
    };

    events.push(ActivityEvent {
        timestamp: first.created_at,
        action: ActivityAction::VariableCreated,
        variable_key: key.to_owned(),
        old_value: None,
        new_value: ActivityEvent::masked_value(&first.value, first.var_type),
        old_type: None,
        new_type: None,
    });

    let mut previous = first;
    for version in versions {
        let type_changed = previous.var_type != version.var_type;
        let value_changed = previous.value != version.value;

        if type_changed {
            events.push(ActivityEvent {
                timestamp: version.created_at,
                action: ActivityAction::VariableTypeChanged,
                variable_key: key.to_owned(),
                old_value: None,
                new_value: None,
                old_type: Some(previous.var_type),
                new_type: Some(version.var_type),
            });
        }

        if value_changed {
            events.push(ActivityEvent {
                timestamp: version.created_at,
                action: ActivityAction::VariableUpdated,
                variable_key: key.to_owned(),
                old_value: ActivityEvent::masked_value(&previous.value, previous.var_type),
                new_value: ActivityEvent::masked_value(&version.value, version.var_type),
                old_type: None,
                new_type: None,
            });
        }

        previous = version;
    }

    if let Some(deleted_at) = variable.deleted_at {
        events.push(ActivityEvent {
            timestamp: deleted_at,
            action: ActivityAction::VariableDeleted,
            variable_key: key.to_owned(),
            old_value: ActivityEvent::masked_value(&previous.value, previous.var_type),
            new_value: None,
            old_type: None,
            new_type: None,
        });
    }

    events
}

/// Infer the variable type from common naming conventions.
pub fn infer_var_type(name: &str) -> VarType {
    let uppercase_name = name.to_ascii_uppercase();
    if SECRET_HINTS
        .iter()
        .any(|hint| uppercase_name.contains(hint))
    {
        VarType::Secret
    } else {
        VarType::Plain
    }
}

#[cfg(test)]
mod tests {
    use super::{
        infer_var_type, synthesize_variable_events, ActivityAction, VarType, Variable,
        VariableVersion,
    };

    #[test]
    fn infers_secret_type_from_sensitive_key_names() {
        assert_eq!(infer_var_type("API_KEY"), VarType::Secret);
        assert_eq!(infer_var_type("db_password"), VarType::Secret);
        assert_eq!(infer_var_type("auth_token"), VarType::Secret);
    }

    #[test]
    fn infers_plain_type_when_name_is_not_sensitive() {
        assert_eq!(infer_var_type("PORT"), VarType::Plain);
        assert_eq!(infer_var_type("APP_ENV"), VarType::Plain);
    }

    #[test]
    fn new_variable_uses_inferred_type() {
        let variable = Variable::new("JWT_SECRET", "top-secret");
        assert_eq!(variable.var_type(), VarType::Secret);
        assert_eq!(variable.value(), "top-secret");
        assert_eq!(variable.versions.len(), 1);
    }

    #[test]
    fn new_with_type_uses_explicit_type() {
        let variable = Variable::new_with_type("value", VarType::Plain);
        assert_eq!(variable.var_type(), VarType::Plain);
    }

    #[test]
    fn deserializes_the_legacy_config_value_as_plain() {
        let version: VariableVersion = toml::from_str(
            "value = \"hello\"\nvar_type = \"Config\"\ncreated_at = \"2024-01-01T00:00:00Z\"\n",
        )
        .expect("parse legacy Config version");
        assert_eq!(version.var_type, VarType::Plain);
    }

    #[test]
    fn record_appends_a_version_and_updates_current() {
        let mut variable = Variable::new_with_type("first", VarType::Plain);
        variable.record("second", VarType::Plain, 10);

        assert_eq!(variable.value(), "second");
        assert_eq!(variable.versions.len(), 2);
        assert_eq!(variable.versions[0].value, "first");
    }

    #[test]
    fn record_trims_to_max_versions_dropping_oldest_first() {
        let mut variable = Variable::new_with_type("v1", VarType::Plain);
        variable.record("v2", VarType::Plain, 2);
        variable.record("v3", VarType::Plain, 2);

        assert_eq!(variable.versions.len(), 2);
        assert_eq!(variable.versions[0].value, "v2");
        assert_eq!(variable.versions[1].value, "v3");
    }

    #[test]
    fn record_clamps_a_zero_max_versions_to_keep_at_least_one() {
        let mut variable = Variable::new_with_type("v1", VarType::Plain);
        variable.record("v2", VarType::Plain, 0);

        assert_eq!(variable.versions.len(), 1);
        assert_eq!(variable.value(), "v2");
    }

    #[test]
    fn record_revives_a_deleted_variable_and_continues_its_history() {
        let mut variable = Variable::new_with_type("v1", VarType::Plain);
        variable.mark_deleted();
        assert!(variable.is_deleted());

        variable.record("v2", VarType::Plain, 10);

        assert!(!variable.is_deleted());
        assert_eq!(variable.versions.len(), 2);
    }

    #[test]
    fn synthesize_variable_events_reconstructs_created_updated_and_type_changed() {
        let mut variable = Variable::new_with_type("3000", VarType::Plain);
        variable.record("4000", VarType::Plain, 10);
        variable.record("4000", VarType::Secret, 10);

        let events = synthesize_variable_events("PORT", &variable);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].action, ActivityAction::VariableCreated);
        assert_eq!(events[0].new_value, Some("3000".to_owned()));
        assert_eq!(events[1].action, ActivityAction::VariableUpdated);
        assert_eq!(events[1].old_value, Some("3000".to_owned()));
        assert_eq!(events[1].new_value, Some("4000".to_owned()));
        assert_eq!(events[2].action, ActivityAction::VariableTypeChanged);
        assert_eq!(events[2].old_type, Some(VarType::Plain));
        assert_eq!(events[2].new_type, Some(VarType::Secret));
    }

    #[test]
    fn synthesize_variable_events_masks_secret_values() {
        let variable = Variable::new_with_type("super-secret", VarType::Secret);
        let events = synthesize_variable_events("API_KEY", &variable);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].new_value, None);
    }

    #[test]
    fn synthesize_variable_events_appends_deleted_when_tombstoned() {
        let mut variable = Variable::new_with_type("value", VarType::Plain);
        variable.mark_deleted();

        let events = synthesize_variable_events("TEMP", &variable);

        assert_eq!(events.len(), 2);
        assert_eq!(events[1].action, ActivityAction::VariableDeleted);
        assert_eq!(events[1].old_value, Some("value".to_owned()));
    }

    #[test]
    fn synthesize_variable_events_emits_nothing_extra_when_nothing_changed() {
        // record() is only ever called by AppService when something
        // actually changed, so every transition in `versions` represents
        // a real change -- this test documents that invariant at the
        // synthesis boundary rather than re-testing AppService's guard.
        let variable = Variable::new_with_type("value", VarType::Plain);
        let events = synthesize_variable_events("KEY", &variable);
        assert_eq!(events.len(), 1);
    }
}
