mod common;

use committy::version::VersionManager;
use std::fs;
use tempfile::tempdir;

fn setup_version_files() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    // Create test version files
    let cargo_toml = dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test-package"
version = "1.0.0"
"#,
    )
    .expect("Failed to write Cargo.toml");

    let package_json = dir.path().join("package.json");
    fs::write(
        &package_json,
        r#"{
  "name": "test-package",
  "version": "1.0.0"
}"#,
    )
    .expect("Failed to write package.json");

    dir
}

#[test]
fn test_version_update() {
    let temp_dir = setup_version_files();
    let mut version_manager = VersionManager::new();

    // Add version files with their patterns
    version_manager
        .add_version_file(
            temp_dir.path().join("Cargo.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add Cargo.toml");

    version_manager
        .add_version_file(
            temp_dir.path().join("package.json"),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add package.json");

    // Update versions
    let updated_files = version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to update versions");

    assert_eq!(updated_files.len(), 2);

    // Verify Cargo.toml
    let cargo_contents =
        fs::read_to_string(temp_dir.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_contents.contains("version = \"2.0.0\""));

    // Verify package.json
    let package_contents = fs::read_to_string(temp_dir.path().join("package.json"))
        .expect("Failed to read package.json");
    assert!(package_contents.contains("\"version\": \"2.0.0\""));
}

#[test]
fn test_nonexistent_file() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let mut version_manager = VersionManager::new();

    // Add a non-existent file
    version_manager
        .add_version_file(
            temp_dir.path().join("nonexistent.json"),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add nonexistent file");

    // Update should succeed but not update any files
    let updated_files = version_manager
        .update_all_versions("1.0.0")
        .expect("Failed to handle nonexistent file");
    assert_eq!(updated_files.len(), 0);
}

#[test]
fn test_invalid_version_pattern() {
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Create file with different version format
    let invalid_file = temp_dir.path().join("invalid.toml");
    fs::write(
        &invalid_file,
        r#"[package]
name = "test-package"
version = v1.0.0
"#,
    )
    .expect("Failed to write invalid.toml");

    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(
            &invalid_file,
            r#"version\s*=\s*"[^"]*""#, // This pattern won't match because version isn't in quotes
            r#"version = "{}""#,
        )
        .expect("Failed to add invalid file");

    // Update should succeed but file should remain unchanged
    version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to handle invalid pattern");

    // Verify file wasn't changed
    let contents = fs::read_to_string(&invalid_file).expect("Failed to read invalid.toml");
    assert!(contents.contains("version = v1.0.0"));
}

#[test]
fn test_multiple_version_patterns() {
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Create file with multiple version occurrences
    let multi_version_file = temp_dir.path().join("multi.toml");
    fs::write(
        &multi_version_file,
        r#"[package]
name = "test-package"
version = "1.0.0"
other_version = "2.0.0"
"#,
    )
    .expect("Failed to write multi.toml");

    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(
            &multi_version_file,
            r#"(?m)^\s*version\s*=\s*"[^"]*""#, // Add (?m)^ to match only at line start
            r#"version = "{}""#,
        )
        .expect("Failed to add multi-version file");

    version_manager
        .update_all_versions("1.1.0")
        .expect("Failed to update version");

    // Verify only the correct version was updated
    let contents = fs::read_to_string(&multi_version_file).expect("Failed to read multi.toml");
    assert!(contents.contains("version = \"1.1.0\""));
    assert!(contents.contains("other_version = \"2.0.0\""));
}

#[test]
fn test_register_common_files() {
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Create common version files
    let files = vec![
        (
            "Cargo.toml",
            r#"[package]
name = "test"
version = "1.0.0""#,
        ),
        (
            "package.json",
            r#"{
  "name": "test",
  "version": "1.0.0"
}"#,
        ),
        (
            "pyproject.toml",
            r#"[project]
name = "test"
version = "1.0.0""#,
        ),
    ];

    for (name, content) in files {
        let file_path = temp_dir.path().join(name);
        fs::write(&file_path, content).unwrap_or_else(|_| panic!("Failed to write {}", name));
    }

    // Register and update all common files
    let mut version_manager = VersionManager::new();

    // Add each file with absolute path
    version_manager
        .add_version_file(
            temp_dir.path().join("Cargo.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add Cargo.toml");

    version_manager
        .add_version_file(
            temp_dir.path().join("package.json"),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add package.json");

    version_manager
        .add_version_file(
            temp_dir.path().join("pyproject.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add pyproject.toml");

    // Update all versions
    version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to update versions");

    // Verify updates
    let cargo_toml =
        fs::read_to_string(temp_dir.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("version = \"2.0.0\""));

    let package_json = fs::read_to_string(temp_dir.path().join("package.json"))
        .expect("Failed to read package.json");
    assert!(package_json.contains("\"version\": \"2.0.0\""));

    let pyproject_toml = fs::read_to_string(temp_dir.path().join("pyproject.toml"))
        .expect("Failed to read pyproject.toml");
    assert!(pyproject_toml.contains("version = \"2.0.0\""));
}

#[test]
fn test_version_with_v_prefix() {
    let temp_dir = setup_version_files();
    let mut version_manager = VersionManager::new();

    version_manager
        .add_version_file(
            temp_dir.path().join("Cargo.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add Cargo.toml");

    // Update with v-prefixed version
    let updated_files = version_manager
        .update_all_versions("v2.0.0")
        .expect("Failed to update versions");

    assert_eq!(updated_files.len(), 1);

    // Verify the v prefix was stripped
    let cargo_contents =
        fs::read_to_string(temp_dir.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_contents.contains("version = \"2.0.0\""));
}

#[test]
fn test_file_with_multiple_versions() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let test_file = temp_dir.path().join("test.json");
    
    // Create a file with multiple version fields
    fs::write(
        &test_file,
        r#"{
  "version": "1.0.0",
  "dependencies": {
    "some-pkg": {
      "version": "1.0.0"
    }
  }
}"#,
    ).expect("Failed to write test file");

    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(
            test_file.clone(),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add test file");

    let updated_files = version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to update versions");

    assert_eq!(updated_files.len(), 1);

    // Verify both versions were updated
    let contents = fs::read_to_string(test_file).expect("Failed to read test file");
    let version_count = contents.matches("\"version\": \"2.0.0\"").count();
    assert_eq!(version_count, 2, "Both version fields should be updated");
}
