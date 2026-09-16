//! FFI layer for Swift/SwiftUI consumption via UniFFI.
//!
//! Wraps the core engine with UniFFI-compatible types. The heavy lifting
//! (parsing, searching, saving) stays in Rust; SwiftUI only displays results.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::{edit, missing, model, project, save, search};

/// Errors surfaced to Swift.
#[derive(Debug, uniffi::Error)]
pub enum LocusError {
    /// The path does not exist or is not accessible.
    NotFound { path: String },
    /// A file was not found within the project.
    FileNotFound { name: String },
    /// A key already exists when trying to add it.
    KeyAlreadyExists { key: String },
    /// A key was not found when trying to delete it.
    KeyNotFound { key: String },
    /// An IO error occurred during scanning or saving.
    Io { message: String },
    /// A parse error in a `.xcstrings` file.
    Parse { message: String },
}

impl std::fmt::Display for LocusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocusError::NotFound { path } => write!(f, "Path not found: {path}"),
            LocusError::FileNotFound { name } => write!(f, "File not found: {name}"),
            LocusError::KeyAlreadyExists { key } => write!(f, "Key already exists: {key}"),
            LocusError::KeyNotFound { key } => write!(f, "Key not found: {key}"),
            LocusError::Io { message } => write!(f, "IO error: {message}"),
            LocusError::Parse { message } => write!(f, "Parse error: {message}"),
        }
    }
}

impl From<project::ScanError> for LocusError {
    fn from(e: project::ScanError) -> Self {
        match e {
            project::ScanError::NotFound(p) => LocusError::NotFound { path: p },
            project::ScanError::Io(e) => LocusError::Io { message: format!("{e}") },
            project::ScanError::Parse(m) => LocusError::Parse { message: m },
        }
    }
}

impl From<edit::EditError> for LocusError {
    fn from(e: edit::EditError) -> Self {
        match e {
            edit::EditError::FileNotFound(n) => LocusError::FileNotFound { name: n },
            edit::EditError::KeyAlreadyExists(k) => LocusError::KeyAlreadyExists { key: k },
            edit::EditError::KeyNotFound(k) => LocusError::KeyNotFound { key: k },
        }
    }
}

impl From<save::SaveError> for LocusError {
    fn from(e: save::SaveError) -> Self {
        match e {
            save::SaveError::Io(e) => LocusError::Io { message: format!("{e}") },
        }
    }
}

/// A loaded localization project — opaque handle for Swift.
///
/// Uses interior mutability (`Mutex`) because UniFFI objects are shared via
/// `Arc`, which forbids `&mut self`.
#[derive(uniffi::Object)]
pub struct LocusProject {
    inner: Mutex<model::Project>,
}

/// Sidebar info for a logical file.
#[derive(uniffi::Record)]
pub struct FileInfo {
    pub name: String,
    /// "strings" or "xcstrings".
    pub format: String,
    pub locales: Vec<String>,
    pub source_language: String,
}

/// One row in the central key table.
#[derive(uniffi::Record)]
pub struct KeyRow {
    pub key: String,
    /// Locale code → optional value (None = key absent for that locale).
    pub translations: HashMap<String, Option<String>>,
    /// Locales where this key is missing.
    pub missing_locales: Vec<String>,
}

/// A single missing-translation entry.
#[derive(uniffi::Record)]
pub struct MissingKeyInfo {
    pub file: String,
    pub key: String,
    pub ref_lang: String,
    pub ref_value: Option<String>,
}

/// Load a project from disk (directory or single `.xcstrings` file).
#[uniffi::export]
pub fn load_project(path: String) -> Result<LocusProject, LocusError> {
    let p = PathBuf::from(&path);
    let proj = project::scan_project(&p)?;
    Ok(LocusProject {
        inner: Mutex::new(proj),
    })
}

#[uniffi::export]
impl LocusProject {
    /// Root directory the project was scanned from.
    pub fn root_path(&self) -> String {
        self.inner.lock().unwrap().root.display().to_string()
    }

    /// All logical files, sorted alphabetically.
    pub fn files(&self) -> Vec<FileInfo> {
        let proj = self.inner.lock().unwrap();
        let mut result: Vec<FileInfo> = proj
            .files
            .values()
            .map(|f| FileInfo {
                name: f.name.clone(),
                format: match f.format {
                    model::FileFormat::Strings => "strings".into(),
                    model::FileFormat::XcStrings => "xcstrings".into(),
                },
                locales: {
                    let mut v: Vec<String> = f.locales.keys().cloned().collect();
                    v.sort();
                    v
                },
                source_language: f.source_language.clone(),
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// All keys for a given file, preserving insertion order from the
    /// source language first, then any extra keys from other locales.
    pub fn keys(&self, file_name: &str) -> Vec<KeyRow> {
        let proj = self.inner.lock().unwrap();
        let file = match proj.files.get(file_name) {
            Some(f) => f,
            None => return vec![],
        };

        // Collect keys in order: source language first, then others
        let mut ordered_keys: Vec<String> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();

        if let Some(sf) = file.locales.get(&file.source_language) {
            for k in sf.keys.keys() {
                if seen.insert(k.as_str()) {
                    ordered_keys.push(k.clone());
                }
            }
        }
        for sf in file.locales.values() {
            for k in sf.keys.keys() {
                if seen.insert(k.as_str()) {
                    ordered_keys.push(k.clone());
                }
            }
        }

        ordered_keys
            .into_iter()
            .map(|key| {
                let mut translations = HashMap::new();
                let mut missing = Vec::new();
                for (lang, sf) in &file.locales {
                    match sf.keys.get(&key) {
                        Some(k) => {
                            translations.insert(lang.clone(), k.value.clone());
                        }
                        None => {
                            translations.insert(lang.clone(), None);
                            missing.push(lang.clone());
                        }
                    }
                }
                KeyRow {
                    key,
                    translations,
                    missing_locales: missing,
                }
            })
            .collect()
    }

    /// Search keys by substring match on the key name.
    pub fn search_by_key(&self, query: &str) -> Vec<KeyRow> {
        let proj = self.inner.lock().unwrap();
        search::search_by_key(&proj, query)
            .into_iter()
            .map(key_row_from_overview)
            .collect()
    }

    /// Search keys by substring match on any translated value.
    pub fn search_by_text(&self, query: &str) -> Vec<KeyRow> {
        let proj = self.inner.lock().unwrap();
        search::search_by_text(&proj, query)
            .into_iter()
            .map(key_row_from_overview)
            .collect()
    }

    /// Find all keys missing for the given locale.
    pub fn find_missing(&self, lang: &str) -> Vec<MissingKeyInfo> {
        let proj = self.inner.lock().unwrap();
        missing::find_missing(&proj, lang)
            .into_iter()
            .map(|m| MissingKeyInfo {
                file: m.file,
                key: m.key,
                ref_lang: m.ref_lang,
                ref_value: m.ref_value,
            })
            .collect()
    }

    /// Set the value of a key for a specific locale.
    /// Creates the locale/key if absent.
    pub fn set_value(
        &self,
        file: &str,
        key: &str,
        lang: &str,
        value: &str,
    ) -> Result<(), LocusError> {
        let mut proj = self.inner.lock().unwrap();
        edit::set_value(&mut proj, file, key, lang, value)?;
        Ok(())
    }

    /// Add a new key with a base-locale value.
    /// Returns an error if the key already exists.
    pub fn add_key(
        &self,
        file: &str,
        key: &str,
        base_lang: &str,
        base_value: &str,
    ) -> Result<(), LocusError> {
        let mut proj = self.inner.lock().unwrap();
        edit::add_key(&mut proj, file, key, base_lang, base_value)?;
        Ok(())
    }

    /// Delete a key from all locales of a file.
    pub fn delete_key(&self, file: &str, key: &str) -> Result<(), LocusError> {
        let mut proj = self.inner.lock().unwrap();
        edit::delete_key(&mut proj, file, key)?;
        Ok(())
    }

    /// Save all modified files atomically. Optional `.bak` backup.
    pub fn save(&self, backup: bool) -> Result<(), LocusError> {
        let proj = self.inner.lock().unwrap();
        save::save_project(&proj, backup)?;
        Ok(())
    }
}

fn key_row_from_overview(ov: search::KeyOverview) -> KeyRow {
    let mut missing = Vec::new();
    for (lang, val) in &ov.translations {
        if val.is_none() {
            missing.push(lang.clone());
        }
    }
    KeyRow {
        key: ov.key,
        translations: ov.translations,
        missing_locales: missing,
    }
}
