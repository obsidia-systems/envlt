use toml::Value;

use crate::{
    error::{EnvltError, Result},
    vault::model::VAULT_VERSION,
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

        migrate(&mut table, 1).expect("migrate");

        assert_eq!(table["version"].as_integer(), Some(2));
        let project = &table["projects"]["demo"];
        assert_eq!(project["activity_log"].as_array(), Some(&Vec::new()));
    }

    #[test]
    fn migrate_preserves_an_existing_activity_log() {
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

        migrate(&mut table, 1).expect("migrate");

        let log = table["projects"]["demo"]["activity_log"]
            .as_array()
            .expect("activity_log array");
        assert_eq!(log.len(), 1);
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
