// Hybrid versioning strategy
// Primary package drives version, others can sync or stay independent

use super::manager::{BumpType, VersionManager, VersionUpdate};
use crate::config::repository::RepositoryConfig;
use crate::packages::MultiPackageDetector;
use anyhow::{Context, Result};
use semver::Version;
use std::collections::HashSet;
use std::path::Path;

/// Hybrid versioning strategy
/// Primary package(s) drive the version, other packages can sync with them or stay independent
pub struct HybridVersioning {
    config: RepositoryConfig,
    repo_path: std::path::PathBuf,
}

impl HybridVersioning {
    /// Create a new hybrid versioning strategy
    pub fn new(config: RepositoryConfig, repo_path: &Path) -> Self {
        Self {
            config,
            repo_path: repo_path.to_path_buf(),
        }
    }

    /// Get all packages that should be updated when a package is bumped
    fn get_synced_packages(&self, package_name: &str) -> Vec<String> {
        let mut synced = vec![package_name.to_string()];

        // Find all packages that sync with this package
        for pkg in &self.config.packages {
            if let Some(ref sync_with) = pkg.sync_with {
                if sync_with == package_name {
                    synced.push(pkg.name.clone());
                    // Recursively find packages that sync with this one
                    synced.extend(self.get_synced_packages(&pkg.name));
                }
            }
        }

        synced
    }
}

impl VersionManager for HybridVersioning {
    fn calculate_updates(
        &self,
        affected_packages: &[String],
        bump_type: BumpType,
    ) -> Result<Vec<VersionUpdate>> {
        let mut updates = Vec::new();
        let mut processed = HashSet::new();

        // Detect current packages to get their versions
        let detector = MultiPackageDetector::new();
        let detected_packages = detector.detect_all(&self.repo_path)?;

        // Determine which packages to update
        let mut packages_to_update = HashSet::new();

        for pkg_name in affected_packages {
            // Find the package in config
            let cfg_pkg = self
                .config
                .packages
                .iter()
                .find(|p| &p.name == pkg_name)
                .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in config", pkg_name))?;

            if cfg_pkg.independent {
                // Independent packages are updated on their own
                packages_to_update.insert(pkg_name.clone());
            } else if cfg_pkg.primary {
                // Primary package: update it and all packages that sync with it
                for synced_pkg in self.get_synced_packages(pkg_name) {
                    packages_to_update.insert(synced_pkg);
                }
            } else if let Some(ref sync_with) = cfg_pkg.sync_with {
                // Package syncs with another: update the sync target and all its synced packages
                for synced_pkg in self.get_synced_packages(sync_with) {
                    packages_to_update.insert(synced_pkg);
                }
            } else {
                // Regular package (not primary, not synced, not independent)
                packages_to_update.insert(pkg_name.clone());
            }
        }

        // Calculate updates for each package to update
        for pkg_name in packages_to_update {
            if processed.contains(&pkg_name) {
                continue;
            }
            processed.insert(pkg_name.clone());

            // Find the package in config
            let cfg_pkg = self
                .config
                .packages
                .iter()
                .find(|p| p.name == pkg_name)
                .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in config", pkg_name))?;

            // Find the detected package to get current version
            let detected_pkg = detected_packages
                .iter()
                .find(|p| p.path == std::path::PathBuf::from(&cfg_pkg.path))
                .ok_or_else(|| {
                    anyhow::anyhow!("Package '{}' not found at {}", pkg_name, cfg_pkg.path)
                })?;

            // Parse current version
            let current_version = Version::parse(&detected_pkg.version).with_context(|| {
                format!(
                    "Invalid version '{}' for package '{}'",
                    detected_pkg.version, pkg_name
                )
            })?;

            // Calculate new version
            let new_version = bump_type.apply(&current_version);

            updates.push(VersionUpdate::new(
                pkg_name.clone(),
                current_version.to_string(),
                new_version.to_string(),
            ));
        }

        Ok(updates)
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
        RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Hybrid,
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
                    primary: true,
                    sync_with: None,
                    independent: false,
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
                    sync_with: Some("cli".to_string()),
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "docs".to_string(),
                    package_type: "node-npm".to_string(),
                    path: "docs".to_string(),
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
            scopes: Default::default(),
            commit_rules: Default::default(),
            workspace: None,
        }
    }

    #[test]
    fn test_hybrid_versioning_primary_package() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create cli (primary)
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

        // Create server (syncs with cli)
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

        // Create docs (independent)
        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir(&docs_dir).unwrap();
        fs::write(
            docs_dir.join("package.json"),
            r#"
{
  "name": "docs",
  "version": "2.0.0"
}
        "#,
        )
        .unwrap();

        let strategy = HybridVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["cli".to_string()], BumpType::Minor)
            .unwrap();

        // Should update cli and server (synced), but NOT docs (independent)
        assert_eq!(updates.len(), 2);

        let cli_update = updates.iter().find(|u| u.package_name == "cli").unwrap();
        assert_eq!(cli_update.old_version, "1.0.0");
        assert_eq!(cli_update.new_version, "1.1.0");

        let server_update = updates.iter().find(|u| u.package_name == "server").unwrap();
        assert_eq!(server_update.old_version, "1.0.0");
        assert_eq!(server_update.new_version, "1.1.0");

        // docs should NOT be in updates
        assert!(updates.iter().all(|u| u.package_name != "docs"));
    }

    #[test]
    fn test_hybrid_versioning_independent_package() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create all packages
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

        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir(&docs_dir).unwrap();
        fs::write(
            docs_dir.join("package.json"),
            r#"
{
  "name": "docs",
  "version": "2.0.0"
}
        "#,
        )
        .unwrap();

        let strategy = HybridVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["docs".to_string()], BumpType::Patch)
            .unwrap();

        // Should only update docs (independent)
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package_name, "docs");
        assert_eq!(updates[0].old_version, "2.0.0");
        assert_eq!(updates[0].new_version, "2.0.1");
    }

    #[test]
    fn test_hybrid_versioning_synced_package() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create all packages
        let cli_dir = temp_dir.path().join("cli");
        fs::create_dir(&cli_dir).unwrap();
        fs::write(
            cli_dir.join("Cargo.toml"),
            r#"
[package]
name = "cli"
version = "1.5.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let server_dir = temp_dir.path().join("server");
        fs::create_dir(&server_dir).unwrap();
        fs::write(
            server_dir.join("package.json"),
            r#"
{
  "name": "server",
  "version": "1.5.0"
}
        "#,
        )
        .unwrap();

        let docs_dir = temp_dir.path().join("docs");
        fs::create_dir(&docs_dir).unwrap();
        fs::write(
            docs_dir.join("package.json"),
            r#"
{
  "name": "docs",
  "version": "2.0.0"
}
        "#,
        )
        .unwrap();

        let strategy = HybridVersioning::new(config, temp_dir.path());
        // When server is affected, it should update cli (its sync target) and server
        let updates = strategy
            .calculate_updates(&["server".to_string()], BumpType::Major)
            .unwrap();

        // Should update both cli and server
        assert_eq!(updates.len(), 2);

        let cli_update = updates.iter().find(|u| u.package_name == "cli").unwrap();
        assert_eq!(cli_update.old_version, "1.5.0");
        assert_eq!(cli_update.new_version, "2.0.0");

        let server_update = updates.iter().find(|u| u.package_name == "server").unwrap();
        assert_eq!(server_update.old_version, "1.5.0");
        assert_eq!(server_update.new_version, "2.0.0");
    }
}
