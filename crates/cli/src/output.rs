//! Output formatting: colors, escaping, display helpers.

use std::io::IsTerminal;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

pub struct Colors {
    pub reset: &'static str,
    pub green: &'static str,
    pub red: &'static str,
    pub yellow: &'static str,
    pub cyan: &'static str,
}

impl Colors {
    pub fn detect() -> Self {
        if std::io::stdout().is_terminal() {
            Self {
                reset: RESET,
                green: GREEN,
                red: RED,
                yellow: YELLOW,
                cyan: CYAN,
            }
        } else {
            Self {
                reset: "",
                green: "",
                red: "",
                yellow: "",
                cyan: "",
            }
        }
    }
}

/// Escape newlines/tabs for single-line display.
pub fn escape_value(v: &str) -> String {
    v.replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

/// Escape a string for JSON output.
pub fn escape_json(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
