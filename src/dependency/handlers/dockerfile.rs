// Dockerfile handler for dependency updates

use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Read version from a Dockerfile by finding FROM or ARG lines with the package name
pub fn read_version(file_path: &Path, package_name: &str) -> Result<String> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    // Try to find version in FROM line: FROM package:version
    let from_pattern = format!(r"FROM\s+{}:([^\s]+)", regex::escape(package_name));
    let from_re = Regex::new(&from_pattern)?;

    if let Some(caps) = from_re.captures(&content) {
        return Ok(caps[1].to_string());
    }

    // Try to find version in ARG line: ARG PACKAGE_VERSION=version
    let arg_name = package_name.to_uppercase().replace('-', "_") + "_VERSION";
    let arg_pattern = format!(r"ARG\s+{}=([^\s]+)", regex::escape(&arg_name));
    let arg_re = Regex::new(&arg_pattern)?;

    if let Some(caps) = arg_re.captures(&content) {
        return Ok(caps[1].to_string());
    }

    Err(anyhow::anyhow!(
        "Version for package '{}' not found in Dockerfile",
        package_name
    ))
}

/// Update version in a Dockerfile
pub fn update_version(file_path: &Path, package_name: &str, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let mut updated_content = content.clone();
    let mut updated = false;

    // Update FROM line: FROM package:version
    let from_pattern = format!(r"FROM\s+{}:([^\s]+)", regex::escape(package_name));
    let from_re = Regex::new(&from_pattern)?;

    if from_re.is_match(&content) {
        let replacement = format!("FROM {}:{}", package_name, new_version);
        updated_content = from_re
            .replace(&updated_content, replacement.as_str())
            .to_string();
        updated = true;
    }

    // Update ARG line: ARG PACKAGE_VERSION=version
    let arg_name = package_name.to_uppercase().replace('-', "_") + "_VERSION";
    let arg_pattern = format!(r"ARG\s+{}=([^\s]+)", regex::escape(&arg_name));
    let arg_re = Regex::new(&arg_pattern)?;

    if arg_re.is_match(&content) {
        let replacement = format!("ARG {}={}", arg_name, new_version);
        updated_content = arg_re
            .replace(&updated_content, replacement.as_str())
            .to_string();
        updated = true;
    }

    if !updated {
        return Err(anyhow::anyhow!(
            "No version reference for package '{}' found in Dockerfile",
            package_name
        ));
    }

    fs::write(file_path, updated_content)
        .with_context(|| format!("Failed to write {}", file_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_version_from_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Dockerfile");

        fs::write(
            &file_path,
            r#"FROM node:18
FROM myapp:1.0.0
RUN echo "test"
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "myapp").unwrap();
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_read_version_from_arg() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Dockerfile");

        fs::write(
            &file_path,
            r#"ARG MYAPP_VERSION=1.5.0
FROM node:18
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "myapp").unwrap();
        assert_eq!(version, "1.5.0");
    }

    #[test]
    fn test_update_version_from_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Dockerfile");

        fs::write(
            &file_path,
            r#"FROM node:18
FROM myapp:1.0.0
RUN echo "test"
"#,
        )
        .unwrap();

        update_version(&file_path, "myapp", "2.0.0").unwrap();

        let version = read_version(&file_path, "myapp").unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_update_version_from_arg() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Dockerfile");

        fs::write(
            &file_path,
            r#"ARG MYAPP_VERSION=1.5.0
FROM myapp:${MYAPP_VERSION}
"#,
        )
        .unwrap();

        update_version(&file_path, "myapp", "2.5.0").unwrap();

        let version = read_version(&file_path, "myapp").unwrap();
        assert_eq!(version, "2.5.0");
    }

    #[test]
    fn test_package_with_hyphen() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Dockerfile");

        fs::write(
            &file_path,
            r#"ARG MY_APP_VERSION=1.0.0
FROM node:18
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "my-app").unwrap();
        assert_eq!(version, "1.0.0");

        update_version(&file_path, "my-app", "1.1.0").unwrap();

        let version = read_version(&file_path, "my-app").unwrap();
        assert_eq!(version, "1.1.0");
    }
}
