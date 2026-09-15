//! Locus CLI — thin wrapper over `locus-core`.
//!
//! Usage:
//! ```text
//! locus <PATH> <COMMAND>
//! ```

mod cli;
mod commands;
mod output;
mod terminal;

use clap::Parser;
use cli::{Cli, Commands};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = PathBuf::from(&cli.path);

    match cli.command {
        Commands::Search { key, text, langs } => {
            match commands::search::run(&root, key, text, langs) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Missing { lang, output } => match commands::missing::run(&root, &lang, output) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::Set {
            file,
            key,
            lang,
            value,
            backup,
        } => {
            match commands::set::run(
                &root,
                file.as_deref(),
                &key,
                lang.as_deref(),
                value.as_deref(),
                backup,
            ) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::AddKey {
            file,
            key,
            lang,
            value,
            backup,
        } => match commands::add_key::run(&root, file.as_deref(), &key, &lang, &value, backup) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}
