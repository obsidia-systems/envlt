use std::{fs, path::Path, process::ExitCode};

use anyhow::{anyhow, Result};
use envlt_core::{bundle, AppService, EnvltError, VarType};

use crate::cli::{print_success, read_bundle_passphrase, read_passphrase};

pub fn run_import(
    service: &AppService,
    file: &Path,
    overwrite: bool,
    dry_run: bool,
    inspect: bool,
) -> Result<ExitCode> {
    let bundle_bytes = fs::read(file)?;

    if inspect {
        return inspect_bundle(&bundle_bytes);
    }

    if dry_run {
        return dry_run_import(service, &bundle_bytes, overwrite);
    }

    let vault_passphrase = read_passphrase(service.store(), false)?;
    let bundle_passphrase = read_bundle_passphrase(false)?;
    let project = service
        .import_project_bundle(
            &bundle_bytes,
            &vault_passphrase,
            &bundle_passphrase,
            overwrite,
        )
        .map_err(friendly_bundle_error)?;
    print_success(&format!("Bundle imported for project '{project}'."))?;
    Ok(ExitCode::SUCCESS)
}

/// Show the bundle's unencrypted header. The header (project name,
/// environment, export time, envlt version) sits outside the encrypted
/// payload, so this never asks for a passphrase and never touches the
/// vault.
fn inspect_bundle(bundle_bytes: &[u8]) -> Result<ExitCode> {
    let archive = bundle::decode_archive(bundle_bytes).map_err(friendly_bundle_error)?;

    println!("project:       {}", archive.header.project);
    println!("environment:   {}", archive.header.environment);
    println!("exported_at:   {}", archive.header.exported_at);
    println!("envlt_version: {}", archive.header.envlt_version);
    println!("(variable names and values require the bundle passphrase; use --dry-run)");

    Ok(ExitCode::SUCCESS)
}

/// Decrypt the bundle and check it against the vault without writing
/// anything, so a bundle can be validated before it is trusted.
fn dry_run_import(service: &AppService, bundle_bytes: &[u8], overwrite: bool) -> Result<ExitCode> {
    let bundle_passphrase = read_bundle_passphrase(false)?;
    let project = bundle::decrypt_project_bundle(bundle_bytes, &bundle_passphrase)
        .map_err(friendly_bundle_error)?;
    let (environment_name, environment) = project
        .environments
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("bundle payload has no environment"))?;

    let vault_passphrase = read_passphrase(service.store(), false)?;
    let already_exists = match service.project_snapshot(&project.name, &vault_passphrase) {
        Ok(_) => true,
        Err(EnvltError::ProjectNotFound { .. }) => false,
        Err(error) => return Err(error.into()),
    };

    if already_exists && !overwrite {
        return Err(anyhow!(
            "project '{}' already exists in the vault; dry run stopped here. \
             Pass --overwrite (without --dry-run) to actually replace it.",
            project.name
        ));
    }

    let action = if already_exists {
        "overwrite"
    } else {
        "create"
    };
    println!(
        "Dry run: importing this bundle would {action} project '{}' environment '{environment_name}' with {} variable(s):",
        project.name,
        environment.variables.len()
    );

    let mut keys: Vec<&String> = environment.variables.keys().collect();
    keys.sort();
    for key in keys {
        let var_type = environment.variables[key].var_type();
        println!("  {key} ({})", format_var_type(var_type));
    }

    println!("Nothing was written. Re-run without --dry-run to apply.");
    Ok(ExitCode::SUCCESS)
}

/// A v1 bundle (pre-environments) fails with `UnsupportedBundleVersion`
/// just like any other unreadable version, but for that specific case a
/// generic "unsupported version" message isn't actionable -- the fix is
/// always "ask the sender to re-export", so say that directly.
fn friendly_bundle_error(error: EnvltError) -> anyhow::Error {
    match error {
        EnvltError::UnsupportedBundleVersion { actual, .. } => anyhow!(
            "This bundle was exported by an older envlt (bundle format v{actual}) and can no \
             longer be imported. Ask the sender to re-export it with the current envlt version."
        ),
        other => other.into(),
    }
}

fn format_var_type(var_type: VarType) -> &'static str {
    match var_type {
        VarType::Secret => "secret",
        VarType::Plain => "plain",
    }
}
