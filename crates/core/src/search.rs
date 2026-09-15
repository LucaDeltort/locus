//! Search operations over a loaded project.

use crate::model::Project;
use std::collections::HashMap;

/// Aggregated view of one key across all locales of a logical file.
#[derive(Debug, Clone)]
pub struct KeyOverview {
    /// The logical file name (e.g. "Localizable").
    pub file: String,
    /// The key string.
    pub key: String,
    /// `(lang, value)` pairs — `None` means the key is missing for that locale.
    pub translations: HashMap<String, Option<String>>,
}

/// Find keys by exact match or prefix.
///
/// Returns all keys whose name starts with `query` across all files.
/// An empty query returns nothing.
pub fn search_by_key(project: &Project, query: &str) -> Vec<KeyOverview> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for (file_name, localization_file) in &project.files {
        // Collect all known keys across all locales
        let mut seen_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for strings_file in localization_file.locales.values() {
            for key in strings_file.keys.keys() {
                seen_keys.insert(key.clone());
            }
        }

        for key in seen_keys {
            if key.contains(query) {
                results.push(build_overview(file_name, &key, localization_file));
            }
        }
    }

    results
}

/// Find keys whose value (in any language) contains the given substring.
pub fn search_by_text(project: &Project, query: &str) -> Vec<KeyOverview> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for (file_name, localization_file) in &project.files {
        let mut matched_keys: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();

        for strings_file in localization_file.locales.values() {
            for (key, entry) in &strings_file.keys {
                if let Some(value) = &entry.value {
                    if value.contains(query) {
                        matched_keys.insert(key.clone());
                    }
                }
            }
        }

        for key in matched_keys {
            results.push(build_overview(file_name, &key, localization_file));
        }
    }

    results
}

fn build_overview(
    file_name: &str,
    key: &str,
    localization_file: &crate::model::LocalizationFile,
) -> KeyOverview {
    let mut translations = HashMap::new();
    for (lang, strings_file) in &localization_file.locales {
        // Include all locales: None if the key is missing for that locale
        let value = strings_file
            .keys
            .get(key)
            .map(|k| k.value.clone())
            .unwrap_or(None);
        translations.insert(lang.clone(), value);
    }
    KeyOverview {
        file: file_name.to_string(),
        key: key.to_string(),
        translations,
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    #[test]
    fn search_by_exact_key() {
        let project = test_helpers::two_locale_project();
        let results = search_by_key(&project, "login.button");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");
        assert_eq!(results[0].file, "Localizable");
    }

    #[test]
    fn search_by_key_substring() {
        let project = test_helpers::two_locale_project();
        let results = search_by_key(&project, "login");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");
    }

    #[test]
    fn search_by_key_no_match() {
        let project = test_helpers::two_locale_project();
        let results = search_by_key(&project, "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn search_by_text_matches_value() {
        let project = test_helpers::two_locale_project();
        let results = search_by_text(&project, "Connexion");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");
    }

    #[test]
    fn search_by_text_matches_english_value() {
        let project = test_helpers::two_locale_project();
        let results = search_by_text(&project, "Sign");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");
    }

    #[test]
    fn search_by_text_case_sensitive() {
        let project = test_helpers::two_locale_project();
        let results = search_by_text(&project, "connexion");
        assert!(results.is_empty());
    }

    #[test]
    fn search_by_text_empty_query_returns_nothing() {
        let project = test_helpers::two_locale_project();
        assert!(search_by_text(&project, "").is_empty());
    }

    #[test]
    fn overview_includes_all_locales() {
        let project = test_helpers::two_locale_project();
        let results = search_by_key(&project, "logout.button");
        assert_eq!(results.len(), 1);
        // logout.button exists in en but not fr
        assert_eq!(
            results[0].translations.get("en").unwrap().as_deref(),
            Some("Log Out")
        );
        // fr should be present with None (missing)
        assert_eq!(results[0].translations.get("fr").unwrap(), &None);
    }
}
