//! Parser for Apple `.strings` files.
//!
//! Format:
//! ```text
//! /* Optional comment */
//! "key" = "value";
//! ```
//!
//! Handles `/* ... */` (multi-line) and `// ...` line comments, double-quoted
//! strings with escape sequences (`\n`, `\t`, `\r`, `\"`, `\\`), unquoted
//! tokens, and duplicate key detection (last value wins).

use crate::model::{Key, StringsFile};
use indexmap::IndexMap;
use std::path::PathBuf;

/// Result of parsing a single `.strings` file.
#[derive(Debug)]
pub struct ParseResult {
    pub file: StringsFile,
    /// Keys that appeared more than once (last value wins).
    pub duplicates: Vec<String>,
    /// Non-fatal issues encountered during parsing.
    pub warnings: Vec<String>,
}

/// Parse the raw text content of a `.strings` file.
///
/// `lang` is the locale code of the parent `lproj` folder.
pub fn parse(content: &str, lang: &str, path: PathBuf) -> ParseResult {
    let mut keys: IndexMap<String, Key> = IndexMap::new();
    let mut duplicates = Vec::new();
    let mut warnings = Vec::new();

    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    let mut pending_comment: Option<String> = None;

    while pos < chars.len() {
        skip_whitespace(&chars, &mut pos);
        if pos >= chars.len() {
            break;
        }

        // Block comment /* ... */ (may span multiple lines)
        if peek_two(&chars, pos) == Some(('/', '*')) {
            pos += 2;
            let start = pos;
            while pos + 1 < chars.len() && !(chars[pos] == '*' && chars[pos + 1] == '/') {
                pos += 1;
            }
            let raw: String = chars[start..pos.min(chars.len())].iter().collect();
            if pos + 1 < chars.len() {
                pos += 2; // skip closing */
            } else {
                warnings.push("Unterminated block comment".to_string());
            }
            pending_comment = Some(raw.trim().to_string());
            continue;
        }

        // Line comment // ...
        if peek_two(&chars, pos) == Some(('/', '/')) {
            pos += 2;
            while pos < chars.len() && chars[pos] != '\n' {
                pos += 1;
            }
            continue;
        }

        // Key-value pair
        let key = match read_token(&chars, &mut pos, &mut warnings) {
            Some(k) => k,
            None => {
                if pos < chars.len() {
                    warnings.push(format!("Unexpected '{}' at offset {}", chars[pos], pos));
                    pos += 1;
                }
                continue;
            }
        };

        skip_whitespace(&chars, &mut pos);

        // Expect '='
        if pos >= chars.len() || chars[pos] != '=' {
            warnings.push(format!("Expected '=' after key \"{}\"", key));
            continue;
        }
        pos += 1;

        skip_whitespace(&chars, &mut pos);

        // Read value
        let value = match read_token(&chars, &mut pos, &mut warnings) {
            Some(v) => v,
            None => {
                warnings.push(format!("Missing value for key \"{}\"", key));
                continue;
            }
        };

        skip_whitespace(&chars, &mut pos);

        // Expect ';'
        if pos < chars.len() && chars[pos] == ';' {
            pos += 1;
        } else {
            warnings.push(format!("Missing ';' after value for key \"{}\"", key));
            continue;
        }

        if keys.contains_key(&key) {
            duplicates.push(key.clone());
        }

        keys.insert(
            key.clone(),
            Key {
                key,
                value: Some(value),
                comment: pending_comment.take(),
                state: crate::model::KeyState::Translated,
                extraction_state: None,
            },
        );
    }

    let file = StringsFile {
        lang: lang.to_string(),
        path,
        keys,
    };

    ParseResult {
        file,
        duplicates,
        warnings,
    }
}

// --- Helpers ---

fn skip_whitespace(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn peek_two(chars: &[char], pos: usize) -> Option<(char, char)> {
    if pos + 1 < chars.len() {
        Some((chars[pos], chars[pos + 1]))
    } else {
        None
    }
}

/// Read a quoted string (with escape handling) or an unquoted token.
fn read_token(chars: &[char], pos: &mut usize, warnings: &mut Vec<String>) -> Option<String> {
    if *pos >= chars.len() {
        return None;
    }

    if chars[*pos] == '"' {
        read_quoted(chars, pos, warnings)
    } else {
        read_unquoted(chars, pos)
    }
}

fn read_quoted(chars: &[char], pos: &mut usize, warnings: &mut Vec<String>) -> Option<String> {
    *pos += 1; // opening "
    let mut result = String::new();

    while *pos < chars.len() && chars[*pos] != '"' {
        if chars[*pos] == '\\' {
            *pos += 1;
            if *pos >= chars.len() {
                warnings.push("Unterminated escape sequence in string".to_string());
                return None;
            }
            match chars[*pos] {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                c => {
                    // Unknown escape — preserve literally
                    result.push('\\');
                    result.push(c);
                }
            }
            *pos += 1;
        } else {
            result.push(chars[*pos]);
            *pos += 1;
        }
    }

    if *pos < chars.len() && chars[*pos] == '"' {
        *pos += 1; // closing "
        Some(result)
    } else {
        warnings.push("Unterminated quoted string".to_string());
        None
    }
}

fn read_unquoted(chars: &[char], pos: &mut usize) -> Option<String> {
    let mut result = String::new();
    while *pos < chars.len()
        && !chars[*pos].is_whitespace()
        && chars[*pos] != '='
        && chars[*pos] != ';'
    {
        result.push(chars[*pos]);
        *pos += 1;
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(content: &str) -> ParseResult {
        parse(content, "en", PathBuf::from("test.strings"))
    }

    // --- Well-formed input ---

    #[test]
    fn single_key_value_pair() {
        let r = parse_str("\"login.button\" = \"Sign In\";");
        assert_eq!(r.file.keys.len(), 1);
        let k = r.file.keys.get("login.button").unwrap();
        assert_eq!(k.key, "login.button");
        assert_eq!(k.value.as_deref(), Some("Sign In"));
        assert!(k.comment.is_none());
    }

    #[test]
    fn block_comment_attaches_to_next_key() {
        let r = parse_str("/* Login button */\n\"key\" = \"val\";");
        assert_eq!(
            r.file.keys.get("key").unwrap().comment.as_deref(),
            Some("Login button")
        );
    }

    #[test]
    fn multiline_block_comment_preserved() {
        let r = parse_str("/* Line one\nLine two */\n\"key\" = \"val\";");
        let c = r.file.keys.get("key").unwrap().comment.as_deref().unwrap();
        assert!(c.contains("Line one"));
        assert!(c.contains("Line two"));
    }

    #[test]
    fn line_comment_does_not_attach() {
        let r = parse_str("// TODO\n\"key\" = \"val\";");
        assert!(r.file.keys.get("key").unwrap().comment.is_none());
    }

    #[test]
    fn multiple_keys_keep_order() {
        let r = parse_str("\"b\" = \"2\";\n\"a\" = \"1\";\n\"c\" = \"3\";");
        let keys: Vec<&str> = r.file.keys.keys().map(|s| s.as_str()).collect();
        // IndexMap preserves insertion order, not alphabetical
        assert_eq!(keys, vec!["b", "a", "c"]);
    }

    #[test]
    fn escape_newline_and_tab() {
        let r = parse_str(r#""k" = "A\nB\tC";"#);
        assert_eq!(
            r.file.keys.get("k").unwrap().value.as_deref(),
            Some("A\nB\tC")
        );
    }

    #[test]
    fn escaped_quotes_and_backslashes() {
        let r = parse_str(r#""k" = "\"\\";"#);
        assert_eq!(
            r.file.keys.get("k").unwrap().value.as_deref(),
            Some(r#""\"#)
        );
    }

    #[test]
    fn empty_value_is_valid() {
        let r = parse_str(r#""k" = "";"#);
        assert_eq!(r.file.keys.get("k").unwrap().value.as_deref(), Some(""));
    }

    #[test]
    fn unicode_in_key_and_value() {
        let r = parse_str("\"café\" = \"日本語\";");
        assert_eq!(
            r.file.keys.get("café").unwrap().value.as_deref(),
            Some("日本語")
        );
    }

    #[test]
    fn special_chars_inside_quotes() {
        // = and ; inside a quoted value are literal content
        let r = parse_str(r#""k" = "a=b;c";"#);
        assert_eq!(
            r.file.keys.get("k").unwrap().value.as_deref(),
            Some("a=b;c")
        );
    }

    #[test]
    fn crlf_line_endings() {
        let r = parse_str("\"k1\" = \"v1\";\r\n\"k2\" = \"v2\";\r\n");
        assert_eq!(r.file.keys.len(), 2);
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn unquoted_key() {
        let r = parse_str("key = \"value\";");
        assert_eq!(
            r.file.keys.get("key").unwrap().value.as_deref(),
            Some("value")
        );
    }

    #[test]
    fn unquoted_value() {
        let r = parse_str("\"key\" = value;");
        assert_eq!(
            r.file.keys.get("key").unwrap().value.as_deref(),
            Some("value")
        );
    }

    // --- Duplicates ---

    #[test]
    fn duplicate_last_value_wins() {
        let r = parse_str(r#""dup" = "first";"dup" = "second";"#);
        assert_eq!(r.file.keys.len(), 1);
        assert_eq!(
            r.file.keys.get("dup").unwrap().value.as_deref(),
            Some("second")
        );
        assert_eq!(r.duplicates, vec!["dup"]);
    }

    // --- Malformed input: no crash, no phantom keys ---

    #[test]
    fn empty_file_produces_nothing() {
        let r = parse_str("");
        assert!(r.file.keys.is_empty());
        assert!(r.warnings.is_empty());
        assert!(r.duplicates.is_empty());
    }

    #[test]
    fn whitespace_only_produces_nothing() {
        let r = parse_str("   \n\t\r\n");
        assert!(r.file.keys.is_empty());
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn missing_semicolon_does_not_insert_key() {
        let r = parse_str(r#""k" = "v""#);
        assert!(r.file.keys.is_empty());
        assert!(r.warnings.iter().any(|w| w.contains("Missing ';'")));
    }

    #[test]
    fn missing_equals_does_not_insert_key() {
        let r = parse_str(r#""k" "v";"#);
        assert!(r.file.keys.is_empty());
        assert!(r.warnings.iter().any(|w| w.contains("Expected '='")));
    }

    #[test]
    fn unterminated_string_does_not_insert_key() {
        let r = parse_str(r#""k" = "unterminated"#);
        assert!(r.file.keys.is_empty());
        assert!(r.warnings.iter().any(|w| w.contains("Unterminated")));
    }

    #[test]
    fn orphan_comment_no_warning() {
        let r = parse_str("/* just a comment */");
        assert!(r.file.keys.is_empty());
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn line_comment_after_key_value_is_ignored() {
        let r = parse_str("\"k\" = \"v\"; // trailing comment\n\"a\" = \"b\";");
        assert_eq!(r.file.keys.len(), 2);
        assert_eq!(r.file.keys.get("k").unwrap().value.as_deref(), Some("v"));
        assert_eq!(r.file.keys.get("a").unwrap().value.as_deref(), Some("b"));
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn carriage_return_escape() {
        let r = parse_str(r#""k" = "A\rB";"#);
        assert_eq!(r.file.keys.get("k").unwrap().value.as_deref(), Some("A\rB"));
    }

    #[test]
    fn unknown_escape_preserved_literally() {
        // \x is not a known escape — the parser keeps it as-is
        let r = parse_str(r#""k" = "A\xB";"#);
        assert_eq!(
            r.file.keys.get("k").unwrap().value.as_deref(),
            Some(r"A\xB")
        );
    }
}
