use anyhow::Result;
use log::{info, warn};
use self_update::update::Release;
use semver::Version;

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
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            current_version: Version::new(0, 0, 0),
            include_prerelease: false,
            release_provider: Box::new(GitHubReleaseProvider),
        }
    }
}

impl Updater {
    pub fn new(current_version: &str) -> Result<Self> {
        Ok(Self {
            current_version: Version::parse(current_version)?,
            include_prerelease: false,
            release_provider: Box::new(GitHubReleaseProvider),
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

    pub fn is_prerelease(version: &str) -> bool {
        version.contains('-')
            || version.contains("alpha")
            || version.contains("beta")
            || version.contains("rc")
    }

    pub async fn check_update(&self) -> Result<Option<Release>> {
        let releases = self.release_provider.fetch_releases()?;

        let available_releases: Vec<&Release> = releases
            .iter()
            .filter(|release| self.include_prerelease || !Self::is_prerelease(&release.version))
            .collect();

        if let Some(latest_release) = available_releases.first() {
            let latest_version = Version::parse(&latest_release.version)?;
            if latest_version > self.current_version {
                info!(
                    "New version {} available{}",
                    latest_version,
                    if Self::is_prerelease(&latest_release.version) {
                        " (pre-release)"
                    } else {
                        ""
                    }
                );
                return Ok(Some((*latest_release).clone()));
            }
        }

        info!("No updates available");
        Ok(None)
    }

    pub fn update_to_version(&self, version_tag: &str) -> Result<()> {
        info!("Starting update process to version {}...", version_tag);
        let status = self_update::backends::github::Update::configure()
            .repo_owner(GITHUB_REPO_OWNER)
            .repo_name(GITHUB_REPO_NAME)
            .bin_name("committy")
            .target_version_tag(version_tag)
            .target(&format!("committy-{}.tar.gz", ASSET_SUFFIX))
            .show_download_progress(true)
            .current_version(&self.current_version.to_string())
            .build()?
            .update()?;

        if status.updated() {
            info!("Update successful! New version: {}", status.version());
        } else {
            warn!("No update available");
        }

        Ok(())
    }

    pub fn update_to_latest(&self) -> Result<()> {
        self.update_to_version("latest")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(updater.check_update());
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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(updater.check_update());
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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(updater.check_update());
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

        let rt = tokio::runtime::Runtime::new().unwrap();

        // Without pre-release flag
        updater.with_prerelease(false);
        let result = rt.block_on(updater.check_update());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // With pre-release flag
        updater.with_prerelease(true);
        let result = rt.block_on(updater.check_update());
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
