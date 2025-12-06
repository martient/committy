// Dependency updater for managing version references

use crate::config::repository::RepositoryConfig;
use anyhow::Result;
use log::debug;
use std::path::{Path, PathBuf};

/// Information about a dependency update
#[derive(Debug, Clone)]
pub struct DependencyUpdate {
    /// File that needs updating
    pub file_path: PathBuf,
    /// Package whose version is referenced
    pub package_name: String,
    /// Old version reference
    pub old_version: String,
    /// New version reference
    pub new_version: String,
    /// Field/key where the version is stored
    pub field: String,
}

impl DependencyUpdate {
    /// Create a new dependency update
    pub fn new(
        file_path: PathBuf,
        package_name: String,
        old_version: String,
        new_version: String,
        field: String,
    ) -> Self {
        Self {
            file_path,
            package_name,
            old_version,
            new_version,
            field,
        }
    }
}

/// Dependency updater for managing version references across files
pub struct DependencyUpdater {
    config: RepositoryConfig,
    repo_path: PathBuf,
}

impl DependencyUpdater {
    /// Create a new dependency updater
    pub fn new(config: RepositoryConfig, repo_path: &Path) -> Self {
        Self {
            config,
            repo_path: repo_path.to_path_buf(),
        }
    }

    /// Calculate dependency updates needed when a package version changes
    ///
    /// # Arguments
    /// * `package_name` - Name of the package whose version changed
    /// * `new_version` - New version of the package
    ///
    /// # Returns
    /// List of dependency updates to apply
    pub fn calculate_updates(
        &self,
        package_name: &str,
        new_version: &str,
    ) -> Result<Vec<DependencyUpdate>> {
        let mut updates = Vec::new();

        debug!(
            "Calculating dependency updates for package '{}' with version '{}'",
            package_name, new_version
        );

        // Find all dependencies that reference this package
        for dep in &self.config.dependencies {
            if dep.source == package_name {
                debug!(
                    "Found dependency: source '{}' has {} target(s)",
                    dep.source,
                    dep.targets.len()
                );

                // Process each target
                for target in &dep.targets {
                    let file_path = self.repo_path.join(&target.file);

                    if let Ok(old_version) = self.read_version_from_file(&file_path, target) {
                        if old_version != new_version {
                            updates.push(DependencyUpdate::new(
                                file_path,
                                package_name.to_string(),
                                old_version,
                                new_version.to_string(),
                                target.field.clone(),
                            ));
                        }
                    } else {
                        debug!("Could not read version from {}", target.file);
                    }
                }
            }
        }

        Ok(updates)
    }

    /// Apply dependency updates to files
    pub fn apply_updates(&self, updates: &[DependencyUpdate]) -> Result<Vec<String>> {
        let mut updated_files = Vec::new();

        for update in updates {
            debug!(
                "Applying update to {} (field '{}'): {} -> {}",
                update.file_path.display(),
                update.field,
                update.old_version,
                update.new_version
            );

            // Find the dependency target for this file
            let target = self.find_target_for_update(update)?;

            // Update the file based on its type
            self.update_file(&update.file_path, target, &update.new_version)?;

            updated_files.push(update.file_path.display().to_string());
        }

        Ok(updated_files)
    }

    /// Find the dependency target for an update
    fn find_target_for_update(
        &self,
        update: &DependencyUpdate,
    ) -> Result<&crate::config::repository::DependencyTarget> {
        for dep in &self.config.dependencies {
            if dep.source == update.package_name {
                for target in &dep.targets {
                    if self.repo_path.join(&target.file) == update.file_path {
                        return Ok(target);
                    }
                }
            }
        }
        Err(anyhow::anyhow!(
            "Dependency target not found for {}",
            update.file_path.display()
        ))
    }

    /// Read version from a file based on dependency target
    fn read_version_from_file(
        &self,
        file_path: &Path,
        target: &crate::config::repository::DependencyTarget,
    ) -> Result<String> {
        use super::handlers;

        let file_type = self.detect_file_type(file_path)?;

        match file_type.as_str() {
            "yaml" | "yml" => handlers::yaml::read_version(file_path, &target.field),
            "json" => handlers::json::read_version(file_path, &target.field),
            "toml" => handlers::toml::read_version(file_path, &target.field),
            "dockerfile" => {
                // For dockerfile, extract package name from field (e.g., "myapp" from field)
                handlers::dockerfile::read_version(file_path, &target.field)
            }
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", file_type)),
        }
    }

    /// Update version in a file based on dependency target
    fn update_file(
        &self,
        file_path: &Path,
        target: &crate::config::repository::DependencyTarget,
        new_version: &str,
    ) -> Result<()> {
        use super::handlers;

        let file_type = self.detect_file_type(file_path)?;

        match file_type.as_str() {
            "yaml" | "yml" => handlers::yaml::update_version(file_path, &target.field, new_version),
            "json" => handlers::json::update_version(file_path, &target.field, new_version),
            "toml" => handlers::toml::update_version(file_path, &target.field, new_version),
            "dockerfile" => {
                // For dockerfile, use field as package name
                handlers::dockerfile::update_version(file_path, &target.field, new_version)
            }
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", file_type)),
        }
    }

    /// Detect file type from extension
    fn detect_file_type(&self, file_path: &Path) -> Result<String> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

        if file_name.to_lowercase() == "dockerfile" || file_name.starts_with("Dockerfile") {
            return Ok("dockerfile".to_string());
        }

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
        DependencyConfig, PackageConfig, RepositoryConfig, RepositoryMetadata, RepositoryType,
        VersioningConfig, VersioningStrategy,
    };
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> RepositoryConfig {
        use crate::config::repository::{DependencyTarget, UpdateStrategy};

        RepositoryConfig {
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
            packages: vec![PackageConfig {
                name: "cli".to_string(),
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
            dependencies: vec![DependencyConfig {
                source: "cli".to_string(),
                description: Some("CLI version in HELM chart".to_string()),
                targets: vec![DependencyTarget {
                    file: "chart/values.yaml".to_string(),
                    field: "image.tag".to_string(),
                    strategy: UpdateStrategy::Auto,
                    format: None,
                }],
            }],
            scopes: Default::default(),
            commit_rules: Default::default(),
            workspace: None,
        }
    }

    #[test]
    fn test_detect_file_type() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let updater = DependencyUpdater::new(config, temp_dir.path());

        assert_eq!(
            updater.detect_file_type(Path::new("values.yaml")).unwrap(),
            "yaml"
        );
        assert_eq!(
            updater.detect_file_type(Path::new("package.json")).unwrap(),
            "json"
        );
        assert_eq!(
            updater.detect_file_type(Path::new("Cargo.toml")).unwrap(),
            "toml"
        );
        assert_eq!(
            updater.detect_file_type(Path::new("Dockerfile")).unwrap(),
            "dockerfile"
        );
    }

    #[test]
    fn test_calculate_updates() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();

        // Create the values.yaml file
        let chart_dir = temp_dir.path().join("chart");
        fs::create_dir(&chart_dir).unwrap();
        fs::write(
            chart_dir.join("values.yaml"),
            r#"
image:
  tag: "1.0.0"
"#,
        )
        .unwrap();

        let updater = DependencyUpdater::new(config, temp_dir.path());
        let updates = updater.calculate_updates("cli", "1.1.0").unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].package_name, "cli");
        assert_eq!(updates[0].old_version, "1.0.0");
        assert_eq!(updates[0].new_version, "1.1.0");
        assert_eq!(updates[0].field, "image.tag");
    }

    #[test]
    fn test_dependency_update_new() {
        let update = DependencyUpdate::new(
            PathBuf::from("values.yaml"),
            "cli".to_string(),
            "1.0.0".to_string(),
            "1.1.0".to_string(),
            "image.tag".to_string(),
        );

        assert_eq!(update.file_path, PathBuf::from("values.yaml"));
        assert_eq!(update.package_name, "cli");
        assert_eq!(update.old_version, "1.0.0");
        assert_eq!(update.new_version, "1.1.0");
        assert_eq!(update.field, "image.tag");
    }
}
