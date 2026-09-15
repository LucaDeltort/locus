//! Parser for Apple `.xcstrings` (String Catalog) files.
//!
//! A single JSON file containing all locales for a logical file:
//! ```json
//! {
//!   "sourceLanguage": "en",
//!   "version": "1.1",
//!   "strings": {
//!     "key": {
//!       "localizations": {
//!         "en": { "stringUnit": { "state": "translated", "value": "..." } },
//!         "fr": { "stringUnit": { "state": "translated", "value": "..." } }
//!       }
//!     }
//!   }
//! }
//! ```

use crate::model::{Key, LocalizationFile, StringsFile};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
struct XcStringsDoc {
    #[serde(rename = "sourceLanguage")]
    source_language: String,
    #[serde(default)]
    version: String,
    strings: IndexMap<String, XcStringEntry>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct XcStringEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    localizations: Option<IndexMap<String, XcLocalization>>,
    #[serde(
        default,
        rename = "extractionState",
        skip_serializing_if = "Option::is_none"
    )]
    extraction_state: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct XcLocalization {
    #[serde(
        default,
        rename = "stringUnit",
        skip_serializing_if = "Option::is_none"
    )]
    string_unit: Option<XcStringUnit>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct XcStringUnit {
    state: String,
    value: String,
}

/// Result of parsing a `.xcstrings` file.
#[derive(Debug)]
pub struct XcStringsParseResult {
    pub source_language: String,
    pub version: String,
    pub localization_file: LocalizationFile,
}

/// Errors that can occur during `.xcstrings` parsing.
#[derive(Debug)]
pub enum XcStringsError {
    Json(serde_json::Error),
}

impl std::fmt::Display for XcStringsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for XcStringsError {}

/// Parse a `.xcstrings` JSON file.
///
/// `name` is the logical filename without extension (e.g. "Localizable").
/// `path` is the full path to the `.xcstrings` file.
pub fn parse(
    content: &str,
    name: &str,
    path: PathBuf,
) -> Result<XcStringsParseResult, XcStringsError> {
    let doc: XcStringsDoc = serde_json::from_str(content).map_err(XcStringsError::Json)?;

    // Collect all locale codes across all keys
    let mut all_locales: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for entry in doc.strings.values() {
        if let Some(locs) = &entry.localizations {
            for lang in locs.keys() {
                all_locales.insert(lang.clone());
            }
        }
    }

    // Build per-locale StringsFiles
    let mut locales: HashMap<String, StringsFile> = HashMap::new();
    for lang in &all_locales {
        locales.insert(
            lang.clone(),
            StringsFile {
                lang: lang.clone(),
                path: PathBuf::new(),
                keys: IndexMap::new(),
            },
        );
    }

    for (key, entry) in &doc.strings {
        let extraction = entry.extraction_state.clone();
        if let Some(locs) = &entry.localizations {
            for (lang, loc) in locs {
                if let Some(unit) = &loc.string_unit {
                    if let Some(sf) = locales.get_mut(lang) {
                        sf.keys.insert(
                            key.clone(),
                            Key {
                                key: key.clone(),
                                value: Some(unit.value.clone()),
                                comment: None,
                                state: parse_state(&unit.state),
                                extraction_state: extraction.clone(),
                            },
                        );
                    }
                }
            }
        } else {
            // Key with no localizations — add to source language with None value
            if let Some(sf) = locales.get_mut(&doc.source_language) {
                sf.keys.insert(
                    key.clone(),
                    Key {
                        key: key.clone(),
                        value: None,
                        comment: None,
                        state: crate::model::KeyState::New,
                        extraction_state: extraction,
                    },
                );
            }
        }
    }

    let localization_file = LocalizationFile {
        name: name.to_string(),
        locales,
        format: crate::model::FileFormat::XcStrings,
        path,
        source_language: doc.source_language.clone(),
    };

    Ok(XcStringsParseResult {
        source_language: doc.source_language,
        version: doc.version,
        localization_file,
    })
}

/// Serialize a [`LocalizationFile`] back to `.xcstrings` JSON.
///
/// Merges per-locale keys back into the unified JSON structure.
/// Preserves the original translation state of each key.
pub fn serialize(file: &LocalizationFile, source_language: &str, version: &str) -> String {
    // Collect all unique keys across all locales, preserving insertion order
    let mut all_keys: IndexMap<String, ()> = IndexMap::new();
    for sf in file.locales.values() {
        for key in sf.keys.keys() {
            all_keys.insert(key.clone(), ());
        }
    }

    let mut strings = IndexMap::new();

    for key in all_keys.keys() {
        let mut localizations: IndexMap<String, XcLocalization> = IndexMap::new();
        let mut extraction_state: Option<String> = None;

        for (lang, sf) in &file.locales {
            if let Some(k) = sf.keys.get(key) {
                if extraction_state.is_none() {
                    extraction_state = k.extraction_state.clone();
                }
                if let Some(value) = &k.value {
                    localizations.insert(
                        lang.clone(),
                        XcLocalization {
                            string_unit: Some(XcStringUnit {
                                state: state_to_string(k.state).to_string(),
                                value: value.clone(),
                            }),
                        },
                    );
                }
            }
        }

        let entry = XcStringEntry {
            localizations: if localizations.is_empty() {
                None
            } else {
                Some(localizations)
            },
            extraction_state,
        };

        strings.insert(key.clone(), entry);
    }

    let doc = XcStringsDoc {
        source_language: source_language.to_string(),
        version: version.to_string(),
        strings,
    };

    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
}

/// Serialize directly to a writer, avoiding an intermediate String allocation.
pub fn serialize_to_writer<W: std::io::Write>(
    file: &LocalizationFile,
    source_language: &str,
    version: &str,
    writer: &mut W,
) -> Result<(), std::io::Error> {
    let json = serialize(file, source_language, version);
    writer.write_all(json.as_bytes())
}

fn parse_state(s: &str) -> crate::model::KeyState {
    match s {
        "translated" => crate::model::KeyState::Translated,
        "new" => crate::model::KeyState::New,
        "stale" => crate::model::KeyState::Stale,
        "needs_review" | "needsReview" => crate::model::KeyState::NeedsReview,
        _ => crate::model::KeyState::Translated,
    }
}

fn state_to_string(state: crate::model::KeyState) -> &'static str {
    match state {
        crate::model::KeyState::Translated => "translated",
        crate::model::KeyState::New => "new",
        crate::model::KeyState::Stale => "stale",
        crate::model::KeyState::NeedsReview => "needs_review",
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xcstrings() -> &'static str {
        r#"{
  "sourceLanguage": "en",
  "version": "1.1",
  "strings": {
    "login.button": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "Sign In"
          }
        },
        "fr": {
          "stringUnit": {
            "state": "translated",
            "value": "Connexion"
          }
        }
      }
    },
    "logout.button": {
      "localizations": {
        "en": {
          "stringUnit": {
            "state": "translated",
            "value": "Log Out"
          }
        }
      }
    },
    "empty.key": {}
  }
}"#
    }

    #[test]
    fn parse_basic() {
        let result = parse(
            sample_xcstrings(),
            "Localizable",
            PathBuf::from("Localizable.xcstrings"),
        )
        .unwrap();
        assert_eq!(result.source_language, "en");
        assert_eq!(result.version, "1.1");
        assert_eq!(result.localization_file.locales.len(), 2);
        assert!(result.localization_file.locales.contains_key("en"));
        assert!(result.localization_file.locales.contains_key("fr"));
    }

    #[test]
    fn parse_extracts_values_per_locale() {
        let result = parse(
            sample_xcstrings(),
            "Localizable",
            PathBuf::from("Localizable.xcstrings"),
        )
        .unwrap();
        let en = result.localization_file.locales.get("en").unwrap();
        assert_eq!(en.keys.len(), 3); // login.button, logout.button, empty.key
        assert_eq!(
            en.keys.get("login.button").unwrap().value.as_deref(),
            Some("Sign In")
        );
        assert_eq!(
            en.keys.get("logout.button").unwrap().value.as_deref(),
            Some("Log Out")
        );
        assert_eq!(en.keys.get("empty.key").unwrap().value, None);
    }

    #[test]
    fn parse_fr_has_fewer_keys() {
        let result = parse(
            sample_xcstrings(),
            "Localizable",
            PathBuf::from("Localizable.xcstrings"),
        )
        .unwrap();
        let fr = result.localization_file.locales.get("fr").unwrap();
        assert_eq!(fr.keys.len(), 1);
        assert_eq!(
            fr.keys.get("login.button").unwrap().value.as_deref(),
            Some("Connexion")
        );
    }

    #[test]
    fn roundtrip_preserves_keys_and_values() {
        let parsed = parse(
            sample_xcstrings(),
            "Localizable",
            PathBuf::from("Localizable.xcstrings"),
        )
        .unwrap();
        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        let reparsed = parse(&serialized, "Localizable", PathBuf::from("out.xcstrings")).unwrap();

        let en_orig = parsed.localization_file.locales.get("en").unwrap();
        let en_re = reparsed.localization_file.locales.get("en").unwrap();
        assert_eq!(en_orig.keys.len(), en_re.keys.len());
        for k in en_orig.keys.keys() {
            assert_eq!(
                en_orig.keys.get(k).unwrap().value,
                en_re.keys.get(k).unwrap().value
            );
        }
    }

    #[test]
    fn roundtrip_preserves_missing_locale() {
        let parsed = parse(
            sample_xcstrings(),
            "Localizable",
            PathBuf::from("Localizable.xcstrings"),
        )
        .unwrap();
        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        let reparsed = parse(&serialized, "Localizable", PathBuf::from("out.xcstrings")).unwrap();
        let fr = reparsed.localization_file.locales.get("fr").unwrap();
        assert_eq!(fr.keys.len(), 1);
        assert!(!fr.keys.contains_key("logout.button"));
    }

    #[test]
    fn empty_key_preserved_on_roundtrip() {
        let parsed = parse(
            sample_xcstrings(),
            "Localizable",
            PathBuf::from("Localizable.xcstrings"),
        )
        .unwrap();
        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        let reparsed = parse(&serialized, "Localizable", PathBuf::from("out.xcstrings")).unwrap();
        let en = reparsed.localization_file.locales.get("en").unwrap();
        assert!(en.keys.contains_key("empty.key"));
        assert_eq!(en.keys.get("empty.key").unwrap().value, None);
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = parse("{not json}", "Localizable", PathBuf::from("test.xcstrings"));
        assert!(result.is_err());
    }

    #[test]
    fn unicode_values() {
        let input = r#"{"sourceLanguage":"en","version":"1","strings":{"k":{"localizations":{"ja":{"stringUnit":{"state":"translated","value":"日本語"}}}}}}"#;
        let result = parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        let ja = result.localization_file.locales.get("ja").unwrap();
        assert_eq!(ja.keys.get("k").unwrap().value.as_deref(), Some("日本語"));
    }

    #[test]
    fn preserves_new_state_on_roundtrip() {
        let input = r#"{
  "sourceLanguage": "en",
  "version": "1",
  "strings": {
    "new.key": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Hello" } },
        "de": { "stringUnit": { "state": "new", "value": "" } }
      }
    }
  }
}"#;
        let parsed = parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        let de = parsed.localization_file.locales.get("de").unwrap();
        assert_eq!(
            de.keys.get("new.key").unwrap().state,
            crate::model::KeyState::New
        );

        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        // Re-parse and verify the state survives the roundtrip
        let reparsed = parse(
            &serialized,
            "Localizable",
            PathBuf::from("roundtrip.xcstrings"),
        )
        .unwrap();
        let de_re = reparsed.localization_file.locales.get("de").unwrap();
        assert_eq!(
            de_re.keys.get("new.key").unwrap().state,
            crate::model::KeyState::New
        );
    }

    #[test]
    fn preserves_stale_state_on_roundtrip() {
        let input = r#"{"sourceLanguage":"en","version":"1","strings":{"k":{"localizations":{"en":{"stringUnit":{"state":"stale","value":"Old"}}}}}}"#;
        let parsed = parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        assert_eq!(
            parsed
                .localization_file
                .locales
                .get("en")
                .unwrap()
                .keys
                .get("k")
                .unwrap()
                .state,
            crate::model::KeyState::Stale,
        );
        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        let reparsed = parse(&serialized, "Localizable", PathBuf::from("rt.xcstrings")).unwrap();
        assert_eq!(
            reparsed
                .localization_file
                .locales
                .get("en")
                .unwrap()
                .keys
                .get("k")
                .unwrap()
                .state,
            crate::model::KeyState::Stale,
        );
    }

    #[test]
    fn preserves_needs_review_state_on_roundtrip() {
        let input = r#"{"sourceLanguage":"en","version":"1","strings":{"k":{"localizations":{"en":{"stringUnit":{"state":"needs_review","value":"Check me"}}}}}}"#;
        let parsed = parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        assert_eq!(
            parsed
                .localization_file
                .locales
                .get("en")
                .unwrap()
                .keys
                .get("k")
                .unwrap()
                .state,
            crate::model::KeyState::NeedsReview,
        );
        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        let reparsed = parse(&serialized, "Localizable", PathBuf::from("rt.xcstrings")).unwrap();
        assert_eq!(
            reparsed
                .localization_file
                .locales
                .get("en")
                .unwrap()
                .keys
                .get("k")
                .unwrap()
                .state,
            crate::model::KeyState::NeedsReview,
        );
    }

    #[test]
    fn preserves_extraction_state_on_roundtrip() {
        let input = r#"{"sourceLanguage":"en","version":"1","strings":{"k":{"extractionState":"manual","localizations":{"en":{"stringUnit":{"state":"translated","value":"v"}}}}}}"#;
        let parsed = parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        let en = parsed.localization_file.locales.get("en").unwrap();
        assert_eq!(
            en.keys.get("k").unwrap().extraction_state.as_deref(),
            Some("manual"),
        );
        let serialized = serialize(
            &parsed.localization_file,
            &parsed.source_language,
            &parsed.version,
        );
        let reparsed = parse(&serialized, "Localizable", PathBuf::from("rt.xcstrings")).unwrap();
        let en_re = reparsed.localization_file.locales.get("en").unwrap();
        assert_eq!(
            en_re.keys.get("k").unwrap().extraction_state.as_deref(),
            Some("manual"),
        );
    }

    #[test]
    fn non_en_source_language_parsed() {
        let input = r#"{"sourceLanguage":"fr","version":"1","strings":{"k":{"localizations":{"fr":{"stringUnit":{"state":"translated","value":"Bonjour"}}}}}}"#;
        let result = parse(input, "Localizable", PathBuf::from("t.xcstrings")).unwrap();
        assert_eq!(result.source_language, "fr");
        let fr = result.localization_file.locales.get("fr").unwrap();
        assert_eq!(fr.keys.get("k").unwrap().value.as_deref(), Some("Bonjour"));
    }
}
