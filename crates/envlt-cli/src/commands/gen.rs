use std::process::ExitCode;

use anyhow::Result;
use envlt_core::{generate_custom_value, supported_gen_types, AppService, Charset, GenType};

use crate::cli::{print_success, read_gen_type, read_passphrase};

const DEFAULT_GEN_TYPE: &str = "token";

pub struct GenOptions<'a> {
    pub gen_type: Option<&'a str>,
    pub list_types: bool,
    pub len: Option<usize>,
    pub hex: bool,
    pub symbols: bool,
    pub set_key: &'a Option<String>,
    pub project: &'a Option<String>,
    pub silent: bool,
}

impl<'a> GenOptions<'a> {
    fn custom_mode_enabled(&self) -> bool {
        self.len.is_some()
    }

    fn resolve_gen_type(&self) -> Result<String> {
        match self.gen_type {
            Some(gen_type) => Ok(gen_type.to_owned()),
            None => read_gen_type(DEFAULT_GEN_TYPE),
        }
    }
}

pub fn run_gen(service: &AppService, options: GenOptions<'_>) -> Result<ExitCode> {
    if options.list_types {
        for supported in supported_gen_types() {
            println!("{}", supported.as_str());
        }
        return Ok(ExitCode::SUCCESS);
    }

    let selected_gen_type = if options.custom_mode_enabled() {
        None
    } else {
        Some(options.resolve_gen_type()?)
    };

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

    match options.set_key {
        Some(key) => {
            let passphrase = read_passphrase(false)?;
            let project = service.resolve_project_name(options.project.as_deref(), None)?;
            let var_type = if options.custom_mode_enabled() {
                None
            } else {
                let gen_type = selected_gen_type
                    .as_deref()
                    .expect("non-custom mode always resolves a generator type");
                Some(GenType::parse(gen_type)?.default_var_type())
            };
            service.set_variable(&project, key, &generated_value, var_type, &passphrase)?;
            if options.silent {
                print_success("Value generated and saved.")?;
            } else {
                println!("{generated_value}");
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
