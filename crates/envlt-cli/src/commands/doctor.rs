use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;
use serde_json::{json, to_string_pretty};

use crate::cli::read_passphrase_if_available;
use crate::output::{render_raw_rows, render_table, rows_to_json_objects, OutputFormat};

pub fn run_doctor(service: &AppService, decrypt: bool, format: OutputFormat) -> Result<ExitCode> {
    let env_or_keyring_available = std::env::var_os("ENVLT_PASSPHRASE").is_some()
        || envlt_core::load_stored_passphrase(service.store())?.is_some();
    let passphrase = if decrypt || env_or_keyring_available {
        read_passphrase_if_available(service.store())?
    } else {
        None
    };

    let report = service.doctor(None, passphrase.as_deref());
    let has_errors = report.has_errors();

    match format {
        OutputFormat::Raw => {
            println!(
                "summary\tok={}\twarn={}\terror={}",
                report.ok_count(),
                report.warn_count(),
                report.error_count()
            );

            let check_rows = report
                .checks
                .iter()
                .map(|check| {
                    vec![
                        check.severity.as_str().to_owned(),
                        check.code.clone(),
                        check.detail.clone(),
                    ]
                })
                .collect::<Vec<_>>();
            println!("{}", render_raw_rows(&check_rows));
        }
        OutputFormat::Table => {
            let summary_rows = vec![
                vec!["ok".to_owned(), report.ok_count().to_string()],
                vec!["warn".to_owned(), report.warn_count().to_string()],
                vec!["error".to_owned(), report.error_count().to_string()],
            ];
            println!("{}", render_table(&["metric", "count"], &summary_rows));
            println!();

            let check_rows = report
                .checks
                .iter()
                .map(|check| {
                    vec![
                        check.severity.as_str().to_owned(),
                        check.code.clone(),
                        check.detail.clone(),
                    ]
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                render_table(&["severity", "code", "detail"], &check_rows)
            );
        }
        OutputFormat::Json => {
            let check_rows = report
                .checks
                .iter()
                .map(|check| {
                    vec![
                        check.severity.as_str().to_owned(),
                        check.code.clone(),
                        check.detail.clone(),
                    ]
                })
                .collect::<Vec<_>>();
            let checks = rows_to_json_objects(&["severity", "code", "detail"], &check_rows);

            let payload = json!({
                "summary": {
                    "ok": report.ok_count(),
                    "warn": report.warn_count(),
                    "error": report.error_count(),
                },
                "checks": checks,
            });

            println!("{}", to_string_pretty(&payload)?);
        }
    }

    if has_errors {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}
