//! Shared test fixtures for the locus-core test suite.
//!
//! Provides builders that construct in-memory `Project` values without
//! touching the filesystem, so every module's tests can share the same
//! baseline without copy-pasting struct literals.

use crate::model::{FileFormat, Key, KeyState, LocalizationFile, Project, StringsFile};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;

/// Build a single `Key` with sensible defaults (translated state, no comment).
fn key(name: &str, value: Option<&str>) -> Key {
    Key {
        key: name.to_string(),
        value: value.map(|s| s.to_string()),
        comment: None,
        state: KeyState::Translated,
        extraction_state: None,
    }
}

/// Insert a key into an IndexMap.
fn insert(map: &mut IndexMap<String, Key>, name: &str, value: Option<&str>) {
    map.insert(name.to_string(), key(name, value));
}

/// Build a `StringsFile` for a locale from a list of (key, value) pairs.
#[allow(dead_code)]
fn strings_file(lang: &str, entries: &[(&str, Option<&str>)]) -> StringsFile {
    let mut keys = IndexMap::new();
    for (k, v) in entries {
        insert(&mut keys, k, *v);
    }
    StringsFile {
        lang: lang.to_string(),
        path: PathBuf::from(format!("{lang}.lproj/Localizable.strings")),
        keys,
    }
}

/// Wrap a set of locales into a `.strings`-format `LocalizationFile`.
fn localization_file(locales: HashMap<String, StringsFile>) -> LocalizationFile {
    LocalizationFile {
        name: "Localizable".to_string(),
        locales,
        format: FileFormat::Strings,
        path: PathBuf::from("."),
        source_language: "en".to_string(),
    }
}

// --- Reusable project builders -------------------------------------------

/// A project with two locales (`en`, `fr`) where `fr` is missing one key.
///
/// - `en`: `login.button` = "Sign In", `logout.button` = "Log Out"
/// - `fr`: `login.button` = "Connexion"
///
/// This is the baseline used by `search`, `missing`, and integration tests.
pub fn two_locale_project() -> Project {
    let mut en_keys = IndexMap::new();
    insert(&mut en_keys, "login.button", Some("Sign In"));
    insert(&mut en_keys, "logout.button", Some("Log Out"));

    let mut fr_keys = IndexMap::new();
    insert(&mut fr_keys, "login.button", Some("Connexion"));

    let mut locales = HashMap::new();
    locales.insert(
        "en".to_string(),
        StringsFile {
            lang: "en".into(),
            path: PathBuf::from("en.lproj/Localizable.strings"),
            keys: en_keys,
        },
    );
    locales.insert(
        "fr".to_string(),
        StringsFile {
            lang: "fr".into(),
            path: PathBuf::from("fr.lproj/Localizable.strings"),
            keys: fr_keys,
        },
    );

    Project {
        root: PathBuf::from("."),
        files: {
            let mut f = HashMap::new();
            f.insert("Localizable".to_string(), localization_file(locales));
            f
        },
    }
}

/// An empty project with no files.
pub fn empty_project() -> Project {
    Project {
        root: PathBuf::from("."),
        files: HashMap::new(),
    }
}

/// A project with a single `en` locale containing one existing key.
pub fn project_with_en() -> Project {
    let mut en_keys = IndexMap::new();
    insert(&mut en_keys, "existing", Some("Hello"));

    let mut locales = HashMap::new();
    locales.insert(
        "en".to_string(),
        StringsFile {
            lang: "en".into(),
            path: PathBuf::from("en.lproj/Localizable.strings"),
            keys: en_keys,
        },
    );

    Project {
        root: PathBuf::from("."),
        files: {
            let mut f = HashMap::new();
            f.insert("Localizable".to_string(), localization_file(locales));
            f
        },
    }
}
