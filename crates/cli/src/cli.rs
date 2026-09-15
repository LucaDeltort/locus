//! Clap CLI definition.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "locus",
    version,
    about = "iOS/macOS .strings localization manager"
)]
pub struct Cli {
    /// Path to the project root or .xcstrings file.
    pub path: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search by key or by text in values.
    Search {
        /// Search by substring in key names.
        #[arg(long)]
        key: Option<String>,
        /// Search by substring in values (all languages).
        #[arg(long)]
        text: Option<String>,
        /// Only show these locales (comma-separated, e.g. "en,fr").
        #[arg(long)]
        langs: Option<String>,
    },
    /// List keys missing for a locale.
    Missing {
        /// Target locale (e.g. "fr").
        #[arg(long)]
        lang: String,
        /// Optional export format.
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    /// Set a value for a key/locale.
    Set {
        /// Logical filename (e.g. "Localizable"). Auto-detected if only one file.
        #[arg(long)]
        file: Option<String>,
        /// Key to set.
        #[arg(long)]
        key: String,
        /// Target locale (omit for interactive mode: fill all locales).
        #[arg(long)]
        lang: Option<String>,
        /// Value to set. Required with --lang, ignored in interactive mode.
        #[arg(long)]
        value: Option<String>,
        /// Save backup before writing (.bak).
        #[arg(long)]
        backup: bool,
    },
    /// Add a new key with a base-locale value (e.g. "en" or "Base").
    AddKey {
        /// Logical filename (e.g. "Localizable"). Auto-detected if only one file.
        #[arg(long)]
        file: Option<String>,
        /// Key to add.
        #[arg(long)]
        key: String,
        /// Base locale for the initial value (e.g. "en", "Base").
        #[arg(long)]
        lang: String,
        /// Initial value in the base locale.
        #[arg(long)]
        value: String,
        /// Save backup before writing (.bak).
        #[arg(long)]
        backup: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Csv,
    Json,
}
