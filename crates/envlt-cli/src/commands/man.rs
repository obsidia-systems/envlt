use std::{fs, path::Path, process::ExitCode};

use anyhow::Result;
use clap::CommandFactory;

use crate::Cli;

pub fn run_man(out: &Path) -> Result<ExitCode> {
    fs::create_dir_all(out)?;
    clap_mangen::generate_to(Cli::command(), out)?;
    println!("Man pages written to {}", out.display());
    Ok(ExitCode::SUCCESS)
}
