// Unified versioning strategy
// All packages share the same version number

use super::manager::{BumpType, VersionManager, VersionUpdate};
use crate::config::repository::RepositoryConfig;
use anyhow::Result;
use semver::Version;
use std::path::Path;

/// Unified versioning strategy
/// All packages share the same version and are bumped together
pub struct UnifiedVersioning {
    config: RepositoryConfig,
    _repo_path: std::path::PathBuf,
}

impl UnifiedVersioning {
    /// Create a new unified versioning strategy
    pub fn new(config: RepositoryConfig, repo_path: &Path) -> Self {
        Self {
            config,
            _repo_path: repo_path.to_path_buf(),
        }
    }
}

impl VersionManager for UnifiedVersioning {
    fn calculate_updates(
        &self,
        _affected_packages: &[String],
        bump_type: BumpType,
    ) -> Result<Vec<VersionUpdate>> {
        let mut updates = Vec::new();

        // Get the unified version from config (Q11: Both - config overrides package)
        let current_version_str =
            self.config
                .versioning
                .unified_version
                .as_ref()
                .ok_or_else(|| {
                    anyhow::anyhow!("Unified versioning requires unified_version in config")
                })?;

        // Parse current version
        let current_version = Version::parse(current_version_str)?;

        // Calculate new version
        let new_version = bump_type.apply(&current_version);

        // Create updates for ALL packages
        for pkg in &self.config.packages {
            updates.push(VersionUpdate::new(
                pkg.name.clone(),
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
    use tempfile::TempDir;

    fn create_test_config(unified_version: &str) -> RepositoryConfig {
        RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::Monorepo,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Unified,
                unified_version: Some(unified_version.to_string()),
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
                    independent: false,
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
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "pkg3".to_string(),
                    package_type: "node-npm".to_string(),
                    path: "pkg3".to_string(),
                    version_file: "package.json".to_string(),
                    version_field: "version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes: Default::default(),
            commit_rules: Default::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        }
    }

    #[test]
    fn test_unified_versioning_minor_bump() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("1.0.0");

        let strategy = UnifiedVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["pkg1".to_string()], BumpType::Minor)
            .unwrap();

        // Should update ALL packages
        assert_eq!(updates.len(), 3);

        for update in &updates {
            assert_eq!(update.old_version, "1.0.0");
            assert_eq!(update.new_version, "1.1.0");
        }
    }

    #[test]
    fn test_unified_versioning_major_bump() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("2.5.3");

        let strategy = UnifiedVersioning::new(config, temp_dir.path());
        let updates = strategy
            .calculate_updates(&["pkg2".to_string()], BumpType::Major)
            .unwrap();

        // Should update ALL packages
        assert_eq!(updates.len(), 3);

        for update in &updates {
            assert_eq!(update.old_version, "2.5.3");
            assert_eq!(update.new_version, "3.0.0");
        }
    }

    #[test]
    fn test_unified_versioning_patch_bump() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config("1.2.3");

        let strategy = UnifiedVersioning::new(config, temp_dir.path());
        let updates = strategy.calculate_updates(&[], BumpType::Patch).unwrap();

        // Should update ALL packages even if no specific packages affected
        assert_eq!(updates.len(), 3);

        for update in &updates {
            assert_eq!(update.old_version, "1.2.3");
            assert_eq!(update.new_version, "1.2.4");
        }
    }

    #[test]
    fn test_unified_versioning_no_version_error() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config("1.0.0");
        config.versioning.unified_version = None;

        let strategy = UnifiedVersioning::new(config, temp_dir.path());
        let result = strategy.calculate_updates(&["pkg1".to_string()], BumpType::Minor);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unified_version"));
    }
}
