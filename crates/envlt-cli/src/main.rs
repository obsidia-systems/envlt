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
    about = "Local-first encrypted environment vault"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init,
    Add {
        project: String,
        #[arg(long, default_value = ".env")]
        file: PathBuf,
        #[arg(long)]
        from_example: Option<PathBuf>,
        #[arg(long)]
        project_path: Option<PathBuf>,
    },
    List,
    Remove {
        project: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
    Doctor {
        #[arg(long)]
        decrypt: bool,
    },
    Vars {
        #[arg(long)]
        project: Option<String>,
    },
    Diff {
        #[arg(long)]
        project: Option<String>,
        other_project: Option<String>,
        #[arg(long, conflicts_with = "other_project")]
        example: Option<PathBuf>,
    },
    Gen {
        #[arg(long = "type")]
        gen_type: Option<String>,
        #[arg(long)]
        list_types: bool,
        #[arg(long)]
        len: Option<usize>,
        #[arg(long, conflicts_with = "symbols")]
        hex: bool,
        #[arg(long, conflicts_with = "hex")]
        symbols: bool,
        #[arg(long, conflicts_with = "silent")]
        show: bool,
        #[arg(long)]
        set: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        silent: bool,
    },
    Export {
        project: String,
        #[arg(long)]
        out: PathBuf,
    },
    Import {
        file: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
    Set {
        #[arg(long)]
        project: Option<String>,
        #[arg(long, conflicts_with_all = ["config", "plain"])]
        secret: bool,
        #[arg(long, conflicts_with_all = ["secret", "plain"])]
        config: bool,
        #[arg(long, conflicts_with_all = ["secret", "config"])]
        plain: bool,
        assignment: String,
    },
    Use {
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value = ".env")]
        out: PathBuf,
    },
    Run {
        #[arg(long)]
        project: Option<String>,
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
}
