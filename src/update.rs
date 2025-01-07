use anyhow::Result;
use log::{info, warn};
use semver::Version;
use self_update::update::Release;

const GITHUB_REPO_OWNER: &str = "martient";
const GITHUB_REPO_NAME: &str = "committy";

#[cfg(target_arch = "aarch64")]
const ASSET_SUFFIX: &str = "macos-arm64";
#[cfg(target_arch = "x86_64")]
const ASSET_SUFFIX: &str = "macos-amd64";

pub struct Updater {
    current_version: Version,
    include_prerelease: bool,
}

impl Updater {
    pub fn new(current_version: &str) -> Result<Self> {
        Ok(Self {
            current_version: Version::parse(current_version)?,
            include_prerelease: false,
        })
    }

    pub fn with_prerelease(mut self, include_prerelease: bool) -> Self {
        self.include_prerelease = include_prerelease;
        self
    }

    pub fn is_prerelease(version: &str) -> bool {
        // Check if version contains pre-release indicators
        version.contains('-') || version.contains("alpha") || 
        version.contains("beta") || version.contains("rc")
    }

    pub async fn check_update(&self) -> Result<Option<Release>> {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner(GITHUB_REPO_OWNER)
            .repo_name(GITHUB_REPO_NAME)
            .build()?
            .fetch()?;

        let available_releases: Vec<&Release> = releases
            .iter()
            .filter(|release| self.include_prerelease || !Self::is_prerelease(&release.version))
            .collect();

        if let Some(latest_release) = available_releases.first() {
            let latest_version = Version::parse(&latest_release.version)?;
            if latest_version > self.current_version {
                info!("New version {} available{}", latest_version, 
                    if Self::is_prerelease(&latest_release.version) { " (pre-release)" } else { "" });
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
            info!(
                "Update successful! New version: {}",
                status.version()
            );
        } else {
            warn!("No update available");
        }

        Ok(())
    }

    pub fn update_to_latest(&self) -> Result<()> {
        self.update_to_version("latest")
    }
}
