//! `search` command implementation.

use crate::commands::load_project;
use crate::output::{escape_value, Colors};
use locus_core::search::{self, KeyOverview};

pub fn run(
    root: &std::path::Path,
    key: Option<String>,
    text: Option<String>,
    langs: Option<String>,
) -> Result<(), String> {
    let proj = load_project(root)?;
    let c = Colors::detect();

    let filter_langs: Option<Vec<String>> =
        langs.map(|s| s.split(',').map(|l| l.trim().to_string()).collect());

    let results = match (key, text) {
        (Some(k), _) => search::search_by_key(&proj, &k),
        (_, Some(t)) => search::search_by_text(&proj, &t),
        _ => return Err("specify --key or --text".to_string()),
    };

    if results.is_empty() {
        println!("No results.");
        return Ok(());
    }

    let file_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string());

    println!();

    for r in &results {
        print_overview(r, &filter_langs, &c);
    }

    println!(
        "\n{}{} key(s) found in {}{}",
        c.cyan,
        results.len(),
        file_name,
        c.reset
    );

    Ok(())
}

fn print_overview(r: &KeyOverview, filter: &Option<Vec<String>>, c: &Colors) {
    let mut langs: Vec<&String> = r.translations.keys().collect();
    langs.sort();

    let visible: Vec<&String> = match filter {
        Some(f) => langs
            .into_iter()
            .filter(|l| f.iter().any(|x| x == l.as_str()))
            .collect(),
        None => langs,
    };

    if visible.is_empty() {
        return;
    }

    let max_lang_len = visible.iter().map(|l| l.len()).max().unwrap_or(0);

    println!();
    println!("{}{}{}", c.yellow, r.key, c.reset);
    for lang in visible {
        let val = r.translations.get(lang).cloned().flatten();
        match val {
            Some(v) => {
                let escaped = escape_value(&v);
                println!(
                    "  {}✓{} {:width$}  {}\"{}\"{}",
                    c.green,
                    c.reset,
                    lang,
                    c.green,
                    escaped,
                    c.reset,
                    width = max_lang_len
                );
            }
            None => {
                println!(
                    "  {}✗{} {:width$}  {}(missing){}",
                    c.red,
                    c.reset,
                    lang,
                    c.red,
                    c.reset,
                    width = max_lang_len
                );
            }
        }
    }
}
