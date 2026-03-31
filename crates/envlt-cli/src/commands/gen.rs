use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{generate_custom_value, supported_gen_types, AppService, Charset, GenType};
use serde_json::to_string_pretty;

use crate::cli::{
    print_success, read_gen_project, read_gen_save_choice, read_gen_set_key, read_gen_type,
    read_passphrase,
};
use crate::output::{render_raw_rows, render_table, OutputFormat};

const DEFAULT_GEN_TYPE: &str = "token";

pub struct GenOptions<'a> {
    pub gen_type: Option<&'a str>,
    pub list_types: bool,
    pub len: Option<usize>,
    pub hex: bool,
    pub symbols: bool,
    pub show: bool,
    pub set_key: &'a Option<String>,
    pub project: &'a Option<String>,
    pub silent: bool,
    pub list_format: Option<OutputFormat>,
}

impl GenOptions<'_> {
    fn custom_mode_enabled(&self) -> bool {
        self.len.is_some()
    }

    fn resolve_gen_type(&self) -> Result<String> {
        match self.gen_type {
            Some(gen_type) => Ok(gen_type.to_owned()),
            None => read_gen_type(DEFAULT_GEN_TYPE),
        }
    }

    fn interactive_preset_mode(&self) -> bool {
        self.gen_type.is_none() && !self.custom_mode_enabled()
    }
}

pub fn run_gen(service: &AppService, options: GenOptions<'_>) -> Result<ExitCode> {
    if options.list_types {
        let supported = supported_gen_types()
            .iter()
            .map(|gen_type| gen_type.as_str().to_owned())
            .collect::<Vec<_>>();

        match options.list_format.unwrap_or(OutputFormat::Table) {
            OutputFormat::Table => {
                let rows = supported
                    .iter()
                    .map(|gen_type| vec![gen_type.clone()])
                    .collect::<Vec<_>>();
                println!("{}", render_table(&["type"], &rows));
            }
            OutputFormat::Raw => {
                let rows = supported
                    .iter()
                    .map(|gen_type| vec![gen_type.clone()])
                    .collect::<Vec<_>>();
                println!("{}", render_raw_rows(&rows));
            }
            OutputFormat::Json => {
                println!("{}", to_string_pretty(&supported)?);
            }
        }

        return Ok(ExitCode::SUCCESS);
    }

    let selected_gen_type = if options.custom_mode_enabled() {
        None
    } else {
        Some(options.resolve_gen_type()?)
    };

    let save_target = resolve_save_target(service, &options)?;

    let generated_value = match options.len {
        Some(len) => {
            let charset = match (options.hex, options.symbols) {
                (true, false) => Charset::Hex,
                (false, true) => Charset::Symbols,
                (false, false) => Charset::Alnum,
                (true, true) => unreachable!("clap enforces conflicts"),
            };
            generate_custom_value(len, charset)?
        }
        None => {
            let gen_type = selected_gen_type
                .as_deref()
                .expect("non-custom mode always resolves a generator type");
            let parsed_type = GenType::parse(gen_type)?;
            service.generate_value(parsed_type)
        }
    };

    match save_target {
        Some((key, project)) => {
            let passphrase = read_passphrase(service.store(), false)?;
            let var_type = if options.custom_mode_enabled() {
                None
            } else {
                let gen_type = selected_gen_type
                    .as_deref()
                    .expect("non-custom mode always resolves a generator type");
                Some(GenType::parse(gen_type)?.default_var_type())
            };
            service.set_variable(&project, &key, &generated_value, var_type, &passphrase)?;
            if options.silent {
                // Explicit silent mode suppresses all output, even after a successful save.
            } else if options.show {
                println!("{generated_value}");
            } else {
                print_success("Value generated and saved.")?;
            }
        }
        None => {
            if !options.silent {
                println!("{generated_value}");
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn resolve_save_target(
    service: &AppService,
    options: &GenOptions<'_>,
) -> Result<Option<(String, String)>> {
    if let Some(key) = options.set_key {
        let project = service.resolve_project_name(options.project.as_deref(), None)?;
        return Ok(Some((key.to_owned(), project)));
    }

    if !options.interactive_preset_mode() {
        return Ok(None);
    }

    if !read_gen_save_choice()? {
        return Ok(None);
    }

    let key = read_gen_set_key()?;
    let project = match service.resolve_project_name(None, None) {
        Ok(project) => project,
        Err(_) => {
            let interactive_project = read_gen_project()?;
            service.resolve_project_name(interactive_project.as_deref(), None)?
        }
    };
    Ok(Some((key, project)))
}
