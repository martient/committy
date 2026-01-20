// Multi-package detector that orchestrates all package manager detectors

use super::cargo::CargoDetector;
use super::npm::NpmDetector;
use super::types::{PackageDetector, PackageInfo};
use anyhow::Result;
use log::debug;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Multi-package detector that can detect various package managers
pub struct MultiPackageDetector {
    detectors: Vec<Box<dyn PackageDetector>>,
    max_depth: usize,
}

impl MultiPackageDetector {
    /// Create a new multi-package detector with default detectors
    pub fn new() -> Self {
        Self {
            detectors: vec![
                Box::new(CargoDetector),
                Box::new(NpmDetector),
                // Add more detectors here as they're implemented
            ],
            max_depth: 5, // Q5: Default max depth of 5
        }
    }

    /// Create a new multi-package detector with custom max depth
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Detect all packages in the repository
    pub fn detect_all(&self, repo_path: &Path) -> Result<Vec<PackageInfo>> {
        debug!("Detecting packages in {}", repo_path.display());
        let mut packages = Vec::new();
        let mut visited = HashSet::new();

        self.detect_recursive(repo_path, repo_path, 0, &mut packages, &mut visited)?;

        debug!("Found {} packages", packages.len());
        Ok(packages)
    }

    /// Recursively detect packages
    fn detect_recursive(
        &self,
        repo_root: &Path,
        current_path: &Path,
        depth: usize,
        packages: &mut Vec<PackageInfo>,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        // Check max depth (Q5: Configurable max depth)
        if depth > self.max_depth {
            return Ok(());
        }

        // Avoid infinite loops
        let canonical = current_path
            .canonicalize()
            .unwrap_or_else(|_| current_path.to_path_buf());
        if !visited.insert(canonical) {
            return Ok(());
        }

        debug!("Scanning {} (depth: {})", current_path.display(), depth);

        // Try each detector
        for detector in &self.detectors {
            if let Some(mut pkg) = detector.detect(current_path)? {
                debug!(
                    "Detected {} package: {} at {}",
                    detector.name(),
                    pkg.name,
                    current_path.display()
                );

                // Make path relative to repo root and normalize root to "." for display
                let relative_path = current_path
                    .strip_prefix(repo_root)
                    .unwrap_or(current_path);

                pkg.path = if relative_path.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    relative_path.to_path_buf()
                };

                // Q6: Detect both workspace root and members, mark relationship
                packages.push(pkg.clone());

                // If it's a workspace, detect members
                if pkg.is_workspace() && !pkg.workspace_members.is_empty() {
                    self.detect_workspace_members(
                        repo_root,
                        current_path,
                        &pkg.workspace_members,
                        depth,
                        packages,
                        visited,
                    )?;
                }

                // Only allow deeper traversal from the repo root; otherwise stop here
                if current_path != repo_root {
                    return Ok(());
                }

                break;
            }
        }

        // Recurse into subdirectories when appropriate. If a package was detected at the
        // repository root we still want to scan siblings (other top-level packages). For
        // detected non-root packages we return early above.
        if current_path.is_dir() {
            for entry in fs::read_dir(current_path)? {
                let entry = entry?;
                let path = entry.path();

                // Skip hidden directories and common ignore patterns
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if self.should_skip(name) {
                        continue;
                    }
                }

                if path.is_dir() {
                    self.detect_recursive(repo_root, &path, depth + 1, packages, visited)?;
                }
            }
        }

        Ok(())
    }

    /// Detect workspace members
    fn detect_workspace_members(
        &self,
        repo_root: &Path,
        workspace_root: &Path,
        members: &[String],
        depth: usize,
        packages: &mut Vec<PackageInfo>,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        for member_pattern in members {
            // Handle glob patterns (simple implementation)
            if member_pattern.contains('*') {
                // Expand glob pattern
                let base_path = member_pattern.split('*').next().unwrap_or("");
                let search_path = workspace_root.join(base_path);

                if search_path.exists() && search_path.is_dir() {
                    for entry in fs::read_dir(&search_path)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_dir() {
                            self.detect_recursive(repo_root, &path, depth + 1, packages, visited)?;
                        }
                    }
                }
            } else {
                // Direct member path
                let member_path = workspace_root.join(member_pattern);
                if member_path.exists() {
                    self.detect_recursive(repo_root, &member_path, depth + 1, packages, visited)?;
                }
            }
        }

        Ok(())
    }

    /// Check if a directory should be skipped
    fn should_skip(&self, name: &str) -> bool {
        // Skip hidden directories
        if name.starts_with('.') {
            return true;
        }

        // Skip common ignore patterns
        matches!(
            name,
            "node_modules" | "target" | "dist" | "build" | ".git" | ".svn" | ".hg"
        )
    }
}

impl Default for MultiPackageDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_single_cargo_package() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-package"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let detector = MultiPackageDetector::new();
        let packages = detector.detect_all(temp_dir.path()).unwrap();

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "test-package");
    }

    #[test]
    fn test_detect_multiple_packages() {
        let temp_dir = TempDir::new().unwrap();

        // Create two separate directories with packages
        let rust_dir = temp_dir.path().join("rust-app");
        fs::create_dir(&rust_dir).unwrap();
        fs::write(
            rust_dir.join("Cargo.toml"),
            r#"
[package]
name = "rust-package"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        // Create npm package in separate directory
        let ui_dir = temp_dir.path().join("ui");
        fs::create_dir(&ui_dir).unwrap();
        fs::write(
            ui_dir.join("package.json"),
            r#"
{
  "name": "ui-package",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        let detector = MultiPackageDetector::new();
        let packages = detector.detect_all(temp_dir.path()).unwrap();

        assert_eq!(packages.len(), 2);
        assert!(packages.iter().any(|p| p.name == "rust-package"));
        assert!(packages.iter().any(|p| p.name == "ui-package"));
    }

    #[test]
    fn test_detect_cargo_workspace() {
        let temp_dir = TempDir::new().unwrap();

        // Create workspace root
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["cli", "lib"]

[package]
name = "workspace-root"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        // Create cli member
        let cli_dir = temp_dir.path().join("cli");
        fs::create_dir(&cli_dir).unwrap();
        fs::write(
            cli_dir.join("Cargo.toml"),
            r#"
[package]
name = "cli"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        // Create lib member
        let lib_dir = temp_dir.path().join("lib");
        fs::create_dir(&lib_dir).unwrap();
        fs::write(
            lib_dir.join("Cargo.toml"),
            r#"
[package]
name = "lib"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let detector = MultiPackageDetector::new();
        let packages = detector.detect_all(temp_dir.path()).unwrap();

        // Should detect workspace root + 2 members = 3 packages
        assert_eq!(packages.len(), 3);
        assert!(packages.iter().any(|p| p.name == "workspace-root"));
        assert!(packages.iter().any(|p| p.name == "cli"));
        assert!(packages.iter().any(|p| p.name == "lib"));
    }

    #[test]
    fn test_max_depth() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested structure
        let mut current = temp_dir.path().to_path_buf();
        for i in 0..10 {
            current = current.join(format!("level{}", i));
            fs::create_dir(&current).unwrap();
        }

        // Add package at depth 10
        fs::write(
            current.join("package.json"),
            r#"
{
  "name": "deep-package",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        // With max_depth = 5, should not find the package
        let detector = MultiPackageDetector::new().with_max_depth(5);
        let packages = detector.detect_all(temp_dir.path()).unwrap();
        assert_eq!(packages.len(), 0);

        // With max_depth = 15, should find it
        let detector = MultiPackageDetector::new().with_max_depth(15);
        let packages = detector.detect_all(temp_dir.path()).unwrap();
        assert_eq!(packages.len(), 1);
    }

    #[test]
    fn test_skip_node_modules() {
        let temp_dir = TempDir::new().unwrap();

        // Create package at root
        fs::write(
            temp_dir.path().join("package.json"),
            r#"
{
  "name": "root-package",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        // Create package in node_modules (should be skipped)
        let node_modules = temp_dir.path().join("node_modules").join("some-dep");
        fs::create_dir_all(&node_modules).unwrap();
        fs::write(
            node_modules.join("package.json"),
            r#"
{
  "name": "dependency",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        let detector = MultiPackageDetector::new();
        let packages = detector.detect_all(temp_dir.path()).unwrap();

        // Should only find root package, not the one in node_modules
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "root-package");
    }
}
