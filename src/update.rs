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
}

impl Updater {
    pub fn new(current_version: &str) -> Result<Self> {
        Ok(Self {
            current_version: Version::parse(current_version)?,
        })
    }

    pub async fn check_update(&self) -> Result<Option<Release>> {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner(GITHUB_REPO_OWNER)
            .repo_name(GITHUB_REPO_NAME)
            .build()?
            .fetch()?;

        if let Some(latest_release) = releases.first() {
            let latest_version = Version::parse(&latest_release.version)?;
            if latest_version > self.current_version {
                info!("New version {} available", latest_version);
                return Ok(Some(latest_release.clone()));
            }
        }

        info!("No updates available");
        Ok(None)
    }

    pub fn update_to_latest(&self) -> Result<()> {
        info!("Starting update process...");
        let status = self_update::backends::github::Update::configure()
            .repo_owner(GITHUB_REPO_OWNER)
            .repo_name(GITHUB_REPO_NAME)
            .bin_name("committy")
            .target_version_tag("latest")
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
}
