use std::collections::HashMap;

use anyhow::Result;
use inquire::Confirm;
use log::{debug, info, warn};
use self_update::update::Release;
use semver::Version;
use serde_json::Value;

use crate::telemetry;

const GITHUB_REPO_OWNER: &str = "martient";
const GITHUB_REPO_NAME: &str = "committy";

#[cfg(target_arch = "aarch64")]
const ASSET_SUFFIX: &str = "macos-arm64";
#[cfg(target_arch = "x86_64")]
const ASSET_SUFFIX: &str = "macos-amd64";

#[cfg_attr(test, mockall::automock)]
pub trait ReleaseProvider {
    fn fetch_releases(&self) -> Result<Vec<Release>>;
}

pub struct GitHubReleaseProvider;

impl ReleaseProvider for GitHubReleaseProvider {
    fn fetch_releases(&self) -> Result<Vec<Release>> {
        Ok(self_update::backends::github::ReleaseList::configure()
            .repo_owner(GITHUB_REPO_OWNER)
            .repo_name(GITHUB_REPO_NAME)
            .build()?
            .fetch()?)
    }
}

pub struct Updater {
    current_version: Version,
    include_prerelease: bool,
    release_provider: Box<dyn ReleaseProvider>,
    non_interactive: bool,
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            current_version: Version::new(0, 0, 0),
            include_prerelease: false,
            release_provider: Box::new(GitHubReleaseProvider),
            non_interactive: false,
        }
    }
}

impl Updater {
    pub fn new(current_version: &str) -> Result<Self> {
        Ok(Self {
            current_version: Version::parse(current_version)?,
            include_prerelease: false,
            release_provider: Box::new(GitHubReleaseProvider),
            non_interactive: false,
        })
    }

    #[cfg(test)]
    pub fn with_provider(mut self, provider: Box<dyn ReleaseProvider>) -> Self {
        self.release_provider = provider;
        self
    }

    pub fn with_prerelease(&mut self, include_prerelease: bool) -> &mut Self {
        self.include_prerelease = include_prerelease;
        self
    }

    pub fn with_non_interactive(&mut self, non_interactive: bool) -> &mut Self {
        self.non_interactive = non_interactive;
        self
    }

    pub fn is_prerelease(version: &str) -> bool {
        version.contains('-')
            || version.contains("alpha")
            || version.contains("beta")
            || version.contains("rc")
    }

    pub fn check_update(&self) -> Result<Option<Release>> {
        let releases = self.release_provider.fetch_releases()?;
        let current_is_prerelease = Self::is_prerelease(&self.current_version.to_string());

        let available_releases: Vec<&Release> = releases
            .iter()
            .filter(|release| {
                let is_release_prerelease = Self::is_prerelease(&release.version);
                // If current version is pre-release, only show pre-releases
                // If current version is stable, only show stable releases unless explicitly including pre-releases
                if current_is_prerelease {
                    is_release_prerelease
                } else {
                    self.include_prerelease || !is_release_prerelease
                }
            })
            .collect();

        if let Some(latest_release) = available_releases.first() {
            let latest_version = Version::parse(&latest_release.version)?;
            if latest_version > self.current_version {
                return Ok(Some((*latest_release).clone()));
            }
        }

        info!("No updates available");
        Ok(None)
    }

    pub fn update_to_version(&self, version_tag: &str) -> Result<()> {
        let version_tag = if !version_tag.starts_with('v') {
            format!("v{}", version_tag)
        } else {
            version_tag.to_string()
        };
        info!("Starting update process for version {}...", version_tag);
        let status = self_update::backends::github::Update::configure()
            .repo_owner(GITHUB_REPO_OWNER)
            .repo_name(GITHUB_REPO_NAME)
            .bin_name("committy")
            .bin_path_in_archive("./committy")
            .target_version_tag(&version_tag)
            .target(&format!("committy-{}.tar.gz", ASSET_SUFFIX))
            .show_download_progress(true)
            .current_version(&self.current_version.to_string())
            .no_confirm(true) // Disable built-in confirmation since we handle it ourselves
            .build()?
            .update()?;

        if status.updated() {
            info!("Update successful! New version: {}", status.version());
        } else {
            warn!("No update available");
        }
        if let Err(e) =
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(telemetry::posthog::publish_event(
                    "update",
                    HashMap::from([
                        ("old_version", Value::from(self.current_version.to_string())),
                        ("new_version", Value::from(version_tag)),
                        ("is_pre_release", Value::from(self.include_prerelease)),
                    ]),
                ))
        {
            debug!("Telemetry error: {:?}", e);
        }

        Ok(())
    }

    fn is_major_or_minor_update(&self, new_version: &Version) -> bool {
        // Don't consider pre-releases as major/minor updates if the user is not on a pre-release
        if Self::is_prerelease(&new_version.to_string()) {
            return false;
        }
        new_version.major > self.current_version.major
            || (new_version.major == self.current_version.major
                && new_version.minor > self.current_version.minor)
    }

    fn should_update(&self, new_version: &Version) -> bool {
        let current_is_pre = Self::is_prerelease(&self.current_version.to_string());
        let new_is_pre = Self::is_prerelease(&new_version.to_string());

        // If user is on pre-release and we're including pre-releases
        if current_is_pre && self.include_prerelease {
            // Only suggest newer pre-releases
            return new_is_pre && new_version > &self.current_version;
        }

        // If user is on pre-release but not including pre-releases
        if current_is_pre && !self.include_prerelease {
            // Only suggest stable versions
            return !new_is_pre && new_version > &self.current_version;
        }

        // If user is on stable and including pre-releases
        if !current_is_pre && self.include_prerelease {
            // Suggest both stable and pre-release updates
            return new_version > &self.current_version;
        }

        // If user is on stable and not including pre-releases
        // Only suggest stable major/minor updates
        !new_is_pre && self.is_major_or_minor_update(new_version)
    }

    pub fn check_and_prompt_update(&mut self) -> Result<Option<Release>> {
        self.include_prerelease = Self::is_prerelease(&self.current_version.to_string());

        if let Ok(Some(release)) = self.check_update() {
            let new_version = Version::parse(&release.version)?;

            if !self.should_update(&new_version) {
                return Ok(None);
            }

            info!(
                "New version {} is available (current version: {})",
                release.version, self.current_version
            );

            // Skip the prompt in non-interactive mode
            if self.non_interactive {
                info!("Skipping update in non-interactive mode");
                return Ok(None);
            }

            let update_type = if self.is_major_or_minor_update(&new_version) {
                "major/minor"
            } else {
                "patch"
            };

            if Confirm::new(&format!(
                "Would you like to update to version {} ({} update)?",
                release.version, update_type
            ))
            .with_default(true)
            .prompt()?
            {
                self.update_to_version(&release.version)?;
                Ok(Some(release))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prerelease_detection() {
        // Test stable versions
        assert!(!Updater::is_prerelease("1.0.0"));
        assert!(!Updater::is_prerelease("2.3.4"));
        assert!(!Updater::is_prerelease("0.1.0"));

        // Test pre-release versions with hyphen
        assert!(Updater::is_prerelease("1.0.0-alpha.1"));
        assert!(Updater::is_prerelease("2.0.0-beta.2"));
        assert!(Updater::is_prerelease("1.0.0-rc.1"));

        // Test pre-release versions without hyphen
        assert!(Updater::is_prerelease("1.0.0alpha1"));
        assert!(Updater::is_prerelease("2.0.0beta2"));
        assert!(Updater::is_prerelease("1.0.0rc1"));
    }

    #[test]
    fn test_updater_creation() {
        // Test valid version
        let result = Updater::new("1.0.0");
        assert!(result.is_ok());
        let updater = result.unwrap();
        assert!(!updater.include_prerelease);

        // Test invalid version
        let result = Updater::new("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_with_prerelease() {
        let mut updater = Updater::new("1.0.0").unwrap();
        assert!(!updater.include_prerelease);

        updater.with_prerelease(true);
        assert!(updater.include_prerelease);

        updater.with_prerelease(false);
        assert!(!updater.include_prerelease);
    }

    #[test]
    fn test_check_update_no_releases() {
        let mut mock_provider = MockReleaseProvider::new();
        mock_provider
            .expect_fetch_releases()
            .times(1)
            .returning(|| Ok(vec![]));

        let updater = Updater::new("1.0.0")
            .unwrap()
            .with_provider(Box::new(mock_provider));

        let result = updater.check_update();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_check_update_with_newer_version() {
        let mut mock_provider = MockReleaseProvider::new();
        let newer_release = Release {
            version: "2.0.0".to_string(),
            name: "v2.0.0".to_string(),
            body: Some("Release notes".to_string()),
            date: "2024-01-07".to_string(),
            assets: vec![],
        };

        mock_provider
            .expect_fetch_releases()
            .times(1)
            .returning(move || Ok(vec![newer_release.clone()]));

        let updater = Updater::new("1.0.0")
            .unwrap()
            .with_provider(Box::new(mock_provider));

        let result = updater.check_update();
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_check_update_with_older_version() {
        let mut mock_provider = MockReleaseProvider::new();
        let older_release = Release {
            version: "1.0.0".to_string(),
            name: "v1.0.0".to_string(),
            body: Some("Release notes".to_string()),
            date: "2024-01-07".to_string(),
            assets: vec![],
        };

        mock_provider
            .expect_fetch_releases()
            .times(1)
            .returning(move || Ok(vec![older_release.clone()]));

        let updater = Updater::new("2.0.0")
            .unwrap()
            .with_provider(Box::new(mock_provider));

        let result = updater.check_update();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_check_update_prerelease_handling() {
        let mut mock_provider = MockReleaseProvider::new();
        let releases = vec![
            Release {
                version: "2.0.0-beta.1".to_string(),
                name: "v2.0.0-beta.1".to_string(),
                body: Some("Beta release".to_string()),
                date: "2024-01-07".to_string(),
                assets: vec![],
            },
            Release {
                version: "1.0.0".to_string(),
                name: "v1.0.0".to_string(),
                body: Some("Stable release".to_string()),
                date: "2024-01-07".to_string(),
                assets: vec![],
            },
        ];

        mock_provider
            .expect_fetch_releases()
            .times(2)
            .returning(move || Ok(releases.clone()));

        let mut updater = Updater::new("1.0.0")
            .unwrap()
            .with_provider(Box::new(mock_provider));

        // Without pre-release flag
        updater.with_prerelease(false);
        let result = updater.check_update();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // With pre-release flag
        updater.with_prerelease(true);
        let result = updater.check_update();
        assert!(result.is_ok());
        let release = result.unwrap().unwrap();
        assert_eq!(release.version, "2.0.0-beta.1");
    }

    #[test]
    fn test_version_parsing() {
        // Test valid versions
        assert!(Version::parse("1.0.0").is_ok());
        assert!(Version::parse("1.0.0-alpha.1").is_ok());
        assert!(Version::parse("1.0.0-beta.2").is_ok());
        assert!(Version::parse("1.0.0-rc.1").is_ok());

        // Test invalid versions
        assert!(Version::parse("invalid").is_err());
        assert!(Version::parse("1.0").is_err());
        assert!(Version::parse("1").is_err());
    }
}
