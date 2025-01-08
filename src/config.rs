pub const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "build", "chore", "ci", "cd", "docs", "perf", "refactor", "revert", "style",
    "test", "security",
];

pub const MAX_SHORT_DESCRIPTION_LENGTH: usize = 150;

pub const MAJOR_REGEX: &str = r"(?im)^(breaking change:|feat(?:\s*\([^)]*\))?!:)";
pub const MINOR_REGEX: &str = r"(?im)^feat(?:\s*\([^)]*\))?:";
pub const PATCH_REGEX: &str = r"(?im)^(fix|docs|style|refactor|perf|test|chore|ci|cd|build|revert|security)(?:\s*\([^)]*\))?:";

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub last_update_check: DateTime<FixedOffset>,
    pub metrics_enabled: bool,
    pub last_metrics_reminder: DateTime<FixedOffset>,
}

impl Default for Config {
    fn default() -> Self {
        debug!("Creating default configuration");
        Self {
            last_update_check: DateTime::parse_from_rfc3339("2006-01-01T00:00:00+01:00").unwrap(),
            metrics_enabled: true,
            last_metrics_reminder: DateTime::parse_from_rfc3339("2006-01-01T00:00:00+01:00")
                .unwrap(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        debug!("Loading configuration from {:?}", config_path);

        if !config_path.exists() {
            info!("No configuration file found, using defaults");
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path)?;
        debug!("Read configuration content: {}", config_str);
        let config: Self = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        debug!("Saving configuration to {:?}", config_path);

        if let Some(parent) = config_path.parent() {
            debug!("Creating config directory: {:?}", parent);
            fs::create_dir_all(parent)?;
        }

        let config_str = toml::to_string(self)?;
        debug!("Writing configuration content: {}", config_str);
        fs::write(config_path, config_str)?;
        info!("Configuration saved successfully");
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
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, Config) {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("HOME", temp_dir.path());

        let config = Config {
            last_update_check: DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00").unwrap(),
            metrics_enabled: true,
            last_metrics_reminder: DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00")
                .unwrap(),
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
            config, loaded_config,
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
