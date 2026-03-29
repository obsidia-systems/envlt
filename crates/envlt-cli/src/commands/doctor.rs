use std::process::ExitCode;

use anyhow::Result;
use envlt_core::AppService;

use crate::cli::read_passphrase;

pub fn run_doctor(service: &AppService, decrypt: bool) -> Result<ExitCode> {
    let env_passphrase_present = std::env::var_os("ENVLT_PASSPHRASE").is_some();
    let passphrase = if decrypt || env_passphrase_present {
        Some(read_passphrase(false)?)
    } else {
        None
    };

    let report = service.doctor(None, passphrase.as_deref());

    println!(
        "summary\tok={}\twarn={}\terror={}",
        report.ok_count(),
        report.warn_count(),
        report.error_count()
    );
    let has_errors = report.has_errors();
    for check in &report.checks {
        println!(
            "{}\t{}\t{}",
            check.severity.as_str(),
            check.code,
            check.detail
        );
    }

    if has_errors {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}
