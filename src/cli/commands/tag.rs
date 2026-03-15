use std::collections::HashMap;

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::{RepositoryConfig, VersioningStrategy};
use crate::dependency::updater::DependencyUpdater;
use crate::error::CliError;
use crate::git;
use crate::input;
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
            let version_manager =
                git::TagGenerator::new(tag_options.clone(), self.bump_config_files);
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
            )?;
        } else if non_interactive {
            // In non-interactive mode, auto-calculate and act based on options
            let mut version_manager =
                git::TagGenerator::new(tag_options.clone(), self.bump_config_files);
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
            let mut version_manager = git::TagGenerator::new(tag_options, self.bump_config_files);
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
        let commit_log = self.get_commit_log_since_last_tag(&repo)?;
        debug!("Commit log since last tag:\n{}", commit_log);

        // Step 2: Detect scopes (affected packages) from commit log
        let affected_packages = self.detect_affected_packages(&commit_log, repo_config)?;
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
        self.create_multi_package_tags(&repo, repo_config, &version_updates)?;
        info!("✅ Tags created successfully");

        Ok(())
    }

    /// Get commit log since last tag
    fn get_commit_log_since_last_tag(&self, repo: &git2::Repository) -> Result<String, CliError> {
        // Try to find the latest tag
        let latest_tag = self.find_latest_tag(repo)?;
        let range = if latest_tag.is_empty() {
            "HEAD".to_string()
        } else {
            format!("{}..HEAD", latest_tag)
        };

        // Use git log to get commit messages
        let repo_path = repo.path();

        let output = std::process::Command::new("git")
            .args(["log", "--pretty=%B", &range])
            .current_dir(repo_path)
            .output()
            .map_err(|e| CliError::Generic(format!("Failed to get commit log: {}", e)))?;

        String::from_utf8(output.stdout)
            .map_err(|e| CliError::Generic(format!("Invalid UTF-8 in commit log: {}", e)))
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
        config: &RepositoryConfig,
    ) -> Result<Vec<String>, CliError> {
        let mut packages = std::collections::HashSet::new();

        // Extract scopes from commit messages (pattern: type(scope): message)
        let scope_regex = Regex::new(r"^[a-z]+\(([^)]+)\):")
            .map_err(|e| CliError::Generic(format!("Regex error: {}", e)))?;

        for line in commit_log.lines() {
            if let Some(caps) = scope_regex.captures(line) {
                let scope = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                // Check if scope matches a package
                if config.packages.iter().any(|p| p.name == scope) {
                    packages.insert(scope.to_string());
                }
            }
        }

        Ok(packages.into_iter().collect())
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
    ) -> Result<(), CliError> {
        let strategy = &config.versioning.strategy;

        match strategy {
            VersioningStrategy::Unified => {
                // Single tag for all packages
                if let Some(update) = updates.first() {
                    let tag_name = format!("v{}", update.new_version);
                    self.create_and_push_tag(repo, &tag_name)?;
                    info!("📌 Created unified tag: {}", tag_name);
                }
            }
            VersioningStrategy::Independent | VersioningStrategy::Hybrid => {
                // Per-package tags
                for update in updates {
                    let tag_name = format!("{}-v{}", update.package_name, update.new_version);
                    self.create_and_push_tag(repo, &tag_name)?;
                    info!("📌 Created tag: {}", tag_name);
                }
            }
        }

        Ok(())
    }

    /// Create a git tag and optionally push to remote
    fn create_and_push_tag(&self, repo: &git2::Repository, tag_name: &str) -> Result<(), CliError> {
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        self.run_git(
            repo_path,
            &["tag", "-a", tag_name, "-m", tag_name],
            "create tag",
        )?;

        info!("✅ Tag '{}' created locally", tag_name);

        // Try to push to remote
        if self.tag_options.will_publish_remote() {
            if let Err(e) = self.push_tag_to_remote(repo, tag_name) {
                debug!("Failed to push tag to remote: {}", e);
            } else {
                info!("📤 Tag '{}' pushed to remote", tag_name);
            }
        }

        Ok(())
    }

    /// Push tag to remote repository
    fn push_tag_to_remote(&self, repo: &git2::Repository, tag_name: &str) -> Result<(), CliError> {
        repo.find_remote("origin").map_err(CliError::from)?;
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        self.run_git(
            repo_path,
            &["push", "origin", &format!("refs/tags/{tag_name}")],
            "push tag to remote",
        )
    }

    fn run_git(&self, repo_path: &Path, args: &[&str], action: &str) -> Result<(), CliError> {
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .map_err(CliError::IoError)?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("git {:?} failed", args)
        } else {
            stderr
        };
        Err(CliError::Generic(format!("Failed to {action}: {detail}")))
    }
}
