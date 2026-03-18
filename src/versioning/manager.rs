// Version manager trait and types

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Type of version bump
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}

impl BumpType {
    /// Apply the bump to a version
    pub fn apply(&self, version: &Version) -> Version {
        match self {
            BumpType::Major => Version::new(version.major + 1, 0, 0),
            BumpType::Minor => Version::new(version.major, version.minor + 1, 0),
            BumpType::Patch => Version::new(version.major, version.minor, version.patch + 1),
        }
    }
}

/// Information about a version update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionUpdate {
    /// Package name
    pub package_name: String,
    /// Old version
    pub old_version: String,
    /// New version
    pub new_version: String,
}

impl VersionUpdate {
    /// Create a new version update
    pub fn new(package_name: String, old_version: String, new_version: String) -> Self {
        Self {
            package_name,
            old_version,
            new_version,
        }
    }
}

/// Trait for version management strategies
pub trait VersionManager {
    /// Calculate version updates for affected packages
    ///
    /// # Arguments
    /// * `affected_packages` - Names of packages affected by the change
    /// * `bump_type` - Type of version bump (major, minor, patch)
    ///
    /// # Returns
    /// List of version updates to apply
    fn calculate_updates(
        &self,
        affected_packages: &[String],
        bump_type: BumpType,
    ) -> Result<Vec<VersionUpdate>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_type_major() {
        let version = Version::new(1, 2, 3);
        let bumped = BumpType::Major.apply(&version);
        assert_eq!(bumped, Version::new(2, 0, 0));
    }

    #[test]
    fn test_bump_type_minor() {
        let version = Version::new(1, 2, 3);
        let bumped = BumpType::Minor.apply(&version);
        assert_eq!(bumped, Version::new(1, 3, 0));
    }

    #[test]
    fn test_bump_type_patch() {
        let version = Version::new(1, 2, 3);
        let bumped = BumpType::Patch.apply(&version);
        assert_eq!(bumped, Version::new(1, 2, 4));
    }

    #[test]
    fn test_version_update_new() {
        let update = VersionUpdate::new(
            "test-package".to_string(),
            "1.0.0".to_string(),
            "1.1.0".to_string(),
        );
        assert_eq!(update.package_name, "test-package");
        assert_eq!(update.old_version, "1.0.0");
        assert_eq!(update.new_version, "1.1.0");
    }
}
