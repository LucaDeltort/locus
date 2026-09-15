//! Integration tests for the CLI binary.
//!
//! Each test builds a temporary project on disk, invokes the `locus`
//! binary via `assert_cmd`, and checks stdout / exit code.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Create a temp dir with a minimal .strings project (en + fr).
fn strings_project() -> TempDir {
    let dir = TempDir::new().unwrap();
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

    dir
}

/// Create a temp dir with a minimal .xcstrings project.
fn xcstrings_project() -> TempDir {
    let dir = TempDir::new().unwrap();
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

// --- search ---

#[test]
fn search_by_key_returns_matching_key() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([dir.path().to_str().unwrap(), "search", "--key", "login"])
        .assert()
        .success()
        .stdout(predicates::str::contains("login.button"));
}

#[test]
fn search_by_text_matches_value() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([
        dir.path().to_str().unwrap(),
        "search",
        "--text",
        "Connexion",
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains("login.button"));
}

#[test]
fn search_no_results_still_succeeds() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([
        dir.path().to_str().unwrap(),
        "search",
        "--key",
        "nonexistent",
    ])
    .assert()
    .success()
    .stdout(predicates::str::contains("No results"));
}

#[test]
fn search_without_key_or_text_fails() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([dir.path().to_str().unwrap(), "search"])
        .assert()
        .failure();
}

// --- missing ---

#[test]
fn missing_reports_keys_absent_for_fr() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([dir.path().to_str().unwrap(), "missing", "--lang", "fr"])
        .assert()
        .success()
        .stdout(predicates::str::contains("logout.button"));
}

#[test]
fn missing_nothing_reports_none() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([dir.path().to_str().unwrap(), "missing", "--lang", "en"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No missing keys"));
}

#[test]
fn missing_json_output_is_valid_array() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    let output = cmd
        .args([
            dir.path().to_str().unwrap(),
            "missing",
            "--lang",
            "fr",
            "--output",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "invalid JSON: {}\n---\n{}\n---",
            e,
            String::from_utf8_lossy(&output.stdout)
        );
    });
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(
        json[0]["key"],
        serde_json::Value::String("logout.button".to_string())
    );
}

// --- set ---

#[test]
fn set_value_writes_to_disk_and_rescan_sees_it() {
    use std::path::PathBuf;

    let dir = strings_project();
    let root = dir.path();

    // Set logout.button for fr
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([
        root.to_str().unwrap(),
        "set",
        "--key",
        "logout.button",
        "--lang",
        "fr",
        "--value",
        "Déconnexion",
    ])
    .assert()
    .success();

    // Verify on disk by re-parsing
    let content = fs::read_to_string(root.join("fr.lproj/Localizable.strings")).unwrap();
    let parsed = locus_core::parser::parse(&content, "fr", PathBuf::from("fr.strings"));
    assert_eq!(
        parsed
            .file
            .keys
            .get("logout.button")
            .unwrap()
            .value
            .as_deref(),
        Some("Déconnexion")
    );

    // Re-scan and check missing is empty for fr
    let proj = locus_core::project::scan_project(root).unwrap();
    let missing = locus_core::missing::find_missing(&proj, "fr");
    assert!(missing.is_empty());
}

// --- add-key ---

#[test]
fn add_key_creates_new_key_on_disk() {
    let dir = strings_project();
    let root = dir.path();

    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([
        root.to_str().unwrap(),
        "add-key",
        "--key",
        "settings.title",
        "--lang",
        "en",
        "--value",
        "Settings",
    ])
    .assert()
    .success();

    // The new key should be visible in en.lproj after rescan
    let proj = locus_core::project::scan_project(root).unwrap();
    let en = proj
        .files
        .get("Localizable")
        .unwrap()
        .locales
        .get("en")
        .unwrap();
    assert_eq!(
        en.keys.get("settings.title").unwrap().value.as_deref(),
        Some("Settings")
    );

    // And it should show up in missing for fr
    let missing = locus_core::missing::find_missing(&proj, "fr");
    assert!(missing.iter().any(|m| m.key == "settings.title"));
}

#[test]
fn add_key_refuses_duplicate() {
    let dir = strings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([
        dir.path().to_str().unwrap(),
        "add-key",
        "--key",
        "login.button",
        "--lang",
        "en",
        "--value",
        "Duplicate",
    ])
    .assert()
    .failure()
    .stderr(predicates::str::contains("already exists"));
}

// --- xcstrings format ---

#[test]
fn xcstrings_search_finds_key() {
    let dir = xcstrings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([dir.path().to_str().unwrap(), "search", "--key", "login"])
        .assert()
        .success()
        .stdout(predicates::str::contains("login.button"));
}

#[test]
fn xcstrings_missing_detects_logout_for_fr() {
    let dir = xcstrings_project();
    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([dir.path().to_str().unwrap(), "missing", "--lang", "fr"])
        .assert()
        .success()
        .stdout(predicates::str::contains("logout.button"));
}

#[test]
fn xcstrings_set_value_persists_to_json_file() {
    let dir = xcstrings_project();
    let root = dir.path();

    let mut cmd = Command::cargo_bin("locus").unwrap();
    cmd.args([
        root.to_str().unwrap(),
        "set",
        "--key",
        "logout.button",
        "--lang",
        "fr",
        "--value",
        "Déconnexion",
    ])
    .assert()
    .success();

    // Re-scan and verify
    let proj = locus_core::project::scan_project(root).unwrap();
    let fr = proj
        .files
        .get("Localizable")
        .unwrap()
        .locales
        .get("fr")
        .unwrap();
    assert_eq!(
        fr.keys.get("logout.button").unwrap().value.as_deref(),
        Some("Déconnexion")
    );

    // No more missing for fr
    let missing = locus_core::missing::find_missing(&proj, "fr");
    assert!(missing.is_empty());
}
