//! In-memory representation of a loaded localization project.

use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;

/// A localization project: all localization files found under a root directory.
#[derive(Debug, Clone)]
pub struct Project {
    /// Root directory the project was scanned from.
    pub root: PathBuf,
    /// Files grouped by logical name (e.g. "Localizable", "Errors").
    pub files: HashMap<String, LocalizationFile>,
}

/// The on-disk format of a localization file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Classic `.strings` files inside `*.lproj` directories.
    Strings,
    /// Single-file String Catalog (`.xcstrings`, JSON-based).
    XcStrings,
}

/// A logical localization file across all detected locales.
///
/// For `.strings`: one entry per locale in `locales`, each with its own path.
/// For `.xcstrings`: one entry per locale, all sharing the same JSON file
/// (the path is stored here, not per-locale).
#[derive(Debug, Clone)]
pub struct LocalizationFile {
    /// Logical name derived from the filename without extension.
    pub name: String,
    /// Per-locale content, keyed by locale code.
    pub locales: HashMap<String, StringsFile>,
    /// On-disk format of this file.
    pub format: FileFormat,
    /// Path to the source file (the `.xcstrings` file itself for that format,
    /// or the first discovered `.strings` path for `.strings` format).
    pub path: PathBuf,
    /// Source language (only meaningful for `.xcstrings`; defaults to "en"
    /// for `.strings`).
    pub source_language: String,
}

/// A single locale's content within a logical file.
#[derive(Debug, Clone)]
pub struct StringsFile {
    /// Locale code (e.g. "fr", "en", "Base").
    pub lang: String,
    /// Full path on disk (for `.strings`; empty for `.xcstrings`).
    pub path: PathBuf,
    /// Keys in insertion order (preserved on save).
    pub keys: IndexMap<String, Key>,
}

/// One localization key and its value within a single locale.
#[derive(Debug, Clone)]
pub struct Key {
    /// The key string itself (e.g. "auth.login.button").
    pub key: String,
    /// Translated value, or `None` if the key exists but has no value yet.
    pub value: Option<String>,
    /// Optional comment attached to the key (`/* ... */`).
    pub comment: Option<String>,
    /// Translation state (only meaningful for `.xcstrings`).
    /// Defaults to "translated" for `.strings` files.
    pub state: KeyState,
    /// Extraction state from source code (only meaningful for `.xcstrings`).
    /// e.g. "manual", "stale". Preserved on roundtrip.
    pub extraction_state: Option<String>,
}

/// Translation state of a key, as used by String Catalogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyState {
    #[default]
    Translated,
    New,
    Stale,
    NeedsReview,
}
