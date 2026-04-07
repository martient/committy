// Workflow orchestrator for coordinating scope detection, versioning, and dependency updates

use crate::config::repository::RepositoryConfig;
use crate::dependency::updater::{DependencyUpdate, DependencyUpdater};
use crate::scope::detector::ScopeDetector;
use crate::versioning::hybrid::HybridVersioning;
use crate::versioning::independent::IndependentVersioning;
use crate::versioning::manager::{BumpType, VersionManager, VersionUpdate};
use crate::versioning::unified::UnifiedVersioning;
use anyhow::{Context, Result};
use log::{debug, info};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Result of workflow orchestration
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowResult {
    /// Detected scopes from staged files
    pub scopes: Vec<String>,
    /// Version updates to apply
    pub version_updates: Vec<VersionUpdate>,
    /// Dependency updates to apply
    pub dependency_updates: Vec<DependencyUpdate>,
    /// Files that will be modified
    pub modified_files: Vec<PathBuf>,
}

impl WorkflowResult {
    /// Check if there are any changes to apply
    pub fn has_changes(&self) -> bool {
        !self.version_updates.is_empty() || !self.dependency_updates.is_empty()
    }
}

/// Workflow orchestrator for multi-package operations
pub struct WorkflowOrchestrator {
    repo_path: PathBuf,
    config: RepositoryConfig,
}

impl WorkflowOrchestrator {
    /// Create a new workflow orchestrator
    pub fn new(repo_path: &Path, config: RepositoryConfig) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            config,
        }
    }

    /// Detect scopes from staged files
    ///
    /// Returns detected scopes based on file patterns and package membership
    pub fn detect_scopes(&self) -> Result<Vec<String>> {
        if !self.config.scopes.auto_detect {
            debug!("Auto-detect is disabled in config");
            return Ok(vec![]);
        }

        let detector = ScopeDetector::new(self.config.clone(), &self.repo_path);
        let scopes = detector.detect_from_staged()?;

        info!("Detected scopes: {:?}", scopes);
        Ok(scopes)
    }

    /// Suggest scopes for the current repository state
    /// Calculate version updates based on commit message and affected packages
    ///
    /// # Arguments
    /// * `commit_message` - The commit message to analyze for bump type
    /// * `scopes` - Scopes/packages affected by the commit
    pub fn calculate_version_updates(
        &self,
        commit_message: &str,
        scopes: &[String],
    ) -> Result<Vec<VersionUpdate>> {
        // Determine bump type from commit message
        let bump_type = self.determine_bump_type(commit_message)?;

        // Calculate updates based on versioning strategy
        let updates = match self.config.versioning.strategy {
            crate::config::repository::VersioningStrategy::Independent => {
                let strategy = IndependentVersioning::new(self.config.clone(), &self.repo_path);
                strategy.calculate_updates(scopes, bump_type)?
            }
            crate::config::repository::VersioningStrategy::Unified => {
                let strategy = UnifiedVersioning::new(self.config.clone(), &self.repo_path);
                strategy.calculate_updates(scopes, bump_type)?
            }
            crate::config::repository::VersioningStrategy::Hybrid => {
                let strategy = HybridVersioning::new(self.config.clone(), &self.repo_path);
                strategy.calculate_updates(scopes, bump_type)?
            }
        };

        info!("Calculated {} version update(s)", updates.len());
        Ok(updates)
    }

    /// Calculate dependency updates based on version changes
    pub fn calculate_dependency_updates(
        &self,
        version_updates: &[VersionUpdate],
    ) -> Result<Vec<DependencyUpdate>> {
        if self.config.dependencies.is_empty() {
            debug!("No dependencies configured");
            return Ok(vec![]);
        }

        let updater = DependencyUpdater::new(self.config.clone(), &self.repo_path);
        let mut all_updates = Vec::new();

        for version_update in version_updates {
            let updates = updater
                .calculate_updates(&version_update.package_name, &version_update.new_version)?;
            all_updates.extend(updates);
        }

        info!("Calculated {} dependency update(s)", all_updates.len());
        Ok(all_updates)
    }

    /// Apply version updates to package files
    pub fn apply_version_updates(&self, updates: &[VersionUpdate]) -> Result<Vec<PathBuf>> {
        let mut updated_files = Vec::new();

        for update in updates {
            debug!(
                "Applying version update: {} {} -> {}",
                update.package_name, update.old_version, update.new_version
            );

            // Find the package config to get version_file and version_field
            let pkg_config = self
                .config
                .packages
                .iter()
                .find(|p| p.name == update.package_name)
                .context(format!(
                    "Package '{}' not found in config",
                    update.package_name
                ))?;

            // Update the version file for this package
            let version_file = self
                .repo_path
                .join(&pkg_config.path)
                .join(&pkg_config.version_file);

            // Use the appropriate handler based on file type
            self.update_version_file(
                &version_file,
                &pkg_config.version_field,
                &update.new_version,
            )?;

            updated_files.push(version_file);
        }

        Ok(updated_files)
    }

    /// Apply dependency updates to files
    pub fn apply_dependency_updates(&self, updates: &[DependencyUpdate]) -> Result<Vec<PathBuf>> {
        let updater = DependencyUpdater::new(self.config.clone(), &self.repo_path);

        let updated_file_strings = updater.apply_updates(updates)?;
        let updated_files = updated_file_strings
            .into_iter()
            .map(PathBuf::from)
            .collect();

        Ok(updated_files)
    }

    /// Run the full workflow: detect scopes, calculate updates, and optionally apply them
    ///
    /// # Arguments
    /// * `commit_message` - The commit message to analyze
    /// * `apply_changes` - Whether to apply the calculated changes
    pub fn run_workflow(
        &self,
        commit_message: &str,
        apply_changes: bool,
    ) -> Result<WorkflowResult> {
        info!("Running workflow orchestrator");

        // Step 1: Detect scopes
        let scopes = self.detect_scopes()?;

        // Step 2: Calculate version updates
        let version_updates = if !scopes.is_empty() {
            self.calculate_version_updates(commit_message, &scopes)?
        } else {
            debug!("No scopes detected, skipping version updates");
            vec![]
        };

        // Step 3: Calculate dependency updates
        let dependency_updates = self.calculate_dependency_updates(&version_updates)?;

        // Step 4: Apply changes if requested
        let mut modified_files = Vec::new();
        if apply_changes {
            let version_files = self.apply_version_updates(&version_updates)?;
            let dependency_files = self.apply_dependency_updates(&dependency_updates)?;

            modified_files.extend(version_files);
            modified_files.extend(dependency_files);

            // Deduplicate files
            let unique_files: HashSet<PathBuf> = modified_files.into_iter().collect();
            modified_files = unique_files.into_iter().collect();

            info!("Applied changes to {} file(s)", modified_files.len());
        }

        Ok(WorkflowResult {
            scopes,
            version_updates,
            dependency_updates,
            modified_files,
        })
    }

    /// Determine bump type from commit message
    fn determine_bump_type(&self, commit_message: &str) -> Result<BumpType> {
        // Use custom rules if available, otherwise use defaults
        let (major_pattern, minor_pattern) = if let Some(rules) = &self.config.versioning.rules {
            (
                rules
                    .major_regex
                    .as_deref()
                    .unwrap_or(crate::config::MAJOR_REGEX),
                rules
                    .minor_regex
                    .as_deref()
                    .unwrap_or(crate::config::MINOR_REGEX),
            )
        } else {
            (crate::config::MAJOR_REGEX, crate::config::MINOR_REGEX)
        };

        let major_regex = regex::Regex::new(major_pattern)?;
        let minor_regex = regex::Regex::new(minor_pattern)?;

        if major_regex.is_match(commit_message) {
            Ok(BumpType::Major)
        } else if minor_regex.is_match(commit_message) {
            Ok(BumpType::Minor)
        } else {
            // Default to patch for any other commit
            Ok(BumpType::Patch)
        }
    }

    /// Update a version file with a new version
    fn update_version_file(&self, file_path: &Path, field: &str, new_version: &str) -> Result<()> {
        use crate::dependency::handlers;

        let file_type = self.detect_file_type(file_path)?;

        match file_type.as_str() {
            "yaml" | "yml" => handlers::yaml::update_version(file_path, field, new_version),
            "json" => handlers::json::update_version(file_path, field, new_version),
            "toml" => handlers::toml::update_version(file_path, field, new_version),
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", file_type)),
        }
    }

    /// Detect file type from extension
    fn detect_file_type(&self, file_path: &Path) -> Result<String> {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        Ok(extension.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::repository::{
        PackageConfig, RepositoryConfig, RepositoryMetadata, RepositoryType, ScopeConfig,
        VersioningConfig, VersioningStrategy,
    };
    use tempfile::TempDir;

    fn create_test_config() -> RepositoryConfig {
        RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test-repo".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![PackageConfig {
                name: "test-package".to_string(),
                package_type: "rust-cargo".to_string(),
                path: ".".to_string(),
                version_file: "Cargo.toml".to_string(),
                version_field: "package.version".to_string(),
                primary: false,
                sync_with: None,
                independent: true,
                workspace_member: false,
                description: None,
            }],
            dependencies: vec![],
            scopes: ScopeConfig::default(),
            commit_rules: Default::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        }
    }

    #[test]
    fn test_orchestrator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let orchestrator = WorkflowOrchestrator::new(temp_dir.path(), config.clone());

        assert_eq!(orchestrator.repo_path, temp_dir.path());
        assert_eq!(orchestrator.config.repository.name, "test-repo");
    }

    #[test]
    fn test_orchestrator_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let orchestrator = WorkflowOrchestrator::new(temp_dir.path(), config.clone());

        assert_eq!(orchestrator.config.repository.name, "test-repo");
    }

    #[test]
    fn test_determine_bump_type() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let orchestrator = WorkflowOrchestrator::new(temp_dir.path(), config);

        assert_eq!(
            orchestrator
                .determine_bump_type("feat: add new feature")
                .unwrap(),
            BumpType::Minor
        );
        assert_eq!(
            orchestrator.determine_bump_type("fix: fix bug").unwrap(),
            BumpType::Patch
        );
        assert_eq!(
            orchestrator
                .determine_bump_type("feat!: breaking change")
                .unwrap(),
            BumpType::Major
        );
    }

    #[test]
    fn test_workflow_result_has_changes() {
        let result = WorkflowResult {
            scopes: vec!["test".to_string()],
            version_updates: vec![],
            dependency_updates: vec![],
            modified_files: vec![],
        };
        assert!(!result.has_changes());

        let result_with_changes = WorkflowResult {
            scopes: vec!["test".to_string()],
            version_updates: vec![VersionUpdate::new(
                "test".to_string(),
                "1.0.0".to_string(),
                "1.1.0".to_string(),
            )],
            dependency_updates: vec![],
            modified_files: vec![],
        };
        assert!(result_with_changes.has_changes());
    }

    #[test]
    fn test_detect_file_type() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let orchestrator = WorkflowOrchestrator::new(temp_dir.path(), config);

        assert_eq!(
            orchestrator
                .detect_file_type(Path::new("Cargo.toml"))
                .unwrap(),
            "toml"
        );
        assert_eq!(
            orchestrator
                .detect_file_type(Path::new("package.json"))
                .unwrap(),
            "json"
        );
        assert_eq!(
            orchestrator
                .detect_file_type(Path::new("values.yaml"))
                .unwrap(),
            "yaml"
        );
    }
}
