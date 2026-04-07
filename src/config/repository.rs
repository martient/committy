// Repository-level configuration for multi-package support
// Loaded from .committy/config.toml

use super::{
    changelog::ChangelogConfig,
    convention::ConventionConfig,
    git::{validate_git_config_overrides, GitConfig},
    release::ReleaseConfig,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Repository-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub repository: RepositoryMetadata,
    pub versioning: VersioningConfig,
    pub packages: Vec<PackageConfig>,
    #[serde(default)]
    pub dependencies: Vec<DependencyConfig>,
    #[serde(default)]
    pub scopes: ScopeConfig,
    #[serde(default)]
    pub commit_rules: CommitRulesConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub convention: Option<ConventionConfig>,
    #[serde(default)]
    pub release: Option<ReleaseConfig>,
    #[serde(default)]
    pub changelog: Option<ChangelogConfig>,
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,
}

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    pub name: String,
    #[serde(rename = "type")]
    pub repo_type: RepositoryType,
    #[serde(default)]
    pub description: Option<String>,
}

/// Repository type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RepositoryType {
    SinglePackage,
    MultiPackage,
    Monorepo,
}

/// Versioning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningConfig {
    pub strategy: VersioningStrategy,
    #[serde(default)]
    pub unified_version: Option<String>,
    #[serde(default)]
    pub rules: Option<VersioningRules>,
}

/// Versioning strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VersioningStrategy {
    Independent,
    Unified,
    Hybrid,
}

/// Custom versioning rules (regex patterns)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningRules {
    #[serde(default)]
    pub major_regex: Option<String>,
    #[serde(default)]
    pub minor_regex: Option<String>,
    #[serde(default)]
    pub patch_regex: Option<String>,
}

/// Package configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub package_type: String,
    pub path: String,
    pub version_file: String,
    pub version_field: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub sync_with: Option<String>,
    #[serde(default)]
    pub independent: bool,
    #[serde(default)]
    pub workspace_member: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// Dependency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConfig {
    pub source: String,
    #[serde(default)]
    pub description: Option<String>,
    pub targets: Vec<DependencyTarget>,
}

/// Dependency target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTarget {
    pub file: String,
    pub field: String,
    pub strategy: UpdateStrategy,
    #[serde(default)]
    pub format: Option<String>,
}

/// Update strategy for dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateStrategy {
    Auto,
    Prompt,
    Manual,
}

/// Scope configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeConfig {
    #[serde(default = "default_true")]
    pub auto_detect: bool,
    #[serde(default = "default_true")]
    pub require_scope_for_multi_package: bool,
    #[serde(default = "default_true")]
    pub allow_multiple_scopes: bool,
    #[serde(default = "default_comma")]
    pub scope_separator: String,
    #[serde(default)]
    pub mappings: Vec<ScopeMapping>,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            require_scope_for_multi_package: true,
            allow_multiple_scopes: true,
            scope_separator: ",".to_string(),
            mappings: vec![],
        }
    }
}

/// Scope mapping (file pattern -> scope)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMapping {
    pub pattern: String,
    pub scope: String,
    pub package: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Commit rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRulesConfig {
    #[serde(default = "default_72")]
    pub max_subject_length: usize,
    #[serde(default = "default_100")]
    pub max_body_line_length: usize,
    #[serde(default)]
    pub require_body: bool,
    #[serde(default)]
    pub allowed_types: Vec<String>,
    #[serde(default)]
    pub custom_types: Vec<CustomCommitType>,
}

impl Default for CommitRulesConfig {
    fn default() -> Self {
        Self {
            max_subject_length: 72,
            max_body_line_length: 100,
            require_body: false,
            allowed_types: vec![],
            custom_types: vec![],
        }
    }
}

/// Custom commit type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommitType {
    pub name: String,
    pub description: String,
    pub bump: BumpType,
}

/// Version bump type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    None,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(rename = "type")]
    pub workspace_type: WorkspaceType,
    pub root: String,
    #[serde(default)]
    pub members: Vec<String>,
}

/// Workspace type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceType {
    Cargo,
    Npm,
    Pnpm,
    Yarn,
    Go,
    None,
}

// Helper functions for serde defaults
fn default_true() -> bool {
    true
}

fn default_comma() -> String {
    ",".to_string()
}

fn default_72() -> usize {
    72
}

fn default_100() -> usize {
    100
}

impl RepositoryConfig {
    /// Load repository configuration from .committy/config.toml
    pub fn load(repo_path: &Path) -> Result<Self> {
        let config_path = Self::get_config_path(repo_path)?;

        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "No .committy/config.toml found at {}",
                config_path.display()
            ));
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;

        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?;

        config.validate(repo_path)?;
        Ok(config)
    }

    /// Try to load repository configuration, returns None if not found
    pub fn try_load(repo_path: &Path) -> Result<Option<Self>> {
        match Self::load(repo_path) {
            Ok(config) => Ok(Some(config)),
            Err(e) => {
                if e.to_string().contains("No .committy/config.toml found") {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Get the path to the repository config file
    pub fn get_config_path(repo_path: &Path) -> Result<PathBuf> {
        Ok(repo_path.join(".committy").join("config.toml"))
    }

    /// Validate the configuration
    pub fn validate(&self, repo_path: &Path) -> Result<()> {
        validate_git_config_overrides(&self.git.config_overrides)?;

        // Validate package names are unique
        let mut names = HashSet::new();
        for pkg in &self.packages {
            if !names.insert(&pkg.name) {
                return Err(anyhow::anyhow!("Duplicate package name: {}", pkg.name));
            }
        }

        // Validate primary packages for hybrid strategy
        if self.versioning.strategy == VersioningStrategy::Hybrid {
            let primary_count = self.packages.iter().filter(|p| p.primary).count();
            if primary_count == 0 {
                return Err(anyhow::anyhow!(
                    "Hybrid strategy requires at least one primary package"
                ));
            }

            // Q13: Multiple primaries allowed, but they must sync
            let primaries: Vec<_> = self.packages.iter().filter(|p| p.primary).collect();
            if primaries.len() > 1 {
                // Check that all primaries sync with each other
                for primary in &primaries {
                    if let Some(ref sync_with) = primary.sync_with {
                        if !primaries.iter().any(|p| &p.name == sync_with) {
                            return Err(anyhow::anyhow!(
                                "Primary package '{}' must sync with another primary package, but syncs with '{}'",
                                primary.name, sync_with
                            ));
                        }
                    }
                }
            }
        }

        // Validate unified version if strategy is unified
        if self.versioning.strategy == VersioningStrategy::Unified
            && self.versioning.unified_version.is_none()
        {
            return Err(anyhow::anyhow!(
                "Unified strategy requires unified_version to be set"
            ));
        }

        // Validate sync_with references
        for pkg in &self.packages {
            if let Some(ref sync_with) = pkg.sync_with {
                if !self.packages.iter().any(|p| &p.name == sync_with) {
                    return Err(anyhow::anyhow!(
                        "Package '{}' syncs with non-existent package '{}'",
                        pkg.name,
                        sync_with
                    ));
                }
            }
        }

        // Q51: Full DAG support - validate no circular dependencies
        self.validate_no_cycles()?;

        // Validate dependency sources exist
        for dep in &self.dependencies {
            if !self.packages.iter().any(|p| p.name == dep.source) {
                return Err(anyhow::anyhow!(
                    "Dependency source '{}' does not exist",
                    dep.source
                ));
            }

            // Q48: Missing targets are errors - validate all target files exist
            for target in &dep.targets {
                let target_path = repo_path.join(&target.file);
                if !target_path.exists() {
                    return Err(anyhow::anyhow!(
                        "Dependency target file not found: {}",
                        target.file
                    ));
                }
            }
        }

        // Validate package paths and version files exist
        for pkg in &self.packages {
            let pkg_path = repo_path.join(&pkg.path);
            if !pkg_path.exists() {
                return Err(anyhow::anyhow!(
                    "Package path not found: {} (package: {})",
                    pkg.path,
                    pkg.name
                ));
            }

            let version_file_path = pkg_path.join(&pkg.version_file);
            if !version_file_path.exists() {
                return Err(anyhow::anyhow!(
                    "Version file not found: {} (package: {})",
                    pkg.version_file,
                    pkg.name
                ));
            }
        }

        Ok(())
    }

    /// Validate no circular dependencies in sync_with relationships
    fn validate_no_cycles(&self) -> Result<()> {
        let mut graph: HashMap<&str, &str> = HashMap::new();
        for pkg in &self.packages {
            if let Some(ref sync_with) = pkg.sync_with {
                graph.insert(&pkg.name, sync_with);
            }
        }

        // DFS to detect cycles
        for start in graph.keys() {
            let mut visited = HashSet::new();
            let mut current = *start;

            while let Some(next) = graph.get(current) {
                if !visited.insert(current) {
                    return Err(anyhow::anyhow!(
                        "Circular dependency detected: package '{}' has a cycle in sync_with chain",
                        start
                    ));
                }
                current = next;
            }
        }

        Ok(())
    }

    /// Check if multi-package mode is enabled
    pub fn is_multi_package(&self) -> bool {
        self.repository.repo_type != RepositoryType::SinglePackage || self.packages.len() > 1
    }
}

#[cfg(test)]
impl RepositoryConfig {
    /// Check if repository config exists
    pub fn exists(repo_path: &Path) -> bool {
        Self::get_config_path(repo_path)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Save repository configuration to .committy/config.toml
    pub fn save(&self, repo_path: &Path) -> Result<()> {
        let config_path = Self::get_config_path(repo_path)?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write {}", config_path.display()))?;

        Ok(())
    }

    /// Get the primary package (for hybrid strategy)
    pub fn get_primary_package(&self) -> Option<&PackageConfig> {
        self.packages.iter().find(|p| p.primary)
    }

    /// Get all packages that sync with a given package
    pub fn get_synced_packages(&self, package_name: &str) -> Vec<&PackageConfig> {
        self.packages
            .iter()
            .filter(|p| {
                p.sync_with
                    .as_ref()
                    .map(|s| s == package_name)
                    .unwrap_or(false)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = RepositoryConfig::get_config_path(temp_dir.path()).unwrap();
        assert_eq!(
            config_path,
            temp_dir.path().join(".committy").join("config.toml")
        );
    }

    #[test]
    fn test_config_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!RepositoryConfig::exists(temp_dir.path()));
    }

    #[test]
    fn test_get_primary_package() {
        let config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "pkg1".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "packages/pkg1".to_string(),
                    version_file: "packages/pkg1/Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: true,
                    sync_with: None,
                    independent: false,
                    workspace_member: true,
                    description: None,
                },
                PackageConfig {
                    name: "pkg2".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "packages/pkg2".to_string(),
                    version_file: "packages/pkg2/Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: Some("pkg1".to_string()),
                    independent: false,
                    workspace_member: true,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes: ScopeConfig::default(),
            commit_rules: CommitRulesConfig::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        };

        let primary = config.get_primary_package();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().name, "pkg1");
    }

    #[test]
    fn test_get_synced_packages() {
        let config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "pkg1".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "packages/pkg1".to_string(),
                    version_file: "packages/pkg1/Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: true,
                    sync_with: None,
                    independent: false,
                    workspace_member: true,
                    description: None,
                },
                PackageConfig {
                    name: "pkg2".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "packages/pkg2".to_string(),
                    version_file: "packages/pkg2/Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: Some("pkg1".to_string()),
                    independent: false,
                    workspace_member: true,
                    description: None,
                },
                PackageConfig {
                    name: "pkg3".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: "packages/pkg3".to_string(),
                    version_file: "packages/pkg3/Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: Some("pkg1".to_string()),
                    independent: false,
                    workspace_member: true,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes: ScopeConfig::default(),
            commit_rules: CommitRulesConfig::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        };

        let synced = config.get_synced_packages("pkg1");
        let names: Vec<_> = synced.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["pkg2", "pkg3"]);
    }

    #[test]
    fn test_validate_duplicate_package_names() {
        let config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "pkg1".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: ".".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "pkg1".to_string(), // Duplicate!
                    package_type: "rust-cargo".to_string(),
                    path: ".".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes: ScopeConfig::default(),
            commit_rules: CommitRulesConfig::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let result = config.validate(temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate package name"));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let temp_dir = TempDir::new().unwrap();

        let config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "pkg1".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: ".".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: Some("pkg2".to_string()),
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "pkg2".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: ".".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: false,
                    sync_with: Some("pkg1".to_string()), // Circular!
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes: ScopeConfig::default(),
            commit_rules: CommitRulesConfig::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        };

        let result = config.validate(temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }
}
