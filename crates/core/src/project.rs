//! Project scanner: detects `.strings` files in `*.lproj` directories
//! and `.xcstrings` (String Catalog) files anywhere in the tree.

use crate::model::{FileFormat, LocalizationFile, Project, StringsFile};
use crate::parser;
use crate::xcstrings;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Scan a root directory or a single `.xcstrings` file and build the
/// in-memory [`Project`] model.
///
/// Detects two formats:
/// - `.strings` files inside `*.lproj` directories (classic format)
/// - `.xcstrings` files anywhere in the tree, or as a direct file argument
///
/// Files are grouped by logical name (filename without extension).
pub fn scan_project(root: &Path) -> Result<Project, ScanError> {
    if !root.exists() {
        return Err(ScanError::NotFound(root.display().to_string()));
    }

    let mut files: HashMap<String, LocalizationFile> = HashMap::new();

    if root.is_file() {
        // Direct file argument — only .xcstrings is supported this way
        load_xcstrings_file(root, &mut files)?;
    } else {
        scan_dir(root, root, &mut files)?;
    }

    Ok(Project {
        root: if root.is_file() {
            root.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            root.to_path_buf()
        },
        files,
    })
}

fn scan_dir(
    root: &Path,
    dir: &Path,
    files: &mut HashMap<String, LocalizationFile>,
) -> Result<(), ScanError> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(ScanError::Io)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();

        // Skip hidden files/directories
        if name.to_string_lossy().starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // lproj directory: parse .strings files inside
            if let Some(locale) = extract_locale(&name.to_string_lossy()) {
                load_lproj_dir(root, &path, &locale, files)?;
            } else {
                // Recurse into non-lproj directories
                scan_dir(root, &path, files)?;
            }
        } else if path.is_file() {
            // .xcstrings file at any level
            let filename = name.to_string_lossy();
            if filename.ends_with(".xcstrings") {
                load_xcstrings_file(&path, files)?;
            }
        }
    }
    Ok(())
}

/// Extract the locale code from an `xx.lproj` directory name.
fn extract_locale(dirname: &str) -> Option<String> {
    dirname.strip_suffix(".lproj").map(|s| s.to_string())
}

/// Load all `.strings` files from a single `*.lproj` directory.
fn load_lproj_dir(
    root: &Path,
    lproj_dir: &Path,
    locale: &str,
    files: &mut HashMap<String, LocalizationFile>,
) -> Result<(), ScanError> {
    let mut entries: Vec<_> = fs::read_dir(lproj_dir)
        .map_err(ScanError::Io)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        if !filename.ends_with(".strings") {
            continue;
        }

        let logical_name = filename
            .strip_suffix(".strings")
            .unwrap_or(&filename)
            .to_string();

        let content = fs::read_to_string(&path).map_err(ScanError::Io)?;
        let result = parser::parse(&content, locale, path.clone());

        let strings_file = StringsFile {
            lang: locale.to_string(),
            path,
            keys: result.file.keys,
        };

        let localization_file = match files.get(&logical_name) {
            Some(existing) if existing.format == FileFormat::XcStrings => {
                // Don't mix .strings into an existing .xcstrings entry
                continue;
            }
            _ => files
                .entry(logical_name.clone())
                .or_insert(LocalizationFile {
                    name: logical_name.clone(),
                    locales: HashMap::new(),
                    format: FileFormat::Strings,
                    path: PathBuf::new(),
                    source_language: "en".to_string(),
                }),
        };

        localization_file
            .locales
            .insert(locale.to_string(), strings_file);
    }

    Ok(())
}

/// Load a single `.xcstrings` file.
fn load_xcstrings_file(
    path: &Path,
    files: &mut HashMap<String, LocalizationFile>,
) -> Result<(), ScanError> {
    let filename = match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return Ok(()),
    };

    let logical_name = filename
        .strip_suffix(".xcstrings")
        .unwrap_or(&filename)
        .to_string();

    let content = fs::read_to_string(path).map_err(ScanError::Io)?;
    let result = xcstrings::parse(&content, &logical_name, path.to_path_buf())
        .map_err(|e| ScanError::Parse(format!("{}", e)))?;

    // Don't overwrite an existing .strings entry with the same logical name
    if let Some(existing) = files.get(&logical_name) {
        if existing.format == FileFormat::Strings {
            return Ok(());
        }
    }

    files.insert(logical_name, result.localization_file);

    Ok(())
}

/// Errors that can occur during project scanning.
#[derive(Debug)]
pub enum ScanError {
    /// The root directory does not exist.
    NotFound(String),
    /// IO error while traversing directories.
    Io(std::io::Error),
    /// Parse error in a `.xcstrings` file.
    Parse(String),
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_strings_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("en.lproj")).unwrap();
        fs::write(
            root.join("en.lproj/Localizable.strings"),
            "\"login.button\" = \"Sign In\";\n\"logout.button\" = \"Log Out\";\n",
        )
        .unwrap();

        fs::create_dir_all(root.join("fr.lproj")).unwrap();
        fs::write(
            root.join("fr.lproj/Localizable.strings"),
            "\"login.button\" = \"Connexion\";\n",
        )
        .unwrap();

        fs::create_dir_all(root.join("Base.lproj")).unwrap();
        fs::write(
            root.join("Base.lproj/Errors.strings"),
            "\"error.network\" = \"Network Error\";\n",
        )
        .unwrap();

        fs::create_dir_all(root.join("Resources/fr.lproj")).unwrap();
        fs::write(
            root.join("Resources/fr.lproj/Localizable.strings"),
            "\"login.button\" = \"Connexion FR2\";\n",
        )
        .unwrap();

        dir
    }

    fn setup_xcstrings_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(
            root.join("Localizable.xcstrings"),
            r#"{
  "sourceLanguage": "en",
  "version": "1.1",
  "strings": {
    "login.button": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Sign In" } },
        "fr": { "stringUnit": { "state": "translated", "value": "Connexion" } }
      }
    },
    "logout.button": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Log Out" } }
      }
    }
  }
}"#,
        )
        .unwrap();

        dir
    }

    #[test]
    fn detects_lproj_directories() {
        let dir = setup_strings_project();
        let project = scan_project(dir.path()).unwrap();
        assert!(project.files.contains_key("Localizable"));
        assert!(project.files.contains_key("Errors"));
    }

    #[test]
    fn groups_locales_by_logical_name() {
        let dir = setup_strings_project();
        let project = scan_project(dir.path()).unwrap();
        let loc = project.files.get("Localizable").unwrap();
        assert_eq!(loc.locales.len(), 2);
        assert!(loc.locales.contains_key("en"));
        assert!(loc.locales.contains_key("fr"));
    }

    #[test]
    fn parses_keys_correctly() {
        let dir = setup_strings_project();
        let project = scan_project(dir.path()).unwrap();
        let en = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("en")
            .unwrap();
        assert_eq!(en.keys.len(), 2);
        assert_eq!(
            en.keys.get("login.button").unwrap().value.as_deref(),
            Some("Sign In")
        );
    }

    #[test]
    fn missing_locale_has_fewer_keys() {
        let dir = setup_strings_project();
        let project = scan_project(dir.path()).unwrap();
        let fr = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("fr")
            .unwrap();
        assert_eq!(fr.keys.len(), 1);
    }

    #[test]
    fn base_lproj_detected() {
        let dir = setup_strings_project();
        let project = scan_project(dir.path()).unwrap();
        let errors = project.files.get("Errors").unwrap();
        assert!(errors.locales.contains_key("Base"));
    }

    #[test]
    fn nested_lproj_found() {
        let dir = setup_strings_project();
        let project = scan_project(dir.path()).unwrap();
        let loc = project.files.get("Localizable").unwrap();
        assert!(loc.locales.contains_key("en"));
        assert!(loc.locales.contains_key("fr"));
    }

    #[test]
    fn empty_directory_produces_empty_project() {
        let dir = tempfile::tempdir().unwrap();
        let project = scan_project(dir.path()).unwrap();
        assert!(project.files.is_empty());
    }

    #[test]
    fn nonexistent_root_returns_error() {
        let result = scan_project(Path::new("/nonexistent/path/xyz"));
        assert!(matches!(result, Err(ScanError::NotFound(_))));
    }

    #[test]
    fn ignores_non_strings_files_in_lproj() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("en.lproj")).unwrap();
        fs::write(root.join("en.lproj/readme.txt"), "hello").unwrap();
        fs::write(root.join("en.lproj/Info.plist"), "<plist/>").unwrap();
        let project = scan_project(root).unwrap();
        assert!(project.files.is_empty());
    }

    #[test]
    fn ignores_hidden_directories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".git/en.lproj")).unwrap();
        fs::write(
            root.join(".git/en.lproj/Localizable.strings"),
            "\"k\" = \"v\";",
        )
        .unwrap();
        let project = scan_project(root).unwrap();
        assert!(project.files.is_empty());
    }

    // --- .xcstrings tests ---

    #[test]
    fn detects_xcstrings_file() {
        let dir = setup_xcstrings_project();
        let project = scan_project(dir.path()).unwrap();
        assert!(project.files.contains_key("Localizable"));
        let loc = project.files.get("Localizable").unwrap();
        assert_eq!(loc.format, FileFormat::XcStrings);
    }

    #[test]
    fn xcstrings_loads_all_locales() {
        let dir = setup_xcstrings_project();
        let project = scan_project(dir.path()).unwrap();
        let loc = project.files.get("Localizable").unwrap();
        assert_eq!(loc.locales.len(), 2);
        assert!(loc.locales.contains_key("en"));
        assert!(loc.locales.contains_key("fr"));
    }

    #[test]
    fn xcstrings_missing_locale_has_fewer_keys() {
        let dir = setup_xcstrings_project();
        let project = scan_project(dir.path()).unwrap();
        let fr = project
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("fr")
            .unwrap();
        assert_eq!(fr.keys.len(), 1);
        assert!(!fr.keys.contains_key("logout.button"));
    }

    #[test]
    fn xcstrings_and_strings_coexist() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // .strings
        fs::create_dir_all(root.join("en.lproj")).unwrap();
        fs::write(root.join("en.lproj/Errors.strings"), "\"err\" = \"Error\";").unwrap();

        // .xcstrings
        fs::write(
            root.join("Localizable.xcstrings"),
            r#"{"sourceLanguage":"en","version":"1","strings":{"k":{"localizations":{"en":{"stringUnit":{"state":"translated","value":"v"}}}}}}"#,
        )
        .unwrap();

        let project = scan_project(root).unwrap();
        assert!(project.files.contains_key("Localizable"));
        assert!(project.files.contains_key("Errors"));
        assert_eq!(
            project.files.get("Localizable").unwrap().format,
            FileFormat::XcStrings
        );
        assert_eq!(
            project.files.get("Errors").unwrap().format,
            FileFormat::Strings
        );
    }

    // --- Full roundtrip integration test on .strings format ---

    fn setup_clean_strings_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // en.lproj with 2 keys
        fs::create_dir_all(root.join("en.lproj")).unwrap();
        fs::write(
            root.join("en.lproj/Localizable.strings"),
            "\"login.button\" = \"Sign In\";\n\"logout.button\" = \"Log Out\";\n",
        )
        .unwrap();

        // fr.lproj with 1 key (logout missing)
        fs::create_dir_all(root.join("fr.lproj")).unwrap();
        fs::write(
            root.join("fr.lproj/Localizable.strings"),
            "\"login.button\" = \"Connexion\";\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn strings_full_roundtrip() {
        use crate::{edit, missing, save, search};

        let dir = setup_clean_strings_project();
        let root = dir.path();

        // 1. Scan
        let mut proj = scan_project(root).unwrap();
        assert_eq!(proj.files.len(), 1);
        assert!(proj.files.contains_key("Localizable"));

        // 2. Search by key
        let results = search::search_by_key(&proj, "login");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");
        assert_eq!(
            results[0].translations.get("en").unwrap().as_deref(),
            Some("Sign In")
        );
        assert_eq!(
            results[0].translations.get("fr").unwrap().as_deref(),
            Some("Connexion")
        );

        // 3. Search by text
        let results = search::search_by_text(&proj, "Connexion");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");

        // 4. Missing detection for fr (logout.button is missing)
        let missing_fr = missing::find_missing(&proj, "fr");
        assert_eq!(missing_fr.len(), 1);
        assert_eq!(missing_fr[0].key, "logout.button");
        assert_eq!(missing_fr[0].ref_lang, "en");
        assert_eq!(missing_fr[0].ref_value.as_deref(), Some("Log Out"));

        // 5. Set the missing value
        edit::set_value(
            &mut proj,
            "Localizable",
            "logout.button",
            "fr",
            "Déconnexion",
        )
        .unwrap();

        // 6. Save
        save::save_project(&proj, false).unwrap();

        // 7. Verify fr.lproj/Localizable.strings was written
        let fr_path = root.join("fr.lproj/Localizable.strings");
        assert!(fr_path.exists());
        let fr_content = fs::read_to_string(&fr_path).unwrap();
        assert!(fr_content.contains("\"logout.button\" = \"Déconnexion\";"));
        // Original key preserved
        assert!(fr_content.contains("\"login.button\" = \"Connexion\";"));

        // 8. Rescan and verify
        let proj2 = scan_project(root).unwrap();
        let fr = proj2
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("fr")
            .unwrap();
        assert_eq!(fr.keys.len(), 2);
        assert_eq!(
            fr.keys.get("logout.button").unwrap().value.as_deref(),
            Some("Déconnexion")
        );
        assert_eq!(
            fr.keys.get("login.button").unwrap().value.as_deref(),
            Some("Connexion")
        );

        // 9. No more missing for fr
        let missing_after = missing::find_missing(&proj2, "fr");
        assert!(missing_after.is_empty());

        // 10. en still has both keys
        let en = proj2
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("en")
            .unwrap();
        assert_eq!(en.keys.len(), 2);

        // 11. en file untouched on disk
        let en_path = root.join("en.lproj/Localizable.strings");
        let en_content = fs::read_to_string(&en_path).unwrap();
        assert!(en_content.contains("\"login.button\" = \"Sign In\";"));
        assert!(en_content.contains("\"logout.button\" = \"Log Out\";"));
    }

    #[test]
    fn strings_add_key_and_save() {
        use crate::{edit, save};

        let dir = setup_strings_project();
        let root = dir.path();

        let mut proj = scan_project(root).unwrap();
        edit::add_key(&mut proj, "Localizable", "new.key", "en", "New Value").unwrap();
        save::save_project(&proj, false).unwrap();

        // Verify written to en.lproj
        let en_path = root.join("en.lproj/Localizable.strings");
        let content = fs::read_to_string(&en_path).unwrap();
        assert!(content.contains("\"new.key\" = \"New Value\";"));

        // Not in fr.lproj (detectable via missing)
        let fr_path = root.join("fr.lproj/Localizable.strings");
        let fr_content = fs::read_to_string(&fr_path).unwrap();
        assert!(!fr_content.contains("new.key"));
    }

    // --- Full roundtrip integration test on .xcstrings format ---

    fn setup_clean_xcstrings_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(
            root.join("Localizable.xcstrings"),
            r#"{
  "sourceLanguage": "en",
  "version": "1.1",
  "strings": {
    "login.button": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Sign In" } },
        "fr": { "stringUnit": { "state": "translated", "value": "Connexion" } }
      }
    },
    "logout.button": {
      "localizations": {
        "en": { "stringUnit": { "state": "translated", "value": "Log Out" } }
      }
    }
  }
}"#,
        )
        .unwrap();

        dir
    }

    #[test]
    fn xcstrings_full_roundtrip() {
        use crate::xcstrings;
        use crate::{edit, missing, save, search};

        let dir = setup_clean_xcstrings_project();
        let root = dir.path();

        // 1. Scan
        let mut proj = scan_project(root).unwrap();
        assert_eq!(proj.files.len(), 1);
        assert!(proj.files.contains_key("Localizable"));
        assert_eq!(
            proj.files.get("Localizable").unwrap().format,
            FileFormat::XcStrings,
        );

        // 2. Search by key
        let results = search::search_by_key(&proj, "login");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "login.button");
        assert_eq!(
            results[0].translations.get("en").unwrap().as_deref(),
            Some("Sign In"),
        );
        assert_eq!(
            results[0].translations.get("fr").unwrap().as_deref(),
            Some("Connexion"),
        );

        // 3. Missing detection for fr (logout.button is missing)
        let missing_fr = missing::find_missing(&proj, "fr");
        assert_eq!(missing_fr.len(), 1);
        assert_eq!(missing_fr[0].key, "logout.button");
        assert_eq!(missing_fr[0].ref_lang, "en");
        assert_eq!(missing_fr[0].ref_value.as_deref(), Some("Log Out"));

        // 4. Set the missing value — state should become Translated
        edit::set_value(
            &mut proj,
            "Localizable",
            "logout.button",
            "fr",
            "Déconnexion",
        )
        .unwrap();
        let fr = proj
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("fr")
            .unwrap();
        assert_eq!(
            fr.keys.get("logout.button").unwrap().value.as_deref(),
            Some("Déconnexion"),
        );
        assert_eq!(
            fr.keys.get("logout.button").unwrap().state,
            crate::model::KeyState::Translated,
        );

        // 5. Save
        save::save_project(&proj, false).unwrap();

        // 6. Re-parse the saved file and verify values survived
        let saved_content = fs::read_to_string(root.join("Localizable.xcstrings")).unwrap();
        let reparsed = xcstrings::parse(
            &saved_content,
            "Localizable",
            root.join("Localizable.xcstrings"),
        )
        .unwrap();
        let fr_re = reparsed.localization_file.locales.get("fr").unwrap();
        assert_eq!(fr_re.keys.len(), 2);
        assert_eq!(
            fr_re.keys.get("login.button").unwrap().value.as_deref(),
            Some("Connexion"),
        );
        assert_eq!(
            fr_re.keys.get("logout.button").unwrap().value.as_deref(),
            Some("Déconnexion"),
        );

        // 7. Rescan from disk and verify no more missing for fr
        let proj2 = scan_project(root).unwrap();
        let missing_after = missing::find_missing(&proj2, "fr");
        assert!(missing_after.is_empty());

        // 8. en still has both keys untouched
        let en = proj2
            .files
            .get("Localizable")
            .unwrap()
            .locales
            .get("en")
            .unwrap();
        assert_eq!(en.keys.len(), 2);
        assert_eq!(
            en.keys.get("login.button").unwrap().value.as_deref(),
            Some("Sign In"),
        );
        assert_eq!(
            en.keys.get("logout.button").unwrap().value.as_deref(),
            Some("Log Out"),
        );
    }
}
