// Configuration hierarchy: repository config > user config > defaults
// Q4: Repository config always wins

use super::repository::RepositoryConfig;
use super::Config as UserConfig;
use anyhow::Result;
use std::path::Path;

/// Merged configuration from repository and user configs
pub struct MergedConfig {
    /// Repository-level config (from .committy/config.toml)
    pub repository: Option<RepositoryConfig>,
    /// User-level config (from ~/.config/committy/config.toml)
    pub user: UserConfig,
}

impl MergedConfig {
    /// Load and merge configurations
    /// Priority: repository > user > defaults
    pub fn load(repo_path: &Path) -> Result<Self> {
        let repository = RepositoryConfig::try_load(repo_path)?;
        let user = UserConfig::load()?;

        Ok(Self { repository, user })
    }

    /// Check if multi-package mode is enabled
    pub fn is_multi_package(&self) -> bool {
        self.repository
            .as_ref()
            .map(|r| r.is_multi_package())
            .unwrap_or(false)
    }

    /// Get major version bump regex pattern
    /// Repository config takes precedence over user config
    pub fn get_major_regex(&self) -> &str {
        self.repository
            .as_ref()
            .and_then(|r| r.versioning.rules.as_ref())
            .and_then(|rules| rules.major_regex.as_deref())
            .unwrap_or(&self.user.major_regex)
    }

    /// Get minor version bump regex pattern
    /// Repository config takes precedence over user config
    pub fn get_minor_regex(&self) -> &str {
        self.repository
            .as_ref()
            .and_then(|r| r.versioning.rules.as_ref())
            .and_then(|rules| rules.minor_regex.as_deref())
            .unwrap_or(&self.user.minor_regex)
    }

    /// Get patch version bump regex pattern
    /// Repository config takes precedence over user config
    pub fn get_patch_regex(&self) -> &str {
        self.repository
            .as_ref()
            .and_then(|r| r.versioning.rules.as_ref())
            .and_then(|rules| rules.patch_regex.as_deref())
            .unwrap_or(&self.user.patch_regex)
    }

    /// Get repository config if it exists
    pub fn repository_config(&self) -> Option<&RepositoryConfig> {
        self.repository.as_ref()
    }

    /// Get user config
    pub fn user_config(&self) -> &UserConfig {
        &self.user
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::repository::{
        RepositoryMetadata, RepositoryType, VersioningConfig, VersioningRules, VersioningStrategy,
    };
    use tempfile::TempDir;

    #[test]
    fn test_merged_config_without_repository() {
        let temp_dir = TempDir::new().unwrap();
        let merged = MergedConfig::load(temp_dir.path()).unwrap();

        assert!(merged.repository.is_none());
        assert!(!merged.is_multi_package());
    }

    #[test]
    fn test_regex_fallback_to_user_config() {
        let temp_dir = TempDir::new().unwrap();
        let merged = MergedConfig::load(temp_dir.path()).unwrap();

        // Should use user config patterns
        assert!(!merged.get_major_regex().is_empty());
        assert!(!merged.get_minor_regex().is_empty());
        assert!(!merged.get_patch_regex().is_empty());
    }

    #[test]
    fn test_repository_config_overrides_user() {
        let temp_dir = TempDir::new().unwrap();

        // Create a repository config with custom regex
        let repo_config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: Some(VersioningRules {
                    major_regex: Some("custom_major".to_string()),
                    minor_regex: Some("custom_minor".to_string()),
                    patch_regex: Some("custom_patch".to_string()),
                }),
            },
            packages: vec![],
            dependencies: vec![],
            scopes: Default::default(),
            commit_rules: Default::default(),
            workspace: None,
        };

        // Save it
        repo_config.save(temp_dir.path()).unwrap();

        // Load merged config
        let merged = MergedConfig::load(temp_dir.path()).unwrap();

        // Should use repository config patterns
        assert_eq!(merged.get_major_regex(), "custom_major");
        assert_eq!(merged.get_minor_regex(), "custom_minor");
        assert_eq!(merged.get_patch_regex(), "custom_patch");
    }
}
