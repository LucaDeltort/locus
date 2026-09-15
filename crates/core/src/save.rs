//! Disk persistence with atomic writes and optional backups.

use crate::model::{Project, StringsFile};
use std::path::Path;

/// Serialize a [`StringsFile`] back to `.strings` text format.
///
/// Preserves insertion order, comments, and uses the standard Apple
/// formatting: one key per line with `/* comment */` above if present.
pub fn serialize_file(strings_file: &StringsFile) -> String {
    let mut out = String::new();

    for key in strings_file.keys.values() {
        if let Some(comment) = &key.comment {
            out.push_str("/* ");
            out.push_str(comment);
            out.push_str(" */\n");
        }
        out.push('"');
        out.push_str(&escape(&key.key));
        out.push_str("\" = \"");
        out.push_str(&escape(key.value.as_deref().unwrap_or("")));
        out.push_str("\";\n");
    }

    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

/// Save all modified files in the project (atomic write per file).
///
/// For `.strings` format: writes each locale to its own file.
/// For `.xcstrings` format: serializes all locales back into the single
/// JSON file.
pub fn save_project(project: &Project, backup: bool) -> Result<(), SaveError> {
    for localization_file in project.files.values() {
        match localization_file.format {
            crate::model::FileFormat::Strings => {
                for strings_file in localization_file.locales.values() {
                    let path = if strings_file.path.as_os_str().is_empty() {
                        project
                            .root
                            .join(format!("{}.lproj", strings_file.lang))
                            .join(format!("{}.strings", localization_file.name))
                    } else {
                        strings_file.path.clone()
                    };
                    let content = serialize_file(strings_file);
                    save_file(&path, &content, "strings", backup)?;
                }
            }
            crate::model::FileFormat::XcStrings => {
                if localization_file.path.as_os_str().is_empty() {
                    continue;
                }
                // Write directly to the temp file, avoiding a 5MB String allocation
                let parent = localization_file
                    .path
                    .parent()
                    .unwrap_or_else(|| Path::new("."));
                std::fs::create_dir_all(parent).map_err(SaveError::Io)?;

                if backup && localization_file.path.exists() {
                    let bak_path = localization_file.path.with_extension("xcstrings.bak");
                    std::fs::copy(&localization_file.path, &bak_path).map_err(SaveError::Io)?;
                }

                let tmp_path = localization_file.path.with_extension("xcstrings.tmp");
                let mut tmp_file = std::fs::File::create(&tmp_path).map_err(SaveError::Io)?;
                crate::xcstrings::serialize_to_writer(
                    localization_file,
                    &localization_file.source_language,
                    "1.1",
                    &mut tmp_file,
                )
                .map_err(|e| SaveError::Io(std::io::Error::other(e)))?;
                drop(tmp_file);
                std::fs::rename(&tmp_path, &localization_file.path).map_err(SaveError::Io)?;
            }
        }
    }
    Ok(())
}

/// Write a single file to disk atomically.
///
/// Writes to a temporary file first, then renames it to the target path.
/// If `backup` is true, the original file is copied to `.bak` before writing.
/// `ext` is the file extension ("strings" or "xcstrings") used for temp/backup naming.
pub fn save_file(path: &Path, content: &str, ext: &str, backup: bool) -> Result<(), SaveError> {
    if backup && path.exists() {
        let bak_path = path.with_extension(format!("{}.bak", ext));
        std::fs::copy(path, &bak_path).map_err(SaveError::Io)?;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(SaveError::Io)?;

    let tmp_path = path.with_extension(format!("{}.tmp", ext));
    std::fs::write(&tmp_path, content).map_err(SaveError::Io)?;
    std::fs::rename(&tmp_path, path).map_err(SaveError::Io)?;

    Ok(())
}

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Key;
    use crate::parser;
    use indexmap::IndexMap;
    use std::path::PathBuf;

    #[test]
    fn roundtrip_preserves_keys_and_values() {
        let input = r#""login.button" = "Sign In";
"logout.button" = "Log Out";"#;
        let parsed = parser::parse(input, "en", PathBuf::from("test.strings"));
        let serialized = serialize_file(&parsed.file);
        let reparsed = parser::parse(&serialized, "en", PathBuf::from("test2.strings"));
        assert_eq!(parsed.file.keys.len(), reparsed.file.keys.len());
        for (key_name, value) in &parsed.file.keys {
            assert_eq!(reparsed.file.keys.get(key_name).unwrap().value, value.value);
        }
    }

    #[test]
    fn roundtrip_preserves_comments() {
        let input = r#"/* Login button */
"login.button" = "Sign In";"#;
        let parsed = parser::parse(input, "en", PathBuf::from("test.strings"));
        let serialized = serialize_file(&parsed.file);
        let reparsed = parser::parse(&serialized, "en", PathBuf::from("test2.strings"));
        assert_eq!(
            reparsed.file.keys.get("login.button").unwrap().comment,
            Some("Login button".to_string())
        );
    }

    #[test]
    fn roundtrip_preserves_order() {
        let input = r#""zebra" = "z";
"alpha" = "a";
"mike" = "m";"#;
        let parsed = parser::parse(input, "en", PathBuf::from("test.strings"));
        let serialized = serialize_file(&parsed.file);
        let reparsed = parser::parse(&serialized, "en", PathBuf::from("test2.strings"));
        let keys: Vec<&str> = reparsed.file.keys.keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["zebra", "alpha", "mike"]);
    }

    #[test]
    fn escape_special_chars_roundtrip() {
        // Serialize a key with quotes, backslashes, and newlines,
        // then re-parse and verify the values survive intact.
        let mut keys = IndexMap::new();
        keys.insert(
            "k".to_string(),
            Key {
                key: "k".into(),
                value: Some("a\"b\\c\nd".into()),
                comment: None,
                state: crate::model::KeyState::Translated,
                extraction_state: None,
            },
        );
        let sf = StringsFile {
            lang: "en".into(),
            path: PathBuf::from("x"),
            keys,
        };
        let serialized = serialize_file(&sf);
        let reparsed = parser::parse(&serialized, "en", PathBuf::from("roundtrip.strings"));
        assert_eq!(
            reparsed.file.keys.get("k").unwrap().value.as_deref(),
            Some("a\"b\\c\nd")
        );
    }

    #[test]
    fn save_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.strings");

        let mut keys = IndexMap::new();
        keys.insert(
            "hello".to_string(),
            Key {
                key: "hello".into(),
                value: Some("World".into()),
                comment: None,
                state: crate::model::KeyState::Translated,
                extraction_state: None,
            },
        );
        let sf = StringsFile {
            lang: "en".into(),
            path: path.clone(),
            keys,
        };

        save_file(&path, &serialize_file(&sf), "strings", false).unwrap();
        // Read back via the parser rather than substring-matching the raw text
        let content = std::fs::read_to_string(&path).unwrap();
        let reparsed = parser::parse(&content, "en", path);
        assert_eq!(
            reparsed.file.keys.get("hello").unwrap().value.as_deref(),
            Some("World")
        );
    }

    #[test]
    fn backup_creates_bak_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.strings");
        std::fs::write(&path, "original").unwrap();

        save_file(&path, "new content", "strings", true).unwrap();
        assert!(path.with_extension("strings.bak").exists());
        assert_eq!(
            std::fs::read_to_string(path.with_extension("strings.bak")).unwrap(),
            "original"
        );
    }

    #[test]
    fn save_project_creates_missing_locale_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create en.lproj/Localizable.strings
        std::fs::create_dir_all(root.join("en.lproj")).unwrap();
        std::fs::write(
            root.join("en.lproj/Localizable.strings"),
            "\"key\" = \"value\";",
        )
        .unwrap();

        // Load project, add a fr locale via set_value (in-memory, no path)
        let mut proj = crate::project::scan_project(root).unwrap();
        crate::edit::set_value(&mut proj, "Localizable", "key", "fr", "valeur").unwrap();

        // Save — should create fr.lproj/Localizable.strings
        save_project(&proj, false).unwrap();

        let fr_path = root.join("fr.lproj/Localizable.strings");
        assert!(fr_path.exists(), "fr.lproj file should be created on save");
        let content = std::fs::read_to_string(&fr_path).unwrap();
        assert!(content.contains("\"key\" = \"valeur\";"));
    }
}
