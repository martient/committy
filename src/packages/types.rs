// Package types and traits

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Package manager type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PackageManager {
    Cargo { workspace: bool },
    Npm { workspace: bool },
    Pnpm { workspace: bool },
    Yarn { workspace: bool },
    GoMod,
    Poetry,
    Pipenv,
    Maven,
    Gradle,
    Helm,
}

impl PackageManager {
    /// Get the package manager name as a string
    pub fn name(&self) -> &str {
        match self {
            PackageManager::Cargo { .. } => "Cargo",
            PackageManager::Npm { .. } => "npm",
            PackageManager::Pnpm { .. } => "pnpm",
            PackageManager::Yarn { .. } => "Yarn",
            PackageManager::GoMod => "Go modules",
            PackageManager::Poetry => "Poetry",
            PackageManager::Pipenv => "Pipenv",
            PackageManager::Maven => "Maven",
            PackageManager::Gradle => "Gradle",
            PackageManager::Helm => "HELM",
        }
    }

    /// Check if this is a workspace
    pub fn is_workspace(&self) -> bool {
        match self {
            PackageManager::Cargo { workspace } => *workspace,
            PackageManager::Npm { workspace } => *workspace,
            PackageManager::Pnpm { workspace } => *workspace,
            PackageManager::Yarn { workspace } => *workspace,
            _ => false,
        }
    }

    /// Get the package type string for config
    pub fn package_type(&self) -> &str {
        match self {
            PackageManager::Cargo { .. } => "rust-cargo",
            PackageManager::Npm { .. } => "node-npm",
            PackageManager::Pnpm { .. } => "node-pnpm",
            PackageManager::Yarn { .. } => "node-yarn",
            PackageManager::GoMod => "go-mod",
            PackageManager::Poetry => "python-poetry",
            PackageManager::Pipenv => "python-pipenv",
            PackageManager::Maven => "java-maven",
            PackageManager::Gradle => "java-gradle",
            PackageManager::Helm => "helm",
        }
    }
}

/// Information about a detected package
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package manager type
    pub manager: PackageManager,
    /// Path to package (relative to repository root)
    pub path: PathBuf,
    /// Current version
    pub version: String,
    /// Version file name (e.g., "Cargo.toml", "package.json")
    pub version_file: String,
    /// Version field path (e.g., "package.version", "version")
    pub version_field: String,
    /// Workspace members (if this is a workspace)
    pub workspace_members: Vec<String>,
}

impl PackageInfo {
    /// Get a display name for the package
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        format!("{} ({})", self.name, self.manager.name())
    }

    /// Check if this is a workspace
    pub fn is_workspace(&self) -> bool {
        self.manager.is_workspace()
    }
}

/// Trait for package manager detectors
pub trait PackageDetector: Send + Sync {
    /// Detect if a package exists at the given path
    /// Returns Some(PackageInfo) if detected, None otherwise
    fn detect(&self, path: &std::path::Path) -> Result<Option<PackageInfo>>;

    /// Get the version from a package at the given path
    fn get_version(&self, path: &std::path::Path) -> Result<String>;

    /// Set the version for a package at the given path
    fn set_version(&self, path: &std::path::Path, version: &str) -> Result<()>;

    /// Get the name of the package manager
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_name() {
        assert_eq!(PackageManager::Cargo { workspace: false }.name(), "Cargo");
        assert_eq!(PackageManager::Npm { workspace: false }.name(), "npm");
        assert_eq!(PackageManager::GoMod.name(), "Go modules");
    }

    #[test]
    fn test_package_manager_is_workspace() {
        assert!(PackageManager::Cargo { workspace: true }.is_workspace());
        assert!(!PackageManager::Cargo { workspace: false }.is_workspace());
        assert!(!PackageManager::GoMod.is_workspace());
    }

    #[test]
    fn test_package_manager_package_type() {
        assert_eq!(
            PackageManager::Cargo { workspace: false }.package_type(),
            "rust-cargo"
        );
        assert_eq!(
            PackageManager::Npm { workspace: false }.package_type(),
            "node-npm"
        );
        assert_eq!(PackageManager::GoMod.package_type(), "go-mod");
    }
}
