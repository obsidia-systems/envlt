mod cli;
mod commands;
mod output;

use std::{path::PathBuf, process::ExitCode};

use anyhow::Result;
use clap::{
    builder::styling::{AnsiColor, Styles},
    Parser, Subcommand,
};
use commands::{
    add::run_add,
    auth::{run_auth_clear, run_auth_save, run_auth_status},
    check::run_check,
    completions::{run_completions, CompletionShell},
    diff::run_diff,
    doctor::run_doctor,
    env::{run_env_add, run_env_list, run_env_remove, run_env_switch},
    export::run_export,
    generate::{run_generate, GenerateOptions},
    get::run_get,
    history::run_history,
    import::run_import,
    init::run_init,
    list::run_list,
    man::run_man,
    pull::run_pull,
    remove::run_remove,
    run::run_run,
    set::run_set,
    unset::run_unset,
    vars::run_vars,
};
use envlt_core::{AppService, VaultStore};
use output::OutputFormat;

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
        Commands::List { format } => run_list(&service, format),
        Commands::Remove { project, yes } => run_remove(&service, &project, yes),
        Commands::Env { command } => match command {
            EnvCommands::List { project, format } => run_env_list(&service, &project, format),
            EnvCommands::Add {
                name,
                project,
                from,
            } => run_env_add(&service, &name, &project, &from),
            EnvCommands::Remove { name, project, yes } => {
                run_env_remove(&service, &name, &project, yes)
            }
            EnvCommands::Switch { name, project } => run_env_switch(&service, &name, &project),
        },
        Commands::Vars {
            project,
            env,
            format,
        } => run_vars(&service, &project, &env, format),
        Commands::Get { key, project, env } => run_get(&service, &key, &project, &env),
        Commands::Set {
            project,
            env,
            assignment,
            secret,
            plain,
        } => run_set(&service, &project, &env, &assignment, secret, plain),
        Commands::Unset { project, env, key } => run_unset(&service, &project, &env, &key),
        Commands::Run {
            project,
            env,
            command,
        } => run_run(&service, &project, &env, &command),
        Commands::Pull { project, env, out } => run_pull(&service, &project, &env, &out),
        Commands::Generate {
            gen_type,
            list_types,
            len,
            hex,
            symbols,
            show,
            set,
            project,
            env,
            silent,
            format,
        } => run_generate(
            &service,
            GenerateOptions {
                gen_type: gen_type.as_deref(),
                list_types,
                len,
                hex,
                symbols,
                show,
                set_key: &set,
                project: &project,
                env: &env,
                silent,
                list_format: format,
            },
        ),
        Commands::Export { project, env, out } => run_export(&service, &project, &env, &out),
        Commands::Import {
            file,
            overwrite,
            dry_run,
            inspect,
        } => run_import(&service, &file, overwrite, dry_run, inspect),
        Commands::Check {
            project,
            env,
            example,
        } => run_check(&service, &project, &env, &example),
        Commands::Diff {
            project,
            env,
            other_project,
            other_env,
            example,
            format,
        } => run_diff(
            &service,
            &project,
            &env,
            &other_project,
            &other_env,
            &example,
            format,
        ),
        Commands::History {
            project,
            env,
            key,
            format,
        } => run_history(&service, &project, &env, key.as_deref(), format),
        Commands::Doctor { decrypt, format } => run_doctor(&service, decrypt, format),
        Commands::Completions { shell } => run_completions(shell),
        Commands::Man { out } => run_man(&out),
        Commands::Auth { command } => match command {
            AuthCommands::Save => run_auth_save(&service),
            AuthCommands::Clear => run_auth_clear(&service),
            AuthCommands::Status { format } => run_auth_status(&service, format),
        },
    }
}

/// Bold headers/usage, green literals (command and flag names), cyan
/// placeholders -- matches the look of `cargo`, `uv`, and `ripgrep`'s
/// `--help` output. Respects `NO_COLOR` and non-terminal output the same
/// way those tools do, since clap only applies styling when it detects an
/// interactive, color-capable terminal.
fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default().bold())
        .usage(AnsiColor::Yellow.on_default().bold())
        .literal(AnsiColor::Green.on_default().bold())
        .placeholder(AnsiColor::Cyan.on_default())
        .error(AnsiColor::Red.on_default().bold())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default())
}

#[derive(Debug, Parser)]
#[command(
    name = "envlt",
    version,
    about = "Local-first encrypted environment vault",
    long_about = "envlt stores project environment variables in an encrypted local vault, regenerates .env files when needed, and can run commands with injected variables without requiring a cloud service.",
    after_help = "Quick start:\n  envlt init\n  envlt add my-project\n  envlt vars --project my-project\n  envlt run --project my-project -- npm start\n\nRunning `envlt` with no arguments is reserved for a future interactive mode; use `envlt --help` or `envlt <command> --help` explicitly.\n\nMore: https://github.com/obsidia-systems/envlt/tree/main/docs",
    styles = styles()
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
        long_about = "Import a project into the encrypted vault from an existing .env file or bootstrap it from a .env.example file. The command also writes a .envlt-link file so the current directory can resolve the project automatically later.",
        after_help = "Examples:\n  envlt add my-project\n  envlt add my-project --file .env.production\n  envlt add my-project --from-example .env.example"
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
    #[command(
        about = "List all stored projects",
        long_about = "List every project currently stored in the vault, one per line (or as a table/JSON array, depending on --format).",
        after_help = "Examples:\n  envlt list\n  envlt list --format json"
    )]
    List {
        #[arg(long, value_enum, default_value_t = OutputFormat::Table, help = "Output format")]
        format: OutputFormat,
    },
    #[command(
        about = "Remove a stored project from the vault",
        long_about = "Remove a project from the encrypted vault. By default envlt asks for confirmation first. If the current directory has a .envlt-link that points to the removed project, envlt clears that link as part of the operation.",
        after_help = "Examples:\n  envlt remove my-project\n  envlt remove my-project --yes"
    )]
    Remove {
        #[arg(help = "Project name to remove from the vault")]
        project: String,
        #[arg(long, short = 'y', help = "Skip the confirmation prompt")]
        yes: bool,
    },
    #[command(
        about = "Manage a project's environments",
        long_about = "List, add, remove, or switch between environments (e.g. local, staging, prod) within a project. Variables are fully duplicated per environment -- there is no inheritance between them."
    )]
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
    #[command(
        about = "Show variables stored for a project",
        long_about = "Display variable names, variable types, and values for a project environment. Secret values are masked by default while Plain values remain visible.",
        after_help = "Examples:\n  envlt vars --project my-project\n  envlt vars --project my-project --env staging\n  envlt vars --project my-project --format json"
    )]
    Vars {
        #[arg(
            long,
            help = "Project to inspect; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to inspect; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Table, help = "Output format")]
        format: OutputFormat,
    },
    #[command(
        about = "Print a single variable's value",
        long_about = "Print one variable's value to stdout, unmasked, for scripting -- e.g. `export DB_PASSWORD=$(envlt get DB_PASSWORD)`. Unlike `vars`, which always masks Secret values, requesting a specific key by name is treated as an intentional reveal, the same way `generate --show` works.",
        after_help = "Examples:\n  envlt get DB_PASSWORD --project my-project\n  envlt get DB_PASSWORD --project my-project --env staging\n  export DB_PASSWORD=$(envlt get DB_PASSWORD --project my-project)"
    )]
    Get {
        #[arg(help = "Variable key to print")]
        key: String,
        #[arg(
            long,
            help = "Project to read from; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to read from; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
    },
    #[command(
        about = "Create or update a project variable",
        long_about = "Set a variable for a project using KEY=VALUE syntax. The variable type can be inferred automatically or overridden explicitly with --secret or --plain.",
        after_help = "Examples:\n  envlt set --project my-project PORT=4000\n  envlt set --project my-project --env staging DATABASE_URL=postgres://staging-host/db\n  envlt set --project my-project --secret JWT_SECRET=supersecret"
    )]
    Set {
        #[arg(
            long,
            help = "Project to update; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to update; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(long, conflicts_with = "plain", help = "Mark the variable as Secret")]
        secret: bool,
        #[arg(long, conflicts_with = "secret", help = "Mark the variable as Plain")]
        plain: bool,
        #[arg(help = "Variable assignment in KEY=VALUE format")]
        assignment: String,
    },
    #[command(
        about = "Delete a project variable",
        long_about = "Remove a variable from a project. The project can be selected explicitly with --project or resolved from .envlt-link. Its version history is kept for `envlt history`; unsetting an already-deleted key is an error.",
        after_help = "Examples:\n  envlt unset JWT_SECRET --project my-project\n  envlt unset JWT_SECRET"
    )]
    Unset {
        #[arg(
            long,
            help = "Project to update; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to update; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(help = "Variable key to delete")]
        key: String,
    },
    #[command(
        about = "Run a child process with vault variables injected",
        long_about = "Resolve a project's variables from the vault and inject them into a child process environment without writing a .env file to disk.",
        after_help = "Examples:\n  envlt run --project my-project -- npm start\n  envlt run --project my-project --env staging -- node server.js"
    )]
    Run {
        #[arg(
            long,
            help = "Project to run with; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to run with; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(
            help = "Command and arguments to execute",
            required = true,
            trailing_var_arg = true
        )]
        command: Vec<String>,
    },
    #[command(
        about = "Write a .env file from a project stored in the vault",
        long_about = "Pull a project's variables from the vault into a .env-style file on disk. This is useful for local tooling that expects a file on disk; prefer `envlt run` when a file is not required.",
        after_help = "Examples:\n  envlt pull --project my-project\n  envlt pull --project my-project --env staging --out .env.local"
    )]
    Pull {
        #[arg(
            long,
            help = "Project to materialize; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to materialize; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(
            long,
            default_value = ".env",
            help = "Output path for the rendered env file"
        )]
        out: PathBuf,
    },
    #[command(
        about = "Generate secure values and optionally store them",
        long_about = "Generate secrets or identifiers using built-in presets or custom length settings. Generated values can be printed, stored directly in the vault, or produced through a guided interactive flow.",
        after_help = "Examples:\n  envlt generate --type jwt-secret\n  envlt generate --len 32 --symbols\n  envlt generate --type jwt-secret --set JWT_SECRET --project my-project\n  envlt generate --list-types"
    )]
    Generate {
        #[arg(
            long = "type",
            help_heading = "Preset generation",
            help = "Generator preset to use"
        )]
        gen_type: Option<String>,
        #[arg(
            long,
            help_heading = "Preset generation",
            help = "List supported generator presets and exit"
        )]
        list_types: bool,
        #[arg(
            long,
            help_heading = "Custom generation",
            help = "Generate a custom value with the requested length"
        )]
        len: Option<usize>,
        #[arg(
            long,
            conflicts_with = "symbols",
            help_heading = "Custom generation",
            help = "Use a hexadecimal alphabet"
        )]
        hex: bool,
        #[arg(
            long,
            conflicts_with = "hex",
            help_heading = "Custom generation",
            help = "Include symbols in the generated value"
        )]
        symbols: bool,
        #[arg(
            long,
            conflicts_with = "silent",
            help_heading = "Storage",
            help = "Reveal the generated value even when storing it in the vault"
        )]
        show: bool,
        #[arg(
            long,
            help_heading = "Storage",
            help = "Store the generated value in the given variable key"
        )]
        set: Option<String>,
        #[arg(
            long,
            help_heading = "Storage",
            help = "Target project for storing the generated value"
        )]
        project: Option<String>,
        #[arg(
            long,
            help_heading = "Storage",
            help = "Target environment for storing the generated value; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(long, help_heading = "Output", help = "Suppress all command output")]
        silent: bool,
        #[arg(
            long,
            value_enum,
            help_heading = "Output",
            help = "Output format for --list-types"
        )]
        format: Option<OutputFormat>,
    },
    #[command(
        about = "Export a project environment to an encrypted .evlt bundle",
        long_about = "Export one environment as an encrypted, portable .evlt bundle. The bundle carries only that environment, flattened to current values (no version history, no soft-deleted variables), so sharing it can't expose more than that one environment's present state.",
        after_help = "Examples:\n  envlt export my-project --out bundle.evlt\n  envlt export my-project --env staging --out staging-bundle.evlt"
    )]
    Export {
        #[arg(help = "Project name to export")]
        project: String,
        #[arg(
            long,
            help = "Environment to export; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(long, help = "Output path for the encrypted bundle")]
        out: PathBuf,
    },
    #[command(
        about = "Import an encrypted .evlt bundle into the vault",
        long_about = "Import a project snapshot from an encrypted .evlt bundle. By default envlt refuses to overwrite an existing project unless --overwrite is provided.",
        after_help = "Examples:\n  envlt import bundle.evlt\n  envlt import bundle.evlt --overwrite\n  envlt import bundle.evlt --inspect\n  envlt import bundle.evlt --dry-run"
    )]
    Import {
        #[arg(help = "Path to the .evlt bundle to import")]
        file: PathBuf,
        #[arg(long, help = "Replace an existing project with the same name")]
        overwrite: bool,
        #[arg(
            long,
            conflicts_with = "inspect",
            help = "Validate and preview the import without writing to the vault"
        )]
        dry_run: bool,
        #[arg(
            long,
            help = "Show the bundle's unencrypted header (project name, export time) without a passphrase"
        )]
        inspect: bool,
    },
    #[command(
        about = "Check that a project satisfies a .env.example contract",
        long_about = "Verify that all variables declared in a .env.example file are present in the vault project. Exit code is 0 when complete and non-zero when variables are missing. This is useful for automation and pre-commit checks.",
        after_help = "Examples:\n  envlt check --project my-project .env.example\n  envlt check .env.example"
    )]
    Check {
        #[arg(
            long,
            help = "Project to check; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to check; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(help = "Path to the .env.example file to check against")]
        example: PathBuf,
    },
    #[command(
        about = "Compare a project against .env.example or another project/environment",
        long_about = "Produce a safe summary diff without printing secret values. Use --example to compare against a .env.example file, a second project name to compare two vault projects, and/or --other-env to compare two environments (of the same project, unless a second project name is also given).",
        after_help = "Examples:\n  envlt diff --project my-project --example .env.example\n  envlt diff --project my-project other-project\n  envlt diff --project my-project --other-env staging"
    )]
    Diff {
        #[arg(
            long,
            help = "Base project to compare; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Base environment to compare; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(
            help_heading = "Comparison target",
            help = "Other project to compare against; defaults to the base project when only --other-env is given"
        )]
        other_project: Option<String>,
        #[arg(
            long = "other-env",
            help_heading = "Comparison target",
            help = "Other environment to compare against; defaults to the base environment"
        )]
        other_env: Option<String>,
        #[arg(
            long,
            conflicts_with_all = ["other_project", "other_env"],
            help_heading = "Comparison target",
            help = "Path to a .env.example file to compare against"
        )]
        example: Option<PathBuf>,
        #[arg(
            long,
            value_enum,
            default_value_t = OutputFormat::Table,
            help_heading = "Output",
            help = "Output format"
        )]
        format: OutputFormat,
    },
    #[command(
        about = "Show the activity log for a project or variable",
        long_about = "Display a durable audit trail of variable lifecycle events. Use without a key to show the full project log, or pass a variable key to filter its history. Secret values are masked automatically.",
        after_help = "Examples:\n  envlt history --project my-project\n  envlt history --project my-project DATABASE_URL\n  envlt history --project my-project --env staging"
    )]
    History {
        #[arg(
            long,
            help = "Project to inspect; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Environment to inspect; falls back to .envlt-link, then \"local\""
        )]
        env: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Table, help = "Output format")]
        format: OutputFormat,
        #[arg(help = "Variable key to show history for")]
        key: Option<String>,
    },
    #[command(
        about = "Run local diagnostics for the vault and project link state",
        long_about = "Inspect the envlt home directory, vault presence, backup presence, and .envlt-link state. Optionally try to decrypt the vault and validate that the linked project exists.",
        after_help = "Examples:\n  envlt doctor\n  envlt doctor --decrypt"
    )]
    Doctor {
        #[arg(
            long,
            help = "Attempt to decrypt the vault and validate linked-project state"
        )]
        decrypt: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Table, help = "Output format")]
        format: OutputFormat,
    },
    #[command(
        about = "Generate shell completion scripts",
        long_about = "Generate shell completion scripts for bash, zsh, fish, PowerShell, or Elvish. Output the result to stdout and redirect it to the appropriate completion directory for your shell.",
        after_help = "Examples:\n  envlt completions zsh > ~/.zfunc/_envlt\n  envlt completions bash > /usr/local/etc/bash_completion.d/envlt"
    )]
    Completions {
        #[arg(help = "Shell to generate completions for")]
        shell: CompletionShell,
    },
    #[command(
        about = "Generate man pages for envlt and every subcommand",
        long_about = "Generate roff-format man pages (envlt.1, envlt-init.1, envlt-add.1, ...) for envlt and every subcommand, from the same definitions used to build --help, so they can never drift out of sync with it.",
        after_help = "Examples:\n  envlt man --out ./man\n  envlt man --out /usr/local/share/man/man1"
    )]
    Man {
        #[arg(
            long,
            default_value = "man",
            help = "Directory to write the generated man pages into"
        )]
        out: PathBuf,
    },
    #[command(
        about = "Manage stored vault authentication",
        long_about = "Manage the vault passphrase in the system keyring. Saved credentials are scoped to the current envlt home directory and allow later commands to run without prompting for the passphrase each time."
    )]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
}

#[derive(Debug, Subcommand)]
enum EnvCommands {
    #[command(
        about = "List a project's environments",
        after_help = "Examples:\n  envlt env list --project my-project\n  envlt env list --project my-project --format raw"
    )]
    List {
        #[arg(
            long,
            help = "Project to inspect; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Table, help = "Output format")]
        format: OutputFormat,
    },
    #[command(
        about = "Add a new environment to a project",
        long_about = "Add a new environment to a project, empty by default. With --from, seed it with another environment's current values instead -- a one-time copy, not an ongoing link: each seeded variable starts its own independent version history.",
        after_help = "Examples:\n  envlt env add staging --project my-project\n  envlt env add staging --project my-project --from local"
    )]
    Add {
        #[arg(help = "Environment name to add, e.g. staging or prod")]
        name: String,
        #[arg(
            long,
            help = "Project to update; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(
            long,
            help = "Seed the new environment with another environment's current values"
        )]
        from: Option<String>,
    },
    #[command(
        about = "Remove an environment and all its variables",
        long_about = "Remove an environment and everything in it, including every variable's version history. A project must always keep at least one environment, so removing the last one is an error. Asks for confirmation unless --yes is given.",
        after_help = "Examples:\n  envlt env remove staging --project my-project\n  envlt env remove staging --project my-project --yes"
    )]
    Remove {
        #[arg(help = "Environment name to remove")]
        name: String,
        #[arg(
            long,
            help = "Project to update; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
        #[arg(long, short = 'y', help = "Skip the confirmation prompt")]
        yes: bool,
    },
    #[command(
        about = "Set the default environment for the current directory",
        long_about = "Pin an environment as this directory's default by writing it into .envlt-link, so --env can be omitted on later commands run from here. Fails if the environment doesn't exist. Re-run it any time to switch a directory's default to a different environment.",
        after_help = "Examples:\n  envlt env switch staging --project my-project\n  envlt env switch staging"
    )]
    Switch {
        #[arg(help = "Environment name to default to, e.g. staging or prod")]
        name: String,
        #[arg(
            long,
            help = "Project to update; falls back to .envlt-link when omitted"
        )]
        project: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum AuthCommands {
    #[command(
        about = "Save the current vault passphrase to the system keyring",
        long_about = "Read the vault passphrase from ENVLT_PASSPHRASE or an interactive prompt, verify that it can decrypt the current vault, and then save it to the system keyring."
    )]
    Save,
    #[command(
        about = "Remove the stored vault passphrase from the system keyring",
        long_about = "Remove the vault passphrase previously saved with `envlt auth save` from the system keyring, scoped to the current envlt home directory. Does not modify the vault itself."
    )]
    Clear,
    #[command(
        about = "Show whether auth sources are available",
        long_about = "Report whether ENVLT_PASSPHRASE is set and whether a stored vault passphrase exists in the system keyring for the current envlt home."
    )]
    Status {
        #[arg(long, value_enum, default_value_t = OutputFormat::Table, help = "Output format")]
        format: OutputFormat,
    },
}
