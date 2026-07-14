// Independent versioning strategy
// Each package maintains its own version independently

use super::manager::{BumpType, VersionManager, VersionUpdate};
use crate::config::repository::RepositoryConfig;
use crate::packages::MultiPackageDetector;
use anyhow::{Context, Result};
use semver::Version;
use std::path::Path;

/// Independent versioning strategy
/// Each package has its own version that is bumped independently
pub struct IndependentVersioning {
    config: RepositoryConfig,
    repo_path: std::path::PathBuf,
}

impl IndependentVersioning {
    /// Create a new independent versioning strategy
    pub fn new(config: RepositoryConfig, repo_path: &Path) -> Self {
        Self {
            config,
            repo_path: repo_path.to_path_buf(),
        }
    }
}

impl VersionManager for IndependentVersioning {
    fn calculate_updates(
        &self,
        affected_packages: &[String],
        bump_type: BumpType,
    ) -> Result<Vec<VersionUpdate>> {
        let mut updates = Vec::new();

        // Detect current packages to get their versions
        let detector = MultiPackageDetector::new();
        let detected_packages = detector.detect_all(&self.repo_path)?;

        // For each affected package, calculate the new version
        for pkg_name in affected_packages {
            // Find the package in config
            let cfg_pkg = self
                .config
                .packages
                .iter()
                .find(|p| &p.name == pkg_name)
                .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in config", pkg_name))?;

            // Find the detected package to get current version
            let detected_pkg = detected_packages
                .iter()
                .find(|p| p.path == std::path::Path::new(&cfg_pkg.path))
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
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "pkg1".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "pkg1".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: true,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "pkg2".to_string(),
                    package_type: "node-npm".to_string(),
                    path: "pkg2".to_string(),
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
            branch_rules: Default::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        }
    }

    #[test]
    fn test_independent_versioning_single_package() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create pkg1
        let pkg1_dir = temp_dir.path().join("pkg1");
        fs::create_dir(&pkg1_dir).unwrap();
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"
[package]
name = "pkg1"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let strategy = IndependentVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["pkg1".to_string()], BumpType::Minor)
            .unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package_name, "pkg1");
        assert_eq!(updates[0].old_version, "1.0.0");
        assert_eq!(updates[0].new_version, "1.1.0");
    }

    #[test]
    fn test_independent_versioning_multiple_packages() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create pkg1
        let pkg1_dir = temp_dir.path().join("pkg1");
        fs::create_dir(&pkg1_dir).unwrap();
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"
[package]
name = "pkg1"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        // Create pkg2
        let pkg2_dir = temp_dir.path().join("pkg2");
        fs::create_dir(&pkg2_dir).unwrap();
        fs::write(
            pkg2_dir.join("package.json"),
            r#"
{
  "name": "pkg2",
  "version": "2.5.0"
}
        "#,
        )
        .unwrap();

        let strategy = IndependentVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["pkg1".to_string(), "pkg2".to_string()], BumpType::Patch)
            .unwrap();

        assert_eq!(updates.len(), 2);

        let pkg1_update = updates.iter().find(|u| u.package_name == "pkg1").unwrap();
        assert_eq!(pkg1_update.old_version, "1.0.0");
        assert_eq!(pkg1_update.new_version, "1.0.1");

        let pkg2_update = updates.iter().find(|u| u.package_name == "pkg2").unwrap();
        assert_eq!(pkg2_update.old_version, "2.5.0");
        assert_eq!(pkg2_update.new_version, "2.5.1");
    }

    #[test]
    fn test_independent_versioning_major_bump() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create pkg1
        let pkg1_dir = temp_dir.path().join("pkg1");
        fs::create_dir(&pkg1_dir).unwrap();
        fs::write(
            pkg1_dir.join("Cargo.toml"),
            r#"
[package]
name = "pkg1"
version = "1.5.3"
edition = "2021"
        "#,
        )
        .unwrap();

        let strategy = IndependentVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["pkg1".to_string()], BumpType::Major)
            .unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].old_version, "1.5.3");
        assert_eq!(updates[0].new_version, "2.0.0");
    }
}
