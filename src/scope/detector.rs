// Scope detector for multi-package repositories

use super::matcher::{FileMatcher, ScopeMapping};
use crate::config::repository::RepositoryConfig;
use crate::packages::MultiPackageDetector;
use anyhow::{Context, Result};
use log::debug;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Scope detector for determining affected scopes from changed files
pub struct ScopeDetector {
    #[cfg_attr(not(test), allow(dead_code))]
    config: RepositoryConfig,
    repo_path: PathBuf,
    file_matcher: FileMatcher,
}

impl ScopeDetector {
    /// Create a new scope detector
    pub fn new(config: RepositoryConfig, repo_path: &Path) -> Self {
        // Build file matcher from config mappings
        let mappings = config
            .scopes
            .mappings
            .iter()
            .map(|m| ScopeMapping::new(m.pattern.clone(), m.scope.clone()))
            .collect();

        let file_matcher = FileMatcher::new(mappings);

        Self {
            config,
            repo_path: repo_path.to_path_buf(),
            file_matcher,
        }
    }

    /// Detect scopes from git staged files (Q16: Staged files)
    pub fn detect_from_staged(&self) -> Result<Vec<String>> {
        let staged_files = self.get_staged_files()?;
        debug!("Staged files: {:?}", staged_files);

        if staged_files.is_empty() {
            return Ok(Vec::new());
        }

        self.detect_from_files(&staged_files)
    }

    /// Detect scopes from a list of file paths
    pub fn detect_from_files(&self, files: &[PathBuf]) -> Result<Vec<String>> {
        let mut scopes = HashSet::new();

        // Convert to Path references
        let file_refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();

        // First, try file pattern matching from config
        let matched_scopes = self.file_matcher.find_scopes(&file_refs)?;
        for scope in matched_scopes {
            scopes.insert(scope);
        }

        // If no scopes found from patterns, try package-based detection (Q17: Both)
        if scopes.is_empty() {
            let package_scopes = self.detect_from_packages(&file_refs)?;
            for scope in package_scopes {
                scopes.insert(scope);
            }
        }

        Ok(scopes.into_iter().collect())
    }

    /// Detect scopes based on which packages contain the files
    fn detect_from_packages(&self, files: &[&Path]) -> Result<Vec<String>> {
        let mut scopes = HashSet::new();

        // Detect packages
        let detector = MultiPackageDetector::new();
        let packages = detector.detect_all(&self.repo_path)?;

        for file in files {
            // Normalize the file path (handle both absolute and relative)
            let file_path = if file.is_absolute() {
                file.strip_prefix(&self.repo_path).unwrap_or(file)
            } else {
                file
            };

            // Find which package this file belongs to
            for pkg in &packages {
                if file_path.starts_with(&pkg.path) {
                    // File is in this package
                    // Use package name as scope
                    scopes.insert(pkg.name.clone());
                    break;
                }
            }
        }

        Ok(scopes.into_iter().collect())
    }

    /// Get list of staged files from git
    fn get_staged_files(&self) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-only"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run git diff")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("git diff failed"));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let files: Vec<PathBuf> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect();

        Ok(files)
    }

    /// Suggest scopes based on available packages and config
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn suggest_scopes(&self) -> Result<Vec<String>> {
        let mut scopes = HashSet::new();

        // Add scopes from config mappings
        for mapping in &self.config.scopes.mappings {
            scopes.insert(mapping.scope.clone());
        }

        // Add package names as potential scopes
        let detector = MultiPackageDetector::new();
        let packages = detector.detect_all(&self.repo_path)?;
        for pkg in packages {
            scopes.insert(pkg.name);
        }

        let mut scope_list: Vec<String> = scopes.into_iter().collect();
        scope_list.sort();
        Ok(scope_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::repository::{
        PackageConfig, RepositoryConfig, RepositoryMetadata, RepositoryType, VersioningConfig,
        VersioningStrategy,
    };
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> RepositoryConfig {
        use crate::config::repository::ScopeConfig;
        use crate::config::repository::ScopeMapping as ConfigScopeMapping;

        let scopes = ScopeConfig {
            auto_detect: true,
            require_scope_for_multi_package: true,
            allow_multiple_scopes: true,
            scope_separator: ",".to_string(),
            mappings: vec![
                ConfigScopeMapping {
                    pattern: "cli/**/*".to_string(),
                    scope: "cli".to_string(),
                    package: "cli".to_string(),
                    description: None,
                },
                ConfigScopeMapping {
                    pattern: "server/**/*".to_string(),
                    scope: "server".to_string(),
                    package: "server".to_string(),
                    description: None,
                },
                ConfigScopeMapping {
                    pattern: "docs/**/*".to_string(),
                    scope: "docs".to_string(),
                    package: "docs".to_string(),
                    description: None,
                },
            ],
        };

        RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "cli".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "cli".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: true,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "server".to_string(),
                    package_type: "node-npm".to_string(),
                    path: "server".to_string(),
                    version_file: "package.json".to_string(),
                    version_field: "version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: true,
                    workspace_member: false,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes,
            commit_rules: Default::default(),
            workspace: None,
        }
    }

    #[test]
    fn test_detect_from_files_with_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        let detector = ScopeDetector::new(config, temp_dir.path());

        let files = vec![
            PathBuf::from("cli/src/main.rs"),
            PathBuf::from("cli/Cargo.toml"),
        ];

        let scopes = detector.detect_from_files(&files).unwrap();
        assert_eq!(scopes.len(), 1);
        assert!(scopes.contains(&"cli".to_string()));
    }

    #[test]
    fn test_detect_from_files_multiple_scopes() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        let detector = ScopeDetector::new(config, temp_dir.path());

        let files = vec![
            PathBuf::from("cli/src/main.rs"),
            PathBuf::from("server/package.json"),
            PathBuf::from("docs/README.md"),
        ];

        let scopes = detector.detect_from_files(&files).unwrap();
        assert_eq!(scopes.len(), 3);
        assert!(scopes.contains(&"cli".to_string()));
        assert!(scopes.contains(&"server".to_string()));
        assert!(scopes.contains(&"docs".to_string()));
    }

    #[test]
    fn test_detect_from_packages() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create cli package
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

        // Create server package
        let server_dir = temp_dir.path().join("server");
        fs::create_dir(&server_dir).unwrap();
        fs::write(
            server_dir.join("package.json"),
            r#"
{
  "name": "server",
  "version": "1.0.0"
}
        "#,
        )
        .unwrap();

        let detector = ScopeDetector::new(config, temp_dir.path());

        let files = vec![Path::new("cli/src/main.rs"), Path::new("server/index.js")];

        let scopes = detector.detect_from_packages(&files).unwrap();
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&"cli".to_string()));
        assert!(scopes.contains(&"server".to_string()));
    }

    #[test]
    fn test_suggest_scopes() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create packages
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

        let detector = ScopeDetector::new(config, temp_dir.path());
        let scopes = detector.suggest_scopes().unwrap();

        // Should include scopes from config and detected packages
        assert!(scopes.contains(&"cli".to_string()));
        assert!(scopes.contains(&"server".to_string()));
        assert!(scopes.contains(&"docs".to_string()));
    }
}
