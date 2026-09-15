//! Missing translation detection.
//!
//! For a given target locale, finds keys present in other locales but
//! absent for that locale. The reference locale (Base or `en`) defines
//! the complete set of expected keys.

use crate::model::Project;
use std::collections::BTreeSet;

/// A single missing key: which file, which key, and reference value.
#[derive(Debug, Clone)]
pub struct MissingKey {
    pub file: String,
    pub key: String,
    /// Locales where this key exists.
    pub present_in: Vec<String>,
    /// Reference locale used for the source value (first present locale).
    pub ref_lang: String,
    /// Source value from the reference locale.
    pub ref_value: Option<String>,
}

/// List all keys missing for the given locale.
///
/// A key is missing if it exists in at least one other locale but not
/// in the target locale. Keys absent everywhere are not reported.
pub fn find_missing(project: &Project, lang: &str) -> Vec<MissingKey> {
    let mut result = Vec::new();

    for (file_name, localization_file) in &project.files {
        // Collect all known keys across all locales
        let mut all_keys: BTreeSet<String> = BTreeSet::new();
        for strings_file in localization_file.locales.values() {
            for key in strings_file.keys.keys() {
                all_keys.insert(key.clone());
            }
        }

        for key in all_keys {
            // Check if the key is present in the target locale
            let target_has_key = localization_file
                .locales
                .get(lang)
                .map(|sf| sf.keys.contains_key(&key))
                .unwrap_or(false);

            if !target_has_key {
                // Find which locales have this key
                let present_in: Vec<String> = localization_file
                    .locales
                    .iter()
                    .filter(|(_, sf)| sf.keys.contains_key(&key))
                    .map(|(l, _)| l.clone())
                    .collect();

                if !present_in.is_empty() {
                    // Prefer the file's source language as reference if it has the key
                    let ref_lang = if present_in.contains(&localization_file.source_language) {
                        localization_file.source_language.clone()
                    } else {
                        present_in[0].clone()
                    };
                    let ref_value = localization_file
                        .locales
                        .get(&ref_lang)
                        .and_then(|sf| sf.keys.get(&key))
                        .and_then(|k| k.value.clone());

                    result.push(MissingKey {
                        file: file_name.clone(),
                        key,
                        present_in,
                        ref_lang,
                        ref_value,
                    });
                }
            }
        }
    }

    result
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    #[test]
    fn finds_missing_keys_for_fr() {
        let project = test_helpers::two_locale_project();
        let missing = find_missing(&project, "fr");
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].key, "logout.button");
        assert_eq!(missing[0].file, "Localizable");
        assert!(missing[0].present_in.contains(&"en".to_string()));
    }

    #[test]
    fn no_missing_for_locale_with_all_keys() {
        let project = test_helpers::two_locale_project();
        let missing = find_missing(&project, "en");
        assert!(missing.is_empty());
    }

    #[test]
    fn missing_for_nonexistent_locale_reports_all() {
        let project = test_helpers::two_locale_project();
        let missing = find_missing(&project, "de");
        assert_eq!(missing.len(), 2); // both keys are missing for de
    }

    #[test]
    fn empty_project_no_missing() {
        let project = test_helpers::empty_project();
        assert!(find_missing(&project, "fr").is_empty());
    }
}
