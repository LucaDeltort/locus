//! `set` command implementation.

use crate::commands::{load_project, resolve_file};
use crate::output::{escape_value, Colors};
use crate::terminal::{read_interactive, SHOULD_EXIT};
use locus_core::edit;
use std::path::Path;

pub fn run(
    root: &Path,
    file: Option<&str>,
    key: &str,
    lang: Option<&str>,
    value: Option<&str>,
    backup: bool,
) -> Result<(), String> {
    let mut proj = load_project(root)?;
    let file = resolve_file(&proj, file)?;
    let c = Colors::detect();

    match (lang, value) {
        // Explicit mode: --lang + --value
        (Some(l), Some(v)) => {
            edit::set_value(&mut proj, &file, key, l, v).map_err(|e| format!("{:?}", e))?;
            locus_core::save::save_project(&proj, backup).map_err(|e| format!("{:?}", e))?;
            println!("{}✓ Set{} {} :: {}", c.green, c.reset, file, key);
            println!("  {}✓{} {}  \"{}\"", c.green, c.reset, l, escape_value(v));
        }
        // Single locale interactive: --lang without --value
        (Some(l), None) => {
            let current_val = proj
                .files
                .get(&file)
                .and_then(|f| f.locales.get(l))
                .and_then(|sf| sf.keys.get(key))
                .and_then(|k| k.value.clone());

            if let Some(v) = &current_val {
                println!(
                    "  {}{}{} already has: \"{}\"",
                    c.green,
                    l,
                    c.reset,
                    escape_value(v)
                );
            }

            let loc_file = proj.files.get(&file).unwrap();
            let source_value = loc_file
                .locales
                .get(&loc_file.source_language)
                .and_then(|sf| sf.keys.get(key))
                .and_then(|k| k.value.clone());

            let copy_val = match (&source_value, l != loc_file.source_language.as_str()) {
                (Some(sv), true) => Some(sv.as_str()),
                _ => None,
            };
            let prompt = format!("  {}{}{}: ", c.yellow, l, c.reset);
            let input = match read_interactive(&prompt, copy_val)? {
                None => return Ok(()),
                Some(v) => v,
            };

            if !input.is_empty() {
                edit::set_value(&mut proj, &file, key, l, &input)
                    .map_err(|e| format!("{:?}", e))?;
                locus_core::save::save_project(&proj, backup).map_err(|e| format!("{:?}", e))?;
                println!("\n{}✓ Set{} {} :: {}", c.green, c.reset, file, key);
                println!(
                    "  {}✓{} {}  \"{}\"",
                    c.green,
                    c.reset,
                    l,
                    escape_value(&input)
                );
            } else {
                println!("{}Skipped.{}", c.red, c.reset);
            }
        }
        // Interactive mode: no --lang, no --value
        (None, None) => {
            let loc_file = proj.files.get(&file).unwrap();
            let source_lang = loc_file.source_language.clone();
            let mut all_langs: Vec<String> = loc_file.locales.keys().cloned().collect();
            all_langs.sort();
            all_langs.sort_by_key(|l| l != &source_lang);

            let file_name = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| root.display().to_string());

            let source_value = loc_file
                .locales
                .get(&source_lang)
                .and_then(|sf| sf.keys.get(key))
                .and_then(|k| k.value.clone());

            println!("{}{} — fill all locales{}", c.cyan, key, c.reset);
            println!(
                "Press {}Enter{} to skip, {}→{} to copy source.\n",
                c.yellow, c.reset, c.yellow, c.reset
            );

            let mut updated = 0usize;

            for lang in &all_langs {
                let current_val = proj
                    .files
                    .get(&file)
                    .and_then(|f| f.locales.get(lang))
                    .and_then(|sf| sf.keys.get(key))
                    .and_then(|k| k.value.clone());

                if let Some(v) = &current_val {
                    println!(
                        "  {}✓{} {}  \"{}\"",
                        c.green,
                        c.reset,
                        lang,
                        escape_value(v)
                    );
                    continue;
                }

                let copy_val = match (&source_value, lang.as_str() != source_lang.as_str()) {
                    (Some(sv), true) => Some(sv.as_str()),
                    _ => None,
                };
                let prompt = format!("  {}{}{}: ", c.yellow, lang, c.reset);
                let input = match read_interactive(&prompt, copy_val) {
                    Ok(None) | Err(_) => break,
                    Ok(Some(v)) => v,
                };

                if SHOULD_EXIT.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                if input.is_empty() {
                    println!("    {}(skipped){}", c.red, c.reset);
                    continue;
                }

                edit::set_value(&mut proj, &file, key, lang, &input)
                    .map_err(|e| format!("{:?}", e))?;
                updated += 1;
            }

            if updated > 0 {
                locus_core::save::save_project(&proj, backup).map_err(|e| format!("{:?}", e))?;
                println!(
                    "\n{}✓ Saved{} — {} locale(s) updated in {}",
                    c.green, c.reset, updated, file_name
                );
            } else {
                println!("\n{}No locales updated.{}", c.cyan, c.reset);
            }
        }
        // Inconsistent: --value without --lang
        (None, Some(_)) => return Err("--value requires --lang".to_string()),
    }

    Ok(())
}
