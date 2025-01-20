use committy::error::CliError;
use committy::version::VersionManager;
use std::fs;

#[test]
fn test_version_bump_only_updates_version_fields() -> Result<(), CliError> {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("pyproject.toml");

    // Create a test pyproject.toml with both a version field and a field containing version
    fs::write(
        &test_file,
        r#"
[project]
name = "test-project"
version = "1.0.0"
description = "A test project"

[tool.ruff]
target-version = "py310"
"#,
    )
    .unwrap();

    let mut manager = VersionManager::new();
    manager.add_version_file(
        &test_file,
        r#"(?m)^version\s*=\s*"(\d+\.\d+\.\d+)""#,
        r#"version = "{}""#,
    )?;

    // Try to bump the version
    manager.update_all_versions("2.0.0")?;

    // Read the file content after update
    let content = fs::read_to_string(&test_file).unwrap();

    // The version field should be updated
    assert!(
        content.contains(r#"version = "2.0.0""#),
        "Version field was not updated correctly"
    );

    // The target-version field should remain unchanged
    assert!(
        content.contains(r#"target-version = "py310""#),
        "target-version field was incorrectly modified"
    );

    Ok(())
}

#[test]
fn test_version_bump_with_various_formats() -> Result<(), CliError> {
    let temp_dir = tempfile::tempdir().unwrap();

    // Test Cargo.toml
    let cargo_file = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_file,
        r#"
[package]
name = "test-project"
version = "1.0.0"
rust-version = "1.70.0"
"#,
    )
    .unwrap();

    // Test package.json
    let package_file = temp_dir.path().join("package.json");
    fs::write(
        &package_file,
        r#"
{
  "name": "test-project",
  "version": "1.0.0",
  "engines": {
    "node-version": "16.x"
  }
}
"#,
    )
    .unwrap();

    let mut manager = VersionManager::new();

    // Register files with more specific patterns
    manager.add_version_file(
        &cargo_file,
        r#"(?m)^version\s*=\s*"(\d+\.\d+\.\d+)""#,
        r#"version = "{}""#,
    )?;

    manager.add_version_file(
        &package_file,
        r#""version":\s*"(\d+\.\d+\.\d+)""#,
        r#""version": "{}""#,
    )?;

    // Bump versions
    manager.update_all_versions("2.0.0")?;

    // Verify Cargo.toml
    let cargo_content = fs::read_to_string(&cargo_file).unwrap();
    assert!(cargo_content.contains(r#"version = "2.0.0""#));
    assert!(cargo_content.contains(r#"rust-version = "1.70.0""#));

    // Verify package.json
    let package_content = fs::read_to_string(&package_file).unwrap();
    assert!(package_content.contains(r#""version": "2.0.0""#));
    assert!(package_content.contains(r#""node-version": "16.x""#));

    Ok(())
}
