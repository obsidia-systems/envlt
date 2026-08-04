use toml::{value::Table, Value};

use crate::{
    error::{EnvltError, Result},
    vault::model::{DEFAULT_ENVIRONMENT, VAULT_VERSION},
};

/// Oldest on-disk vault format version this build can still read.
///
/// A vault older than this has no migration path and is rejected outright,
/// the same way a version newer than [`VAULT_VERSION`] is.
pub const MIN_SUPPORTED_VAULT_VERSION: u32 = 1;

/// Migrate `table` in place from `from_version` up to [`VAULT_VERSION`],
/// applying one versioned step at a time.
///
/// Each step only has to know about the single format change it
/// introduced, so adding a future migration means adding one match arm
/// here rather than reworking existing ones. Operating on the raw TOML
/// table (rather than the current `VaultData` struct) means a migration
/// can rename or restructure fields, not just fill in defaults.
pub fn migrate(table: &mut toml::value::Table, from_version: u32) -> Result<()> {
    if from_version > VAULT_VERSION {
        return Err(EnvltError::UnsupportedVaultVersion {
            expected: VAULT_VERSION,
            actual: from_version,
        });
    }

    let mut version = from_version;
    while version < VAULT_VERSION {
        match version {
            1 => migrate_v1_to_v2(table),
            2 => migrate_v2_to_v3(table),
            other => {
                return Err(EnvltError::UnsupportedVaultVersion {
                    expected: VAULT_VERSION,
                    actual: other,
                })
            }
        }
        version += 1;
    }
    table.insert("version".to_owned(), Value::Integer(i64::from(version)));
    Ok(())
}

/// v1 -> v2: added a per-project `activity_log`, defaulting to empty for
/// projects created before the activity log feature existed.
fn migrate_v1_to_v2(table: &mut toml::value::Table) {
    let Some(Value::Table(projects)) = table.get_mut("projects") else {
        return;
    };

    for (_name, project) in projects.iter_mut() {
        if let Value::Table(project_table) = project {
            project_table
                .entry("activity_log")
                .or_insert_with(|| Value::Array(Vec::new()));
        }
    }
}

/// v2 -> v3: introduced `Environment` as an explicit layer between a
/// project and its variables, and moved from a hand-maintained delta log
/// to full per-variable version history.
///
/// Every pre-v3 project's `variables` become the sole environment, named
/// [`DEFAULT_ENVIRONMENT`]. The old `activity_log` is dropped, not carried
/// forward: it already stored `None` for `Secret` old/new values (masked
/// by design), so only `Plain` entries could ever be reconstructed --
/// converting just those would produce a history that is inexplicably
/// deeper for some variables than others. Dropping it uniformly is honest
/// about the trade-off, and isn't true data loss: the pre-migration
/// ciphertext is preserved verbatim in `vault.v2.pre-migration.age` by
/// `VaultStore`'s existing backup mechanism.
fn migrate_v2_to_v3(table: &mut toml::value::Table) {
    let Some(Value::Table(projects)) = table.get_mut("projects") else {
        return;
    };

    for (_name, project) in projects.iter_mut() {
        if let Value::Table(project_table) = project {
            migrate_project_v2_to_v3(project_table);
        }
    }
}

fn migrate_project_v2_to_v3(project_table: &mut Table) {
    project_table.remove("activity_log");

    let project_created_at = project_table.get("created_at").cloned();
    let project_updated_at = project_table.get("updated_at").cloned();

    let Some(Value::Table(mut variables)) = project_table.remove("variables") else {
        return;
    };

    for (_key, variable) in variables.iter_mut() {
        if let Value::Table(var_table) = variable {
            restructure_variable_v2_to_v3(var_table);
        }
    }

    let mut environment = Table::new();
    environment.insert(
        "name".to_owned(),
        Value::String(DEFAULT_ENVIRONMENT.to_owned()),
    );
    if let Some(created_at) = project_created_at {
        environment.insert("created_at".to_owned(), created_at);
    }
    if let Some(updated_at) = project_updated_at {
        environment.insert("updated_at".to_owned(), updated_at);
    }
    environment.insert("variables".to_owned(), Value::Table(variables));

    let mut environments = Table::new();
    environments.insert(DEFAULT_ENVIRONMENT.to_owned(), Value::Table(environment));

    project_table.insert("environments".to_owned(), Value::Table(environments));
}

/// `{value, var_type, created_at, updated_at}` -> `{versions: [{value,
/// var_type, created_at}]}` (`description`/`deleted_at` are left absent
/// and pick up their `#[serde(default)]` of `None`).
///
/// The single seeded version is stamped with the old `updated_at` (when
/// the value was last actually written), not `created_at` (when the
/// variable was first created) -- `Variable::updated_at()` is defined as
/// "the last version's `created_at`", and using the old `updated_at` here
/// keeps that identity true immediately after migration.
fn restructure_variable_v2_to_v3(var_table: &mut Table) {
    let value = var_table.remove("value");
    let var_type = var_table.remove("var_type");
    let created_at = var_table.remove("created_at");
    let version_created_at = var_table.remove("updated_at").or(created_at);

    let mut version = Table::new();
    if let Some(value) = value {
        version.insert("value".to_owned(), value);
    }
    if let Some(var_type) = var_type {
        version.insert("var_type".to_owned(), var_type);
    }
    if let Some(created_at) = version_created_at {
        version.insert("created_at".to_owned(), created_at);
    }

    var_table.insert(
        "versions".to_owned(),
        Value::Array(vec![Value::Table(version)]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_v1_to_v2_backfills_missing_activity_log() {
        let mut table: toml::value::Table = toml::from_str(
            r#"
            version = 1

            [projects.demo]
            name = "demo"
            "#,
        )
        .expect("parse table");

        migrate_v1_to_v2(&mut table);

        let project = &table["projects"]["demo"];
        assert_eq!(project["activity_log"].as_array(), Some(&Vec::new()));
    }

    #[test]
    fn migrate_preserves_an_existing_activity_log_through_v1_to_v2() {
        let mut table: toml::value::Table = toml::from_str(
            r#"
            version = 1

            [[projects.demo.activity_log]]
            variable_key = "PORT"
            action = "VariableCreated"

            [projects.demo]
            name = "demo"
            "#,
        )
        .expect("parse table");

        migrate_v1_to_v2(&mut table);

        let log = table["projects"]["demo"]["activity_log"]
            .as_array()
            .expect("activity_log array");
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn migrate_v2_to_v3_moves_variables_into_local_environment() {
        let mut table: toml::value::Table = toml::from_str(
            r#"
            version = 2
            created_at = "2024-01-01T00:00:00Z"
            updated_at = "2024-01-01T00:00:00Z"

            [projects.demo]
            name = "demo"
            created_at = "2024-01-01T00:00:00Z"
            updated_at = "2024-06-01T00:00:00Z"

            [projects.demo.variables.PORT]
            value = "3000"
            var_type = "Plain"
            created_at = "2024-01-01T00:00:00Z"
            updated_at = "2024-02-01T00:00:00Z"

            [projects.demo.activity_log]
            "#,
        )
        .expect("parse table");

        migrate_v2_to_v3(&mut table);

        let project = &table["projects"]["demo"];
        assert!(project.get("variables").is_none());
        assert!(project.get("activity_log").is_none());

        let environment = &project["environments"]["local"];
        assert_eq!(environment["name"].as_str(), Some("local"));

        let versions = environment["variables"]["PORT"]["versions"]
            .as_array()
            .expect("versions array");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0]["value"].as_str(), Some("3000"));
        assert_eq!(versions[0]["var_type"].as_str(), Some("Plain"));
        assert_eq!(
            versions[0]["created_at"].as_str(),
            Some("2024-02-01T00:00:00Z")
        );
    }

    #[test]
    fn migrate_v1_to_v3_end_to_end_produces_one_local_environment() {
        let mut table: toml::value::Table = toml::from_str(
            r#"
            version = 1
            created_at = "2024-01-01T00:00:00Z"
            updated_at = "2024-01-01T00:00:00Z"

            [projects.demo]
            name = "demo"
            created_at = "2024-01-01T00:00:00Z"
            updated_at = "2024-01-01T00:00:00Z"

            [projects.demo.variables.PORT]
            value = "3000"
            var_type = "Plain"
            created_at = "2024-01-01T00:00:00Z"
            updated_at = "2024-01-01T00:00:00Z"
            "#,
        )
        .expect("parse table");

        migrate(&mut table, 1).expect("migrate");

        assert_eq!(table["version"].as_integer(), Some(3));
        let project = &table["projects"]["demo"];
        assert!(project.get("activity_log").is_none());
        assert!(project["environments"]["local"]["variables"]
            .get("PORT")
            .is_some());
    }

    #[test]
    fn migrate_rejects_a_version_with_no_known_step() {
        let mut table: toml::value::Table = toml::from_str("version = 99").expect("parse table");

        let error = migrate(&mut table, 99).expect_err("no migration path");
        assert!(matches!(
            error,
            EnvltError::UnsupportedVaultVersion {
                expected: VAULT_VERSION,
                actual: 99
            }
        ));
    }
}
