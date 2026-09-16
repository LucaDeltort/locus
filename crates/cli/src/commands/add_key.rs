//! `add-key` command implementation.

use crate::commands::{load_project, resolve_file};
use crate::output::{escape_value, Colors};
use locus_core::edit;

pub fn run(
    root: &std::path::Path,
    file: Option<&str>,
    key: &str,
    lang: &str,
    value: &str,
    backup: bool,
) -> Result<(), String> {
    let mut proj = load_project(root)?;
    let file = resolve_file(&proj, file)?;

    edit::add_key(&mut proj, &file, key, lang, value).map_err(|e| match e {
        edit::EditError::FileNotFound(f) => format!("file '{}' not found", f),
        edit::EditError::KeyAlreadyExists(k) => format!("key '{}' already exists in '{}'", k, file),
        edit::EditError::KeyNotFound(k) => format!("key '{}' not found in '{}'", k, file),
    })?;

    locus_core::save::save_project(&proj, backup).map_err(|e| format!("{:?}", e))?;

    let c = Colors::detect();
    println!("{}✓ Added{} {} :: {}", c.green, c.reset, file, key);
    println!(
        "  {}✓{} {}  \"{}\"",
        c.green,
        c.reset,
        lang,
        escape_value(value)
    );
    Ok(())
}
