//! In-memory edits: adding keys and setting values.
//!
//! All operations mutate the model only. Persistence is handled by
//! [`save`](crate::save).

use crate::model::{Key, Project};
use indexmap::IndexMap;

/// Add a new key to a file with a base-locale value (Base or `en`).
///
/// Other locales are left empty (detectable via [`missing`](crate::missing)).
/// Returns an error if the key already exists in any locale.
pub fn add_key(
    project: &mut Project,
    file: &str,
    key: &str,
    base_lang: &str,
    base_value: &str,
) -> Result<(), EditError> {
    let localization_file = project
        .files
        .get_mut(file)
        .ok_or_else(|| EditError::FileNotFound(file.to_string()))?;

    // Refuse to overwrite an existing key
    for sf in localization_file.locales.values() {
        if sf.keys.contains_key(key) {
            return Err(EditError::KeyAlreadyExists(key.to_string()));
        }
    }

    let strings_file = localization_file
        .locales
        .entry(base_lang.to_string())
        .or_insert_with(|| crate::model::StringsFile {
            lang: base_lang.to_string(),
            path: std::path::PathBuf::new(), // will be set on save
            keys: IndexMap::new(),
        });

    strings_file.keys.insert(
        key.to_string(),
        Key {
            key: key.to_string(),
            value: Some(base_value.to_string()),
            comment: None,
            state: crate::model::KeyState::Translated,
            extraction_state: None,
        },
    );

    Ok(())
}

/// Set the value of a key for a specific locale.
///
/// Creates the locale entry if it doesn't exist. Creates the key if it
/// doesn't exist in that locale.
pub fn set_value(
    project: &mut Project,
    file: &str,
    key: &str,
    lang: &str,
    value: &str,
) -> Result<(), EditError> {
    let localization_file = project
        .files
        .get_mut(file)
        .ok_or_else(|| EditError::FileNotFound(file.to_string()))?;

    let strings_file = localization_file
        .locales
        .entry(lang.to_string())
        .or_insert_with(|| crate::model::StringsFile {
            lang: lang.to_string(),
            path: std::path::PathBuf::new(),
            keys: IndexMap::new(),
        });

    strings_file.keys.insert(
        key.to_string(),
        Key {
            key: key.to_string(),
            value: Some(value.to_string()),
            comment: None,
            state: crate::model::KeyState::Translated,
            extraction_state: None,
        },
    );

    Ok(())
}

#[derive(Debug)]
pub enum EditError {
    FileNotFound(String),
    KeyAlreadyExists(String),
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileFormat, LocalizationFile};
    use crate::test_helpers;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn add_key_creates_locale_if_needed() {
        let mut project = test_helpers::empty_project();
        project.files.insert(
            "Localizable".to_string(),
            LocalizationFile {
                name: "Localizable".into(),
                locales: HashMap::new(),
                format: FileFormat::Strings,
                path: PathBuf::new(),
                source_language: "en".into(),
            },
        );

        add_key(&mut project, "Localizable", "new.key", "en", "New Value").unwrap();
        let loc = project.files.get("Localizable").unwrap();
        assert!(loc.locales.contains_key("en"));
        assert_eq!(
            loc.locales["en"]
                .keys
                .get("new.key")
                .unwrap()
                .value
                .as_deref(),
            Some("New Value")
        );
    }

    #[test]
    fn add_key_returns_error_for_missing_file() {
        let mut project = test_helpers::empty_project();
        let err = add_key(&mut project, "Nonexistent", "key", "en", "val").unwrap_err();
        assert!(matches!(err, EditError::FileNotFound(_)));
    }

    #[test]
    fn add_key_refuses_to_overwrite_existing() {
        let mut project = test_helpers::project_with_en();
        let err = add_key(&mut project, "Localizable", "existing", "en", "new val").unwrap_err();
        assert!(matches!(err, EditError::KeyAlreadyExists(_)));
    }

    #[test]
    fn set_value_creates_locale_and_key() {
        let mut project = test_helpers::project_with_en();

        set_value(&mut project, "Localizable", "existing", "fr", "Bonjour").unwrap();
        let fr = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("fr")
            .unwrap();
        assert_eq!(
            fr.keys.get("existing").unwrap().value.as_deref(),
            Some("Bonjour")
        );
    }

    #[test]
    fn set_value_overwrites_existing() {
        let mut project = test_helpers::project_with_en();

        set_value(&mut project, "Localizable", "existing", "en", "Updated").unwrap();
        let en = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("en")
            .unwrap();
        assert_eq!(
            en.keys.get("existing").unwrap().value.as_deref(),
            Some("Updated")
        );
    }

    #[test]
    fn set_value_creates_new_key_in_existing_locale() {
        let mut project = test_helpers::project_with_en();

        set_value(&mut project, "Localizable", "brand.new", "en", "Brand New").unwrap();
        let en = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("en")
            .unwrap();
        assert_eq!(
            en.keys.get("brand.new").unwrap().value.as_deref(),
            Some("Brand New")
        );
    }

    // --- set_value on .xcstrings format ---

    fn xcstrings_project_with_new_state() -> Project {
        use crate::xcstrings;
        let input = r#"{"sourceLanguage":"en","version":"1","strings":{"k":{"localizations":{"en":{"stringUnit":{"state":"new","value":""}}}}}}"#;
        let parsed = xcstrings::parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        let mut proj = Project {
            root: PathBuf::from("."),
            files: HashMap::new(),
        };
        proj.files
            .insert("Localizable".to_string(), parsed.localization_file);
        proj
    }

    #[test]
    fn set_value_on_xcstrings_marks_as_translated() {
        use crate::model::KeyState;
        let mut project = xcstrings_project_with_new_state();
        set_value(&mut project, "Localizable", "k", "en", "Hello").unwrap();
        let en = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("en")
            .unwrap();
        assert_eq!(en.keys.get("k").unwrap().value.as_deref(), Some("Hello"));
        assert_eq!(en.keys.get("k").unwrap().state, KeyState::Translated);
    }

    #[test]
    fn add_key_on_xcstrings_creates_key_and_locale() {
        use crate::model::{FileFormat, LocalizationFile};
        let mut project = test_helpers::empty_project();
        project.files.insert(
            "Localizable".to_string(),
            LocalizationFile {
                name: "Localizable".into(),
                locales: HashMap::new(),
                format: FileFormat::XcStrings,
                path: PathBuf::from("Localizable.xcstrings"),
                source_language: "en".into(),
            },
        );

        add_key(&mut project, "Localizable", "new.key", "en", "New Value").unwrap();
        let loc = project.files.get("Localizable").unwrap();
        assert!(loc.locales.contains_key("en"));
        assert_eq!(
            loc.locales["en"]
                .keys
                .get("new.key")
                .unwrap()
                .value
                .as_deref(),
            Some("New Value")
        );
    }
}
