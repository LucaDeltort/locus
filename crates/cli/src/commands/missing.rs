//! `missing` command implementation.

use crate::cli::OutputFormat;
use crate::commands::load_project;
use crate::output::{escape_json, escape_value, Colors};
use locus_core::missing;

pub fn run(root: &std::path::Path, lang: &str, output: Option<OutputFormat>) -> Result<(), String> {
    let proj = load_project(root)?;
    let missing_keys = missing::find_missing(&proj, lang);
    let c = Colors::detect();

    if missing_keys.is_empty() {
        println!("No missing keys for locale '{}'.", lang);
        return Ok(());
    }

    match output {
        Some(OutputFormat::Csv) => {
            println!("file,key,ref_lang,ref_value,present_in");
            for mk in &missing_keys {
                let ref_val = mk.ref_value.as_deref().unwrap_or("");
                println!(
                    "{},{},{},\"{}\",\"{}\"",
                    mk.file,
                    mk.key,
                    mk.ref_lang,
                    ref_val.replace('"', "\"\""),
                    mk.present_in.join("; ")
                );
            }
        }
        Some(OutputFormat::Json) => {
            println!("[");
            for (i, mk) in missing_keys.iter().enumerate() {
                let present_in: Vec<String> =
                    mk.present_in.iter().map(|l| format!("\"{}\"", l)).collect();
                let ref_val = match &mk.ref_value {
                    Some(v) => format!("\"{}\"", escape_json(v)),
                    None => "null".to_string(),
                };
                let obj = format!(
                    "  {{\"file\": \"{}\", \"key\": \"{}\", \"refLang\": \"{}\", \"refValue\": {}, \"presentIn\": [{}]}}",
                    mk.file,
                    mk.key,
                    mk.ref_lang,
                    ref_val,
                    present_in.join(", ")
                );
                if i + 1 < missing_keys.len() {
                    println!("{},", obj);
                } else {
                    println!("{}", obj);
                }
            }
            println!("]");
        }
        None => {
            let file_name = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| root.display().to_string());

            let mut current_file = String::new();
            let max_key_len = missing_keys
                .iter()
                .map(|mk| if mk.key.is_empty() { 11 } else { mk.key.len() })
                .max()
                .unwrap_or(0);

            println!();
            for mk in &missing_keys {
                if mk.file != current_file {
                    if !current_file.is_empty() {
                        println!();
                    }
                    println!("{}{}{}", c.yellow, mk.file, c.reset);
                    current_file = mk.file.clone();
                }
                let ref_val = mk
                    .ref_value
                    .as_deref()
                    .map(escape_value)
                    .unwrap_or_default();
                let display_key = if mk.key.is_empty() {
                    "(empty key)".to_string()
                } else {
                    mk.key.clone()
                };
                println!(
                    "  {}✗{} {:width$}  {}{}{}  \"{}\"",
                    c.red,
                    c.reset,
                    display_key,
                    c.green,
                    mk.ref_lang,
                    c.reset,
                    ref_val,
                    width = max_key_len
                );
            }
            println!(
                "\n{}{} missing key(s) for [{}] in {}{}",
                c.cyan,
                missing_keys.len(),
                lang,
                file_name,
                c.reset
            );
        }
    }

    Ok(())
}
