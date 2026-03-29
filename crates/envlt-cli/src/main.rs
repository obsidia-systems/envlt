mod cli;
mod commands;

use std::{path::PathBuf, process::ExitCode};

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{
    add::run_add,
    diff::run_diff,
    doctor::run_doctor,
    export::run_export,
    gen::{run_gen, GenOptions},
    import::run_import,
    init::run_init,
    list::run_list,
    remove::run_remove,
    run::run_run,
    set::run_set,
    use_cmd::run_use,
    vars::run_vars,
};
use envlt_core::{AppService, VaultStore};

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<ExitCode> {
    let args = Cli::parse();
    let store = VaultStore::from_env()?;
    let service = AppService::new(store);

    match args.command {
        Commands::Init => run_init(&service),
        Commands::Add {
            project,
            file,
            from_example,
            project_path,
        } => run_add(&service, &project, &file, &from_example, project_path),
        Commands::List => run_list(&service),
        Commands::Remove { project, yes } => run_remove(&service, &project, yes),
        Commands::Vars { project } => run_vars(&service, &project),
        Commands::Doctor { decrypt } => run_doctor(&service, decrypt),
        Commands::Diff {
            project,
            other_project,
            example,
        } => run_diff(&service, &project, &other_project, &example),
        Commands::Gen {
            gen_type,
            list_types,
            len,
            hex,
            symbols,
            show,
            set,
            project,
            silent,
        } => run_gen(
            &service,
            GenOptions {
                gen_type: gen_type.as_deref(),
                list_types,
                len,
                hex,
                symbols,
                show,
                set_key: &set,
                project: &project,
                silent,
            },
        ),
        Commands::Export { project, out } => run_export(&service, &project, &out),
        Commands::Import { file, overwrite } => run_import(&service, &file, overwrite),
        Commands::Set {
            project,
            assignment,
            secret,
            config,
            plain,
        } => run_set(&service, &project, &assignment, secret, config, plain),
        Commands::Use { project, out } => run_use(&service, &project, &out),
        Commands::Run { project, command } => run_run(&service, &project, &command),
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "envlt",
    version,
    about = "Local-first encrypted environment vault",
    long_about = "envlt stores project environment variables in an encrypted local vault, regenerates .env files when needed, and can run commands with injected variables without requiring a cloud service."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Initialize the encrypted local vault")]
    Init,
    #[command(
        about = "Import variables from .env or .env.example into the vault",
        long_about = "Import a project into the encrypted vault from an existing .env file or bootstrap it from a .env.example file. The command also writes a .envlt-link file so the current directory can resolve the project automatically later."
    )]
    Add {
        #[arg(help = "Project name to create in the vault")]
        project: String,
        #[arg(long, default_value = ".env", help = "Path to the .env file to import")]
        file: PathBuf,
        #[arg(
            long,
            help = "Bootstrap from a .env.example file instead of a .env file"
        )]
        from_example: Option<PathBuf>,
        #[arg(help = "Project root to associate with the .envlt-link file", long)]
        project_path: Option<PathBuf>,
    },
    #[command(about = "List all stored projects")]
    List,
    #[command(
        about = "Remove a stored project from the vault",
        long_about = "Remove a project from the encrypted vault. By default envlt asks for confirmation first. If the current directory has a .envlt-link that points to the removed project, envlt clears that link as part of the operation."
    )]
    Remove {
        #[arg(help = "Project name to remove from the vault")]
        project: String,
        #[arg(long, short = 'y', help = "Skip the confirmation prompt")]
        yes: bool,
    },
    #[command(
        about = "Run local diagnostics for the vault and project link state",
        long_about = "Inspect the envlt home directory, vault presence, backup presence, and .envlt-link state. Optionally try to decrypt the vault and validate that the linked project exists."
    )]
    Doctor {
        #[arg(
            long,
            help = "Attempt to decrypt the vault and validate linked-project state"
        )]
        decrypt: bool,
    },
    #[command(
        about = "Show variables stored for a project",
        long_about = "Display variable names, variable types, and values for a project. Secret values are masked by default while Config and Plain values remain visible."
    )]
    Vars {
        #[arg(
            long,
            help = "Project to inspect; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
    },
    #[command(
        about = "Compare a project against .env.example or another project",
        long_about = "Produce a safe summary diff without printing secret values. Use --example to compare against a .env.example file, or pass another project name to compare two vault projects."
    )]
    Diff {
        #[arg(
            long,
            help = "Base project to compare; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(help = "Other project to compare against")]
        other_project: Option<String>,
        #[arg(
            long,
            conflicts_with = "other_project",
            help = "Path to a .env.example file to compare against"
        )]
        example: Option<PathBuf>,
    },
    #[command(
        about = "Generate secure values and optionally store them",
        long_about = "Generate secrets or identifiers using built-in presets or custom length settings. Generated values can be printed, stored directly in the vault, or produced through a guided interactive flow."
    )]
    Gen {
        #[arg(long = "type", help = "Generator preset to use")]
        gen_type: Option<String>,
        #[arg(long, help = "List supported generator presets and exit")]
        list_types: bool,
        #[arg(long, help = "Generate a custom value with the requested length")]
        len: Option<usize>,
        #[arg(long, conflicts_with = "symbols", help = "Use a hexadecimal alphabet")]
        hex: bool,
        #[arg(
            long,
            conflicts_with = "hex",
            help = "Include symbols in the generated value"
        )]
        symbols: bool,
        #[arg(
            long,
            conflicts_with = "silent",
            help = "Reveal the generated value even when storing it in the vault"
        )]
        show: bool,
        #[arg(long, help = "Store the generated value in the given variable key")]
        set: Option<String>,
        #[arg(long, help = "Target project for storing the generated value")]
        project: Option<String>,
        #[arg(long, help = "Suppress all command output")]
        silent: bool,
    },
    #[command(about = "Export a project to an encrypted .evlt bundle")]
    Export {
        #[arg(help = "Project name to export")]
        project: String,
        #[arg(long, help = "Output path for the encrypted bundle")]
        out: PathBuf,
    },
    #[command(
        about = "Import an encrypted .evlt bundle into the vault",
        long_about = "Import a project snapshot from an encrypted .evlt bundle. By default envlt refuses to overwrite an existing project unless --overwrite is provided."
    )]
    Import {
        #[arg(help = "Path to the .evlt bundle to import")]
        file: PathBuf,
        #[arg(long, help = "Replace an existing project with the same name")]
        overwrite: bool,
    },
    #[command(
        about = "Create or update a project variable",
        long_about = "Set a variable for a project using KEY=VALUE syntax. The variable type can be inferred automatically or overridden explicitly with --secret, --config, or --plain."
    )]
    Set {
        #[arg(
            long,
            help = "Project to update; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            conflicts_with_all = ["config", "plain"],
            help = "Mark the variable as Secret"
        )]
        secret: bool,
        #[arg(
            long,
            conflicts_with_all = ["secret", "plain"],
            help = "Mark the variable as Config"
        )]
        config: bool,
        #[arg(
            long,
            conflicts_with_all = ["secret", "config"],
            help = "Mark the variable as Plain"
        )]
        plain: bool,
        #[arg(help = "Variable assignment in KEY=VALUE format")]
        assignment: String,
    },
    #[command(
        about = "Write a .env file from a project stored in the vault",
        long_about = "Materialize a project's variables into a .env-style file. This is useful for local tooling that expects a file on disk."
    )]
    Use {
        #[arg(
            long,
            help = "Project to materialize; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            default_value = ".env",
            help = "Output path for the rendered env file"
        )]
        out: PathBuf,
    },
    #[command(
        about = "Run a child process with vault variables injected",
        long_about = "Resolve a project's variables from the vault and inject them into a child process environment without writing a .env file to disk."
    )]
    Run {
        #[arg(
            long,
            help = "Project to run with; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            help = "Command and arguments to execute",
            required = true,
            trailing_var_arg = true
        )]
        command: Vec<String>,
    },
}
