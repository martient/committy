use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use log::{debug, info};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;


pub const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "build", "chore", "ci", "cd", "docs", "perf", "refactor", "revert", "style",
    "test", "security", "config",
];

pub const BRANCH_TYPES: &[&str] = &[
    "feat", "fix", "refactor", "test", "docs", "perf", "security", "hotfix", "release", "spike",
    "tooling",
];

pub const MAX_SHORT_DESCRIPTION_LENGTH: usize = 150;
pub const MAX_TICKET_NAME_LENGTH: usize = 10;
pub const MAX_SCOPE_NAME_LENGTH: usize = 15;

pub const MAJOR_REGEX: &str = r"(?im)^(breaking change:|feat(?:\s*\([^)]*\))?!:)";
pub const MINOR_REGEX: &str = r"(?im)^feat(?:\s*\([^)]*\))?:";
pub const PATCH_REGEX: &str = r"(?im)^(fix|docs|style|refactor|perf|test|chore|ci|cd|build|revert|security|config)(?:\s*\([^)]*\))?:";

pub const CHANGELOG_TEMPLATE: &str = r"";
lazy_static! {
    pub static ref CHANGELOG_CATEGORY_TEMPLATE: Vec<ChangelogCategory> = vec![
        ChangelogCategory {
            name: "Features".to_string(),
            types: vec!["feat".to_string()],
        },
        ChangelogCategory {
            name: "Fixes".to_string(),
            types: vec!["fix".to_string()],
        },
        ChangelogCategory {
            name: "Maintenance".to_string(),
            types: vec![
                "build".to_string(),
                "chore".to_string(),
                "ci".to_string(),
                "cd".to_string(),
                "docs".to_string(),
                "perf".to_string(),
                "refactor".to_string(),
                "revert".to_string(),
                "style".to_string(),
                "test".to_string(),
                "security".to_string(),
            ],
        },
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangelogCategory {
    pub name: String,
    pub types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub last_update_check: DateTime<FixedOffset>,
    pub metrics_enabled: bool,
    pub last_metrics_reminder: DateTime<FixedOffset>,
    pub user_id: String,
    pub changelog: ChangelogConfig,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ChangelogConfig {
    pub template: String,
    pub categories: Vec<ChangelogCategory>,
}

impl Default for Config {
    fn default() -> Self {
        debug!("Creating default configuration");
        Self {
            last_update_check: DateTime::parse_from_rfc3339("2006-01-01T00:00:00+01:00").unwrap(),
            metrics_enabled: true,
            last_metrics_reminder: DateTime::parse_from_rfc3339("2006-01-01T00:00:00+01:00")
                .unwrap(),
            user_id: "".to_string(),
            changelog: ChangelogConfig {
                template: CHANGELOG_TEMPLATE.to_string(),
                categories: CHANGELOG_CATEGORY_TEMPLATE.clone(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        debug!("Loading configuration from {:?}", config_path);

        if !config_path.exists() {
            debug!("No configuration file found, using defaults");
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path)?;
        debug!("Read configuration content: {}", config_str);
        // Load config with possible missing fields (serde default fills them)
        let mut config: Self = toml::from_str(&config_str)?;
        // If any field is still default (i.e., was missing in the file), re-save
        let mut needs_save = false;
        if config.user_id.is_empty() {
            debug!("User ID is missing, generating new UUID");
            config.user_id = Uuid::new_v4().to_string();
            needs_save = true;
        }
        if needs_save {
            debug!("Config missing new fields, saving updated config to disk");
            let _ = config.save();
        }
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        debug!("Saving configuration");
        debug!("Configuration path: {:?}", config_path);
        debug!("Saving configuration to {:?}", config_path);

        if let Some(parent) = config_path.parent() {
            debug!("Creating config directory: {:?}", parent);
            fs::create_dir_all(parent)?;
        }

        let config_str = toml::to_string(self)?;
        debug!("Writing configuration content: {}", config_str);
        fs::write(config_path, config_str)?;
        debug!("Configuration saved successfully");
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let path = home.join(".config").join("committy").join("config.toml");
        debug!("Config path resolved to: {:?}", path);
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::Builder;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn setup_test_env() -> (TempDir, Config) {
        let uuid = Uuid::new_v4();
        let temp_dir = Builder::new().prefix(&uuid.to_string()).tempdir().unwrap();

        // let temp_dir = TempDir::new(Uuid::new_v4().to_string()).unwrap();
        env::set_var("HOME", temp_dir.path());

        let config = Config {
            last_update_check: DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00").unwrap(),
            metrics_enabled: true,
            last_metrics_reminder: DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00")
                .unwrap(),
            user_id: Uuid::new_v4().to_string(),
            changelog: ChangelogConfig {
                template: String::new(),
                categories: Vec::new(),
            },
        };

        (temp_dir, config)
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.metrics_enabled);
        assert_eq!(
            config.last_update_check.to_rfc3339(),
            "2006-01-01T00:00:00+01:00"
        );
        assert_eq!(
            config.last_metrics_reminder.to_rfc3339(),
            "2006-01-01T00:00:00+01:00"
        );
    }

    #[test]
    fn test_save_and_load_config() {
        let (_temp_dir, config) = setup_test_env();

        // Save the config
        config.save().expect("Failed to save config");

        // Load the config and verify it matches the original
        let loaded_config = Config::load().expect("Failed to load config");
        assert_eq!(
            config.changelog, loaded_config.changelog,
            "Loaded config should match saved config"
        );
    }

    #[test]
    fn test_load_nonexistent_config() {
        let (temp_dir, _) = setup_test_env();
        let config_dir = temp_dir.path().join(".config").join("committy");

        // Ensure config directory is empty
        if config_dir.exists() {
            fs::remove_dir_all(&config_dir).unwrap();
        }

        // Loading non-existent config should return default
        let config = Config::load().expect("Failed to load default config");
        assert!(config.metrics_enabled);

        assert_eq!(
            config.last_update_check.to_rfc3339(),
            "2006-01-01T00:00:00+01:00"
        );
        assert_eq!(
            config.last_metrics_reminder.to_rfc3339(),
            "2006-01-01T00:00:00+01:00"
        );
    }

    #[test]
    fn test_config_directory_creation() {
        let (temp_dir, config) = setup_test_env();
        let config_dir = temp_dir.path().join(".config").join("committy");

        // Ensure directory doesn't exist initially
        if config_dir.exists() {
            fs::remove_dir_all(&config_dir).unwrap();
        }
        assert!(!config_dir.exists());

        // Save should create the directory and file
        config.save().expect("Failed to save config");

        assert!(config_dir.exists(), "Config directory should exist");
        assert!(
            config_dir.join("config.toml").exists(),
            "Config file should exist"
        );

        // Verify the saved content
        let content =
            fs::read_to_string(config_dir.join("config.toml")).expect("Failed to read config file");
        assert!(!content.is_empty(), "Config file should not be empty");
    }

    #[test]
    fn test_update_config_values() {
        let (_temp_dir, mut config) = setup_test_env();

        // Save initial config
        config.save().expect("Failed to save config");

        // Update values
        config.metrics_enabled = false;
        config.last_update_check =
            DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00").unwrap();
        config.last_metrics_reminder =
            DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00").unwrap();

        // Save updated config
        config.save().expect("Failed to save updated config");

        // Load and verify
        let loaded_config = Config::load().expect("Failed to load config");
        assert_eq!(
            config, loaded_config,
            "Loaded config should match updated config"
        );
    }
}
