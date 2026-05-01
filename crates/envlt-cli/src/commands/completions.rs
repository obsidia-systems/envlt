use std::{io, process::ExitCode};

use anyhow::Result;
use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};

use crate::Cli;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl From<CompletionShell> for Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Elvish => Shell::Elvish,
        }
    }
}

pub fn run_completions(shell: CompletionShell) -> Result<ExitCode> {
    let mut cmd = Cli::command();
    let shell: Shell = shell.into();
    generate(shell, &mut cmd, "envlt", &mut io::stdout());
    Ok(ExitCode::SUCCESS)
}
