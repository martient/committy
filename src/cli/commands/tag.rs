use std::collections::HashMap;

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::{RepositoryConfig, VersioningStrategy};
use crate::dependency::updater::DependencyUpdater;
use crate::error::CliError;
use crate::git;
use crate::input;
use crate::scope::detector::ScopeDetector;
use crate::telemetry;
use crate::versioning::hybrid::HybridVersioning;
use crate::versioning::independent::IndependentVersioning;
use crate::versioning::manager::{BumpType, VersionManager};
use crate::versioning::unified::UnifiedVersioning;
use log::debug;
use log::info;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct TagCommand {
    #[structopt(short, long, help = "Provide a tag name")]
    name: Option<String>,

    #[structopt(short = "y", long, help = "Want to create a new version (y/N)")]
    validate: bool,

    #[structopt(
        short = "b",
        long = "bump-files",
        help = "Want to auto bump the config to the new version (y/N)"
    )]
    bump_config_files: bool,

    #[structopt(flatten)]
    tag_options: git::TagGeneratorOptions,

    /// Output format: text or json
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    /// Update dependency references during tagging
    #[structopt(long, help = "Update dependency references in other packages")]
    update_deps: bool,

    #[structopt(
        long = "git-config",
        help = "Pass through git -c key=value overrides (repeatable)"
    )]
    git_config: Vec<String>,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct TagCommandOutput {
    command: String,
    ok: bool,
    dry_run: bool,
    errors: Option<Vec<String>>,
    old_tag: Option<String>,
    new_tag: Option<String>,
    pre_release: Option<bool>,
    published: bool,
}

impl Command for TagCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        if self.tag_options.publish_requested() && !self.tag_options.confirm_publish() {
            return Err(CliError::InputError(
                "Publishing a tag requires --confirm-publish".to_string(),
            ));
        }

        let default_repo_path = PathBuf::from(".");
        let effective_repo_path = if self.repo_path != default_repo_path {
            self.repo_path.clone()
        } else {
            PathBuf::from(&self.tag_options.source)
        };

        if self.tag_options.source != "."
            && self.repo_path != default_repo_path
            && Path::new(&self.tag_options.source) != self.repo_path.as_path()
        {
            return Err(CliError::InputError(
                "Use either --repo-path or --source for tag repository selection, not both"
                    .to_string(),
            ));
        }

        if git::has_staged_changes_from(&effective_repo_path)? {
            return Err(CliError::StagedChanges);
        }

        let git_command_config =
            git::resolve_git_command_config(&effective_repo_path, &self.git_config)?;

        // Load merged config to detect multi-package mode
        let merged_config = MergedConfig::load(&effective_repo_path).ok();

        // Check if multi-package mode
        let is_multi_package = merged_config
            .as_ref()
            .map(|c| c.is_multi_package())
            .unwrap_or(false);

        let mut tag_options = self.tag_options.clone();
        tag_options.source = effective_repo_path.display().to_string();

        if let Some(name) = &self.name {
            // Explicit tag name provided - use legacy flow
            let version_manager = git::TagGenerator::new(
                tag_options.clone(),
                self.bump_config_files,
                git_command_config.clone(),
            );
            version_manager.create_and_push_tag(&version_manager.open_repository()?, name)?;
            let payload = TagCommandOutput {
                command: "tag".into(),
                ok: true,
                dry_run: self.tag_options.dry_run(),
                errors: None,
                old_tag: None,
                new_tag: Some(name.clone()),
                pre_release: None,
                published: self.tag_options.will_publish_remote(),
            };
            if self.output == "json" {
                println!("{}", serde_json::to_string(&payload).unwrap());
            } else {
                println!("Tag {name} created successfully!");
            }
        } else if is_multi_package {
            // Multi-package mode - use new strategy-aware flow
            self.execute_multi_package(
                non_interactive,
                merged_config.as_ref().unwrap(),
                &effective_repo_path,
                &git_command_config,
            )?;
        } else if non_interactive {
            // In non-interactive mode, auto-calculate and act based on options
            let mut version_manager = git::TagGenerator::new(
                tag_options.clone(),
                self.bump_config_files,
                git_command_config.clone(),
            );
            version_manager.run()?;

            // Print the calculated tag so callers/tests can consume it
            let payload = TagCommandOutput {
                command: "tag".into(),
                ok: true,
                dry_run: self.tag_options.dry_run(),
                errors: None,
                old_tag: Some(version_manager.current_tag.clone()),
                new_tag: Some(version_manager.new_tag.clone()),
                pre_release: Some(version_manager.is_pre_release),
                published: self.tag_options.will_publish_remote(),
            };
            if self.output == "json" {
                println!("{}", serde_json::to_string(&payload).unwrap());
            } else {
                println!("{}", version_manager.new_tag);
            }
        } else {
            let validate = if !self.validate {
                input::ask_want_create_new_tag()?
            } else {
                true
            };
            if !validate {
                info!("Abort");
                return Ok(());
            }
            let mut version_manager = git::TagGenerator::new(
                tag_options,
                self.bump_config_files,
                git_command_config.clone(),
            );
            version_manager.run()?;
            let payload = TagCommandOutput {
                command: "tag".into(),
                ok: true,
                dry_run: self.tag_options.dry_run(),
                errors: None,
                old_tag: Some(version_manager.current_tag.clone()),
                new_tag: Some(version_manager.new_tag.clone()),
                pre_release: Some(version_manager.is_pre_release),
                published: self.tag_options.will_publish_remote(),
            };
            if self.output == "json" {
                println!("{}", serde_json::to_string(&payload).unwrap());
            } else {
                println!("Tag {} created successfully!", version_manager.new_tag);
            }
            if let Err(e) =
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(telemetry::posthog::publish_event(
                        "tag_created",
                        HashMap::from([
                            ("old_tag", Value::from(version_manager.current_tag)),
                            ("new_tag", Value::from(version_manager.new_tag)),
                            (
                                "is_pre_release",
                                Value::from(version_manager.is_pre_release),
                            ),
                            ("allow_bump_files", Value::from(self.bump_config_files)),
                        ]),
                    ))
            {
                debug!("Telemetry error: {e:?}");
            }
        }

        Ok(())
    }
}

impl TagCommand {
    /// Execute multi-package tag operation
    fn execute_multi_package(
        &self,
        _non_interactive: bool,
        config: &MergedConfig,
        repo_path: &Path,
        git_command_config: &git::GitCommandConfig,
    ) -> Result<(), CliError> {
        let repo_config = config
            .repository
            .as_ref()
            .ok_or_else(|| CliError::Generic("Repository config not found".to_string()))?;

        info!(
            "🏢 Multi-package mode detected, using {:?} strategy",
            repo_config.versioning.strategy
        );

        // Step 1: Get commit log since last tag
        let repo = git::discover_repository_from(repo_path)?;
        let latest_tag = self.find_latest_tag(&repo)?;
        let latest_tag = (!latest_tag.is_empty()).then_some(latest_tag);
        let commit_log =
            self.get_commit_log_since_tag(&repo, latest_tag.as_deref(), git_command_config)?;
        debug!("Commit log since last tag:\n{}", commit_log);

        // Step 2: Detect affected packages from commit scopes and changed files
        let changed_files =
            self.get_changed_files_since_tag(&repo, latest_tag.as_deref(), git_command_config)?;
        let affected_packages =
            self.detect_affected_packages(&commit_log, &changed_files, repo_config)?;
        if affected_packages.is_empty() {
            info!("ℹ️ No packages affected by commits. Skipping tag creation.");
            return Ok(());
        }
        info!("📦 Affected packages: {}", affected_packages.join(", "));

        // Step 3: Determine version bump type from commit messages
        let bump_type = self.determine_bump_type(&commit_log, config)?;
        info!("📈 Version bump type: {:?}", bump_type);

        // Step 4: Route through versioning strategy
        let version_updates =
            self.calculate_version_updates(repo_config, repo_path, &affected_packages, bump_type)?;

        if version_updates.is_empty() {
            info!("ℹ️ No version updates calculated. Skipping tag creation.");
            return Ok(());
        }

        if self.tag_options.dry_run() {
            for tag_name in planned_multi_package_tags(repo_config, &version_updates) {
                info!("🧪 Dry run: Tag would be {}", tag_name);
                if self.output != "json" {
                    println!("{}", tag_name);
                }
            }
            return Ok(());
        }

        // Step 5: Update version files per package
        if self.bump_config_files {
            self.apply_version_updates(repo_path, &version_updates)?;
            info!(
                "✅ Updated version files for {} package(s)",
                version_updates.len()
            );
        }

        // Step 6: Update dependencies if requested
        if self.update_deps {
            self.apply_dependency_updates(repo_config, repo_path, &version_updates)?;
            info!("✅ Updated dependency references");
        }

        // Step 7: Create tags based on versioning strategy
        self.create_multi_package_tags(&repo, repo_config, &version_updates, git_command_config)?;
        info!("✅ Tags created successfully");

        Ok(())
    }

    /// Get commit log since last tag
    fn get_commit_log_since_tag(
        &self,
        repo: &git2::Repository,
        latest_tag: Option<&str>,
        git_command_config: &git::GitCommandConfig,
    ) -> Result<String, CliError> {
        let range = if latest_tag.is_none() {
            "HEAD".to_string()
        } else {
            format!("{}..HEAD", latest_tag.unwrap())
        };

        // Use git log to get commit messages
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        let output = git::run_git_capture(
            repo_path,
            &["log", "--pretty=%B", &range],
            "get commit log",
            git_command_config,
        )?;

        String::from_utf8(output.stdout)
            .map_err(|e| CliError::Generic(format!("Invalid UTF-8 in commit log: {}", e)))
    }

    fn get_changed_files_since_tag(
        &self,
        repo: &git2::Repository,
        latest_tag: Option<&str>,
        git_command_config: &git::GitCommandConfig,
    ) -> Result<Vec<PathBuf>, CliError> {
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        let output = if let Some(tag) = latest_tag {
            let range = format!("{tag}..HEAD");
            git::run_git_capture(
                repo_path,
                &["diff", "--name-only", &range],
                "get changed files",
                git_command_config,
            )?
        } else {
            git::run_git_capture(
                repo_path,
                &["log", "--format=", "--name-only", "HEAD"],
                "get changed files",
                git_command_config,
            )?
        };
        let files = String::from_utf8(output.stdout)
            .map_err(|e| CliError::Generic(format!("Invalid UTF-8 in changed file list: {}", e)))?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        Ok(files)
    }

    /// Find the latest tag in the repository
    fn find_latest_tag(&self, repo: &git2::Repository) -> Result<String, CliError> {
        let tags = repo.tag_names(None).map_err(CliError::from)?;
        Ok(tags.iter().flatten().last().unwrap_or("").to_string())
    }

    /// Detect affected packages from commit log using scopes
    fn detect_affected_packages(
        &self,
        commit_log: &str,
        changed_files: &[PathBuf],
        config: &RepositoryConfig,
    ) -> Result<Vec<String>, CliError> {
        let mut packages = std::collections::HashSet::new();

        // Extract scopes from commit headers (pattern: type(scope)!: message)
        let scope_regex = Regex::new(r"^[a-z0-9-]+\(([^)]+)\)(?:!)?:")
            .map_err(|e| CliError::Generic(format!("Regex error: {}", e)))?;

        for line in commit_log.lines() {
            if let Some(caps) = scope_regex.captures(line) {
                let scope = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                for package in resolve_scope_packages(scope, config) {
                    packages.insert(package);
                }
            }
        }

        if !changed_files.is_empty() {
            let detector = ScopeDetector::new(config.clone(), &self.repo_path);
            let changed_file_scopes = detector.detect_from_files(changed_files).map_err(|e| {
                CliError::Generic(format!("Failed to detect scopes from files: {e}"))
            })?;
            for scope in changed_file_scopes {
                for package in resolve_scope_packages(&scope, config) {
                    packages.insert(package);
                }
            }
        }

        let mut packages = packages.into_iter().collect::<Vec<_>>();
        packages.sort();
        Ok(packages)
    }

    /// Determine version bump type from commit messages
    fn determine_bump_type(
        &self,
        commit_log: &str,
        config: &MergedConfig,
    ) -> Result<BumpType, CliError> {
        let major_regex = config.get_major_regex();
        let minor_regex = config.get_minor_regex();

        let major_re = Regex::new(major_regex)
            .map_err(|e| CliError::Generic(format!("Invalid major regex: {}", e)))?;
        let minor_re = Regex::new(minor_regex)
            .map_err(|e| CliError::Generic(format!("Invalid minor regex: {}", e)))?;

        if major_re.is_match(commit_log) {
            Ok(BumpType::Major)
        } else if minor_re.is_match(commit_log) {
            Ok(BumpType::Minor)
        } else {
            Ok(BumpType::Patch)
        }
    }

    /// Calculate version updates using appropriate strategy
    fn calculate_version_updates(
        &self,
        config: &RepositoryConfig,
        repo_path: &Path,
        affected_packages: &[String],
        bump_type: BumpType,
    ) -> Result<Vec<crate::versioning::manager::VersionUpdate>, CliError> {
        let strategy = &config.versioning.strategy;

        let updates = match strategy {
            VersioningStrategy::Independent => {
                let strat = IndependentVersioning::new(config.clone(), repo_path);
                strat
                    .calculate_updates(affected_packages, bump_type)
                    .map_err(|e| CliError::Generic(format!("Failed to calculate updates: {}", e)))?
            }
            VersioningStrategy::Unified => {
                let strat = UnifiedVersioning::new(config.clone(), repo_path);
                strat
                    .calculate_updates(affected_packages, bump_type)
                    .map_err(|e| CliError::Generic(format!("Failed to calculate updates: {}", e)))?
            }
            VersioningStrategy::Hybrid => {
                let strat = HybridVersioning::new(config.clone(), repo_path);
                strat
                    .calculate_updates(affected_packages, bump_type)
                    .map_err(|e| CliError::Generic(format!("Failed to calculate updates: {}", e)))?
            }
        };

        Ok(updates)
    }

    /// Apply version updates to package files
    fn apply_version_updates(
        &self,
        repo_path: &Path,
        updates: &[crate::versioning::manager::VersionUpdate],
    ) -> Result<(), CliError> {
        for update in updates {
            // Find package config to get version file
            let config = RepositoryConfig::try_load(repo_path)
                .map_err(|e| CliError::Generic(format!("Failed to load config: {}", e)))?
                .ok_or_else(|| CliError::Generic("No repository config".to_string()))?;

            let pkg_config = config
                .packages
                .iter()
                .find(|p| p.name == update.package_name)
                .ok_or_else(|| {
                    CliError::Generic(format!(
                        "Package '{}' not found in config",
                        update.package_name
                    ))
                })?;

            // Update version file
            let version_file = repo_path
                .join(&pkg_config.path)
                .join(&pkg_config.version_file);

            let content = fs::read_to_string(&version_file)
                .map_err(|e| CliError::Generic(format!("Failed to read version file: {}", e)))?;

            let updated = content.replace(&update.old_version, &update.new_version);

            fs::write(&version_file, updated)
                .map_err(|e| CliError::Generic(format!("Failed to write version file: {}", e)))?;

            // Stage the updated file
            if let Err(e) = git::stage_file(&version_file) {
                debug!("Failed to stage {}: {}", version_file.display(), e);
            }
        }

        Ok(())
    }

    /// Apply dependency updates to other packages
    fn apply_dependency_updates(
        &self,
        config: &RepositoryConfig,
        repo_path: &Path,
        updates: &[crate::versioning::manager::VersionUpdate],
    ) -> Result<(), CliError> {
        let updater = DependencyUpdater::new(config.clone(), repo_path);

        for update in updates {
            match updater.calculate_updates(&update.package_name, &update.new_version) {
                Ok(dep_updates) => {
                    match updater.apply_updates(&dep_updates) {
                        Ok(updated_files) => {
                            for file in updated_files {
                                // Stage the updated dependency file
                                if let Err(e) = git::stage_file(Path::new(&file)) {
                                    debug!("Failed to stage {}: {}", file, e);
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Failed to apply dependency updates: {}", e);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to calculate dependency updates: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Create appropriate tags based on versioning strategy
    fn create_multi_package_tags(
        &self,
        repo: &git2::Repository,
        config: &RepositoryConfig,
        updates: &[crate::versioning::manager::VersionUpdate],
        git_command_config: &git::GitCommandConfig,
    ) -> Result<(), CliError> {
        let strategy = &config.versioning.strategy;

        match strategy {
            VersioningStrategy::Unified => {
                // Single tag for all packages
                if let Some(update) = updates.first() {
                    let tag_name = format!("v{}", update.new_version);
                    self.create_and_push_tag(repo, &tag_name, git_command_config)?;
                    info!("📌 Created unified tag: {}", tag_name);
                }
            }
            VersioningStrategy::Independent => {
                // Per-package tags
                for update in updates {
                    let tag_name = format!("{}-v{}", update.package_name, update.new_version);
                    self.create_and_push_tag(repo, &tag_name, git_command_config)?;
                    info!("📌 Created tag: {}", tag_name);
                }
            }
            VersioningStrategy::Hybrid => {
                if let Some(tag_name) = hybrid_tag_name(config, updates) {
                    self.create_and_push_tag(repo, &tag_name, git_command_config)?;
                    info!("📌 Created hybrid tag: {}", tag_name);
                } else {
                    info!("ℹ️ No primary package version change. Skipping tag creation.");
                }
            }
        }

        Ok(())
    }

    /// Create a git tag and optionally push to remote
    fn create_and_push_tag(
        &self,
        repo: &git2::Repository,
        tag_name: &str,
        git_command_config: &git::GitCommandConfig,
    ) -> Result<(), CliError> {
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        git::run_git(
            repo_path,
            &["tag", "-a", tag_name, "-m", tag_name],
            "create tag",
            git_command_config,
        )?;

        info!("✅ Tag '{}' created locally", tag_name);

        // Try to push to remote
        if self.tag_options.will_publish_remote() {
            if let Err(e) = self.push_tag_to_remote(repo, tag_name, git_command_config) {
                debug!("Failed to push tag to remote: {}", e);
            } else {
                info!("📤 Tag '{}' pushed to remote", tag_name);
            }
        }

        Ok(())
    }

    /// Push tag to remote repository
    fn push_tag_to_remote(
        &self,
        repo: &git2::Repository,
        tag_name: &str,
        git_command_config: &git::GitCommandConfig,
    ) -> Result<(), CliError> {
        repo.find_remote("origin").map_err(CliError::from)?;
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        git::run_git(
            repo_path,
            &["push", "origin", &format!("refs/tags/{tag_name}")],
            "push tag to remote",
            git_command_config,
        )
    }
}

fn resolve_scope_packages(scope_value: &str, config: &RepositoryConfig) -> Vec<String> {
    let mut packages = std::collections::HashSet::new();
    let separator = config.scopes.scope_separator.as_str();

    let scopes = if separator.is_empty() {
        vec![scope_value]
    } else {
        scope_value.split(separator).collect::<Vec<_>>()
    };

    for scope in scopes {
        let scope = scope.trim();
        if scope.is_empty() {
            continue;
        }

        if config.packages.iter().any(|package| package.name == scope) {
            packages.insert(scope.to_string());
        }

        for mapping in config
            .scopes
            .mappings
            .iter()
            .filter(|mapping| mapping.scope == scope)
        {
            if config
                .packages
                .iter()
                .any(|package| package.name == mapping.package)
            {
                packages.insert(mapping.package.clone());
            }
        }
    }

    let mut packages = packages.into_iter().collect::<Vec<_>>();
    packages.sort();
    packages
}

fn hybrid_tag_name(
    config: &RepositoryConfig,
    updates: &[crate::versioning::manager::VersionUpdate],
) -> Option<String> {
    let primary = config.packages.iter().find(|package| package.primary)?;
    let update = updates
        .iter()
        .find(|update| update.package_name == primary.name)?;
    Some(format!("v{}", update.new_version))
}

fn planned_multi_package_tags(
    config: &RepositoryConfig,
    updates: &[crate::versioning::manager::VersionUpdate],
) -> Vec<String> {
    match config.versioning.strategy {
        VersioningStrategy::Unified => updates
            .first()
            .map(|update| vec![format!("v{}", update.new_version)])
            .unwrap_or_default(),
        VersioningStrategy::Independent => updates
            .iter()
            .map(|update| format!("{}-v{}", update.package_name, update.new_version))
            .collect(),
        VersioningStrategy::Hybrid => hybrid_tag_name(config, updates).into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{hybrid_tag_name, planned_multi_package_tags, resolve_scope_packages, TagCommand};
    use crate::config::repository::{
        PackageConfig, RepositoryConfig, RepositoryMetadata, RepositoryType, ScopeConfig,
        ScopeMapping, VersioningConfig, VersioningStrategy,
    };
    use crate::versioning::manager::VersionUpdate;
    use std::path::PathBuf;
    use structopt::StructOpt;

    fn create_multi_package_config() -> RepositoryConfig {
        RepositoryConfig {
            repository: RepositoryMetadata {
                name: "workspace".to_string(),
                repo_type: RepositoryType::MultiPackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Hybrid,
                unified_version: None,
                rules: None,
            },
            packages: vec![
                PackageConfig {
                    name: "committy-cli".to_string(),
                    package_type: "rust-cargo".to_string(),
                    path: ".".to_string(),
                    version_file: "Cargo.toml".to_string(),
                    version_field: "package.version".to_string(),
                    primary: true,
                    sync_with: None,
                    independent: false,
                    workspace_member: false,
                    description: None,
                },
                PackageConfig {
                    name: "docs".to_string(),
                    package_type: "node-npm".to_string(),
                    path: "docs".to_string(),
                    version_file: "package.json".to_string(),
                    version_field: "version".to_string(),
                    primary: false,
                    sync_with: None,
                    independent: true,
                    workspace_member: false,
                    description: None,
                },
            ],
            dependencies: vec![],
            scopes: ScopeConfig {
                auto_detect: true,
                require_scope_for_multi_package: true,
                allow_multiple_scopes: true,
                scope_separator: ",".to_string(),
                mappings: vec![
                    ScopeMapping {
                        pattern: "src/**".to_string(),
                        scope: "core".to_string(),
                        package: "committy-cli".to_string(),
                        description: None,
                    },
                    ScopeMapping {
                        pattern: "docs/**".to_string(),
                        scope: "docs".to_string(),
                        package: "docs".to_string(),
                        description: None,
                    },
                ],
            },
            commit_rules: Default::default(),
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        }
    }

    #[test]
    fn resolve_scope_packages_uses_scope_mappings() {
        let config = create_multi_package_config();

        assert_eq!(
            resolve_scope_packages("core", &config),
            vec!["committy-cli"]
        );
    }

    #[test]
    fn detect_affected_packages_accepts_breaking_multi_scope_headers() {
        let command = TagCommand::from_iter(["tag"]);
        let config = create_multi_package_config();
        let packages = command
            .detect_affected_packages("feat(core, docs)!: release both\n", &[], &config)
            .unwrap();

        assert_eq!(packages, vec!["committy-cli", "docs"]);
    }

    #[test]
    fn detect_affected_packages_falls_back_to_changed_files() {
        let command = TagCommand::from_iter(["tag"]);
        let config = create_multi_package_config();
        let packages = command
            .detect_affected_packages(
                "fix: tag mapping\n",
                &[PathBuf::from("src/cli/commands/tag.rs")],
                &config,
            )
            .unwrap();

        assert_eq!(packages, vec!["committy-cli"]);
    }

    #[test]
    fn hybrid_tag_name_uses_primary_package_version() {
        let config = create_multi_package_config();
        let tag = hybrid_tag_name(
            &config,
            &[
                VersionUpdate::new(
                    "committy-cli".to_string(),
                    "1.0.0".to_string(),
                    "1.0.1".to_string(),
                ),
                VersionUpdate::new("docs".to_string(), "2.0.0".to_string(), "2.0.1".to_string()),
            ],
        );

        assert_eq!(tag.as_deref(), Some("v1.0.1"));
    }

    #[test]
    fn hybrid_tag_name_skips_when_only_independent_package_changes() {
        let config = create_multi_package_config();
        let tag = hybrid_tag_name(
            &config,
            &[VersionUpdate::new(
                "docs".to_string(),
                "2.0.0".to_string(),
                "2.0.1".to_string(),
            )],
        );

        assert!(tag.is_none());
    }

    #[test]
    fn planned_multi_package_tags_uses_hybrid_repo_tag() {
        let config = create_multi_package_config();
        let tags = planned_multi_package_tags(
            &config,
            &[VersionUpdate::new(
                "committy-cli".to_string(),
                "1.0.0".to_string(),
                "1.0.1".to_string(),
            )],
        );

        assert_eq!(tags, vec!["v1.0.1"]);
    }
}
