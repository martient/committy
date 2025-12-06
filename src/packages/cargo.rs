// Cargo (Rust) package detector

use super::types::{PackageDetector, PackageInfo, PackageManager};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use toml_edit::{value, DocumentMut};

pub struct CargoDetector;

impl PackageDetector for CargoDetector {
    fn detect(&self, path: &Path) -> Result<Option<PackageInfo>> {
        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&cargo_toml)
            .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

        let doc: DocumentMut = content
            .parse()
            .with_context(|| format!("Failed to parse {}", cargo_toml.display()))?;

        // Check if it's a workspace
        let is_workspace = doc.get("workspace").is_some();
        let has_package = doc.get("package").is_some();

        // Get workspace members if it's a workspace
        let mut workspace_members = Vec::new();
        if is_workspace {
            if let Some(workspace) = doc.get("workspace") {
                if let Some(members) = workspace.get("members") {
                    if let Some(members_array) = members.as_array() {
                        for member in members_array.iter() {
                            if let Some(member_str) = member.as_str() {
                                workspace_members.push(member_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        // If it's a workspace without a package section, we'll detect it but note it's workspace-only
        let name = if has_package {
            doc.get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("No package name in Cargo.toml"))?
                .to_string()
        } else if is_workspace {
            // Workspace-only Cargo.toml
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("workspace")
                .to_string()
        } else {
            return Err(anyhow::anyhow!(
                "Cargo.toml has neither [package] nor [workspace] section"
            ));
        };

        let version = if has_package {
            self.get_version(path)?
        } else {
            "0.0.0".to_string() // Workspace-only has no version
        };

        Ok(Some(PackageInfo {
            name,
            manager: PackageManager::Cargo {
                workspace: is_workspace,
            },
            path: path.to_path_buf(),
            version,
            version_file: "Cargo.toml".to_string(),
            version_field: "package.version".to_string(),
            workspace_members,
        }))
    }

    fn get_version(&self, path: &Path) -> Result<String> {
        let cargo_toml = path.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml)
            .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

        let doc: DocumentMut = content
            .parse()
            .with_context(|| format!("Failed to parse {}", cargo_toml.display()))?;

        let version = doc
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No version in Cargo.toml"))?
            .to_string();

        Ok(version)
    }

    fn set_version(&self, path: &Path, version: &str) -> Result<()> {
        let cargo_toml = path.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml)
            .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

        let mut doc: DocumentMut = content
            .parse()
            .with_context(|| format!("Failed to parse {}", cargo_toml.display()))?;

        // Update version
        if let Some(package) = doc.get_mut("package") {
            if let Some(package_table) = package.as_table_mut() {
                package_table["version"] = value(version);
            }
        } else {
            return Err(anyhow::anyhow!("No [package] section in Cargo.toml"));
        }

        fs::write(&cargo_toml, doc.to_string())
            .with_context(|| format!("Failed to write {}", cargo_toml.display()))?;

        Ok(())
    }

    fn name(&self) -> &str {
        "Cargo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_cargo_package() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-package"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let detector = CargoDetector;
        let result = detector.detect(temp_dir.path()).unwrap();

        assert!(result.is_some());
        let pkg = result.unwrap();
        assert_eq!(pkg.name, "test-package");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.version_file, "Cargo.toml");
        assert!(!pkg.is_workspace());
    }

    #[test]
    fn test_detect_cargo_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"
[workspace]
members = ["cli", "lib", "server"]

[package]
name = "workspace-root"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let detector = CargoDetector;
        let result = detector.detect(temp_dir.path()).unwrap();

        assert!(result.is_some());
        let pkg = result.unwrap();
        assert_eq!(pkg.name, "workspace-root");
        assert!(pkg.is_workspace());
        assert_eq!(pkg.workspace_members.len(), 3);
        assert!(pkg.workspace_members.contains(&"cli".to_string()));
    }

    #[test]
    fn test_get_version() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-package"
version = "2.5.3"
edition = "2021"
        "#,
        )
        .unwrap();

        let detector = CargoDetector;
        let version = detector.get_version(temp_dir.path()).unwrap();
        assert_eq!(version, "2.5.3");
    }

    #[test]
    fn test_set_version() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-package"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let detector = CargoDetector;
        detector.set_version(temp_dir.path(), "2.0.0").unwrap();

        let version = detector.get_version(temp_dir.path()).unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_no_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let detector = CargoDetector;
        let result = detector.detect(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }
}
