// Configuration hierarchy: repository config > user config > defaults
// Q4: Repository config always wins

use super::Config as UserConfig;
use super::{
    changelog::ChangelogConfig,
    convention::ConventionConfig,
    release::ReleaseConfig,
    repository::{CommitRulesConfig, CustomCommitType, RepositoryConfig},
};
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

    /// Get effective git config overrides in precedence order before CLI overrides
    pub fn get_git_config_overrides(&self) -> Vec<String> {
        let mut overrides = self.user.git.config_overrides.clone();
        if let Some(repository) = &self.repository {
            overrides.extend(repository.git.config_overrides.iter().cloned());
        }
        overrides
    }

    pub fn effective_convention(&self) -> ConventionConfig {
        let mut config = self
            .user
            .convention
            .clone()
            .unwrap_or_else(ConventionConfig::default);

        if let Some(repository) = &self.repository {
            if let Some(repository_convention) = &repository.convention {
                config = repository_convention.clone();
            }
            apply_legacy_commit_rules(&mut config, &repository.commit_rules);
        }

        config
    }

    pub fn effective_release(&self) -> ReleaseConfig {
        let mut config = self.user.release.clone().unwrap_or_default();
        if let Some(repository) = &self.repository {
            if let Some(repository_release) = &repository.release {
                config = repository_release.clone();
            }
        }
        config
    }

    pub fn effective_changelog(&self) -> ChangelogConfig {
        let mut config = self.user.changelog.clone().unwrap_or_default();
        if let Some(repository) = &self.repository {
            if let Some(repository_changelog) = &repository.changelog {
                config = repository_changelog.clone();
            }
        }
        config
    }

    /// Get user config
    pub fn user_config(&self) -> &UserConfig {
        &self.user
    }
}

fn apply_legacy_commit_rules(config: &mut ConventionConfig, rules: &CommitRulesConfig) {
    config.max_subject_length = rules.max_subject_length;
    config.max_body_line_length = rules.max_body_line_length;
    config.require_body = rules.require_body;

    if !rules.allowed_types.is_empty() {
        config
            .types
            .retain(|item| rules.allowed_types.contains(&item.name));
    }

    for custom in &rules.custom_types {
        if let Some(existing) = config
            .types
            .iter_mut()
            .find(|item| item.name == custom.name)
        {
            existing.description = custom.description.clone();
            existing.bump = custom.bump.clone();
        } else {
            config.types.push(custom_commit_type_to_convention(custom));
        }
    }

    if let Some(type_question) = config.questions.iter_mut().find(|item| item.key == "type") {
        type_question.choices = config.types.iter().map(|item| item.name.clone()).collect();
    }
}

fn custom_commit_type_to_convention(
    custom: &CustomCommitType,
) -> super::convention::ConventionType {
    super::convention::ConventionType {
        name: custom.name.clone(),
        description: custom.description.clone(),
        bump: custom.bump.clone(),
        changelog_section: "Custom".to_string(),
        aliases: vec![],
        hidden: false,
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
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
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
