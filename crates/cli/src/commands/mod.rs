//! Command implementations.

pub mod add_key;
pub mod missing;
pub mod search;
pub mod set;

use locus_core::project;

/// Load a project from a path, mapping errors to user-friendly strings.
pub fn load_project(root: &std::path::Path) -> Result<locus_core::model::Project, String> {
    project::scan_project(root).map_err(|e| match e {
        project::ScanError::NotFound(p) => format!("directory not found: {}", p),
        project::ScanError::Io(e) => format!("IO error: {}", e),
        project::ScanError::Parse(e) => format!("parse error: {}", e),
    })
}

/// Resolve the logical file name: use the provided one, or auto-detect
/// if there's exactly one file in the project.
pub fn resolve_file(
    proj: &locus_core::model::Project,
    file: Option<&str>,
) -> Result<String, String> {
    match file {
        Some(f) => {
            if !proj.files.contains_key(f) {
                return Err(format!("file '{}' not found in project", f));
            }
            Ok(f.to_string())
        }
        None => {
            let names: Vec<&String> = proj.files.keys().collect();
            match names.len() {
                0 => Err("no localization files found in project".to_string()),
                1 => Ok(names[0].clone()),
                _ => Err(format!(
                    "multiple files found ({}), specify --file",
                    names
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
    }
}
