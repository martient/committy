// npm/pnpm/yarn (Node.js) package detector

use super::types::{PackageDetector, PackageInfo, PackageManager};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct NpmDetector;

impl PackageDetector for NpmDetector {
    fn detect(&self, path: &Path) -> Result<Option<PackageInfo>> {
        let package_json = path.join("package.json");
        if !package_json.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&package_json)
            .with_context(|| format!("Failed to read {}", package_json.display()))?;

        let json: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", package_json.display()))?;

        // Get package name
        let name = json
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow::anyhow!("No name in package.json"))?
            .to_string();

        // Check if it's a workspace
        let is_workspace = json.get("workspaces").is_some();

        // Get workspace members if it's a workspace
        let mut workspace_members = Vec::new();
        if is_workspace {
            if let Some(workspaces) = json.get("workspaces") {
                // workspaces can be an array or an object with "packages" field
                let workspaces_array = if workspaces.is_array() {
                    workspaces.as_array()
                } else {
                    workspaces.get("packages").and_then(|p| p.as_array())
                };

                if let Some(arr) = workspaces_array {
                    for member in arr.iter() {
                        if let Some(member_str) = member.as_str() {
                            workspace_members.push(member_str.to_string());
                        }
                    }
                }
            }
        }

        // Determine package manager type
        let manager = self.detect_package_manager(path, is_workspace)?;

        let version = self.get_version(path)?;

        Ok(Some(PackageInfo {
            name,
            manager,
            path: path.to_path_buf(),
            version,
            version_file: "package.json".to_string(),
            version_field: "version".to_string(),
            workspace_members,
        }))
    }

    fn get_version(&self, path: &Path) -> Result<String> {
        let package_json = path.join("package.json");
        let content = fs::read_to_string(&package_json)
            .with_context(|| format!("Failed to read {}", package_json.display()))?;

        let json: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", package_json.display()))?;

        let version = json
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No version in package.json"))?
            .to_string();

        Ok(version)
    }

    fn set_version(&self, path: &Path, version: &str) -> Result<()> {
        let package_json = path.join("package.json");
        let content = fs::read_to_string(&package_json)
            .with_context(|| format!("Failed to read {}", package_json.display()))?;

        let mut json: Value = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", package_json.display()))?;

        // Update version
        if let Some(obj) = json.as_object_mut() {
            obj.insert("version".to_string(), json!(version));
        } else {
            return Err(anyhow::anyhow!("package.json is not an object"));
        }

        // Write back with pretty formatting
        let formatted =
            serde_json::to_string_pretty(&json).context("Failed to serialize package.json")?;

        fs::write(&package_json, formatted)
            .with_context(|| format!("Failed to write {}", package_json.display()))?;

        Ok(())
    }

    fn name(&self) -> &str {
        "npm"
    }
}

impl NpmDetector {
    /// Detect which package manager is being used (npm, pnpm, or yarn)
    fn detect_package_manager(&self, path: &Path, is_workspace: bool) -> Result<PackageManager> {
        // Check for lock files to determine package manager
        if path.join("pnpm-lock.yaml").exists() {
            return Ok(PackageManager::Pnpm {
                workspace: is_workspace,
            });
        }

        if path.join("yarn.lock").exists() {
            return Ok(PackageManager::Yarn {
                workspace: is_workspace,
            });
        }

        // Default to npm
        Ok(PackageManager::Npm {
            workspace: is_workspace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_npm_package() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"
{
  "name": "test-package",
  "version": "1.0.0",
  "description": "Test package"
}
        "#,
        )
        .unwrap();

        let detector = NpmDetector;
        let result = detector.detect(temp_dir.path()).unwrap();

        assert!(result.is_some());
        let pkg = result.unwrap();
        assert_eq!(pkg.name, "test-package");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.version_file, "package.json");
        assert!(!pkg.is_workspace());
        assert_eq!(pkg.manager.name(), "npm");
    }

    #[test]
    fn test_detect_npm_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"
{
  "name": "workspace-root",
  "version": "1.0.0",
  "workspaces": ["packages/*", "apps/*"]
}
        "#,
        )
        .unwrap();

        let detector = NpmDetector;
        let result = detector.detect(temp_dir.path()).unwrap();

        assert!(result.is_some());
        let pkg = result.unwrap();
        assert_eq!(pkg.name, "workspace-root");
        assert!(pkg.is_workspace());
        assert_eq!(pkg.workspace_members.len(), 2);
        assert!(pkg.workspace_members.contains(&"packages/*".to_string()));
    }

    #[test]
    fn test_detect_pnpm() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        let pnpm_lock = temp_dir.path().join("pnpm-lock.yaml");

        fs::write(
            &package_json,
            r#"
{
  "name": "test-package",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        fs::write(&pnpm_lock, "# pnpm lock file").unwrap();

        let detector = NpmDetector;
        let result = detector.detect(temp_dir.path()).unwrap();

        assert!(result.is_some());
        let pkg = result.unwrap();
        assert_eq!(pkg.manager.name(), "pnpm");
    }

    #[test]
    fn test_detect_yarn() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        let yarn_lock = temp_dir.path().join("yarn.lock");

        fs::write(
            &package_json,
            r#"
{
  "name": "test-package",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        fs::write(&yarn_lock, "# yarn lock file").unwrap();

        let detector = NpmDetector;
        let result = detector.detect(temp_dir.path()).unwrap();

        assert!(result.is_some());
        let pkg = result.unwrap();
        assert_eq!(pkg.manager.name(), "Yarn");
    }

    #[test]
    fn test_get_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"
{
  "name": "test-package",
  "version": "2.5.3"
}
        "#,
        )
        .unwrap();

        let detector = NpmDetector;
        let version = detector.get_version(temp_dir.path()).unwrap();
        assert_eq!(version, "2.5.3");
    }

    #[test]
    fn test_set_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        fs::write(
            &package_json,
            r#"
{
  "name": "test-package",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        let detector = NpmDetector;
        detector.set_version(temp_dir.path(), "2.0.0").unwrap();

        let version = detector.get_version(temp_dir.path()).unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_no_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let detector = NpmDetector;
        let result = detector.detect(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }
}
