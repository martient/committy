use serde_json::Value;
use std::collections::HashMap;

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::RepositoryConfig;
use crate::error::CliError;
use crate::git;
use crate::input;
use crate::input::validation::{auto_correct_scope, suggest_commit_type};
use crate::scope::detector::ScopeDetector;
use crate::telemetry;
use crate::workflow::orchestrator::WorkflowOrchestrator;
use log::{debug, info, warn};
use structopt::StructOpt;

#[derive(Debug, StructOpt, Default)]
pub struct CommitCommand {
    #[structopt(long = "type", help = "Type of commit (e.g., feat, fix, docs)")]
    commit_type: Option<String>,

    #[structopt(long, help = "Scope of the commit")]
    scope: Option<String>,

    #[structopt(long, help = "Short commit message")]
    message: Option<String>,

    #[structopt(long, help = "Long/detailed commit message")]
    long_message: Option<String>,

    #[structopt(long, help = "Mark this as a breaking change")]
    breaking_change: bool,

    #[structopt(long, help = "Amend the previous commit")]
    amend: bool,
}

impl Command for CommitCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        // Validate git configuration first
        git::validate_git_config()?;

        if !git::has_staged_changes()? {
            return Err(CliError::NoStagedChanges);
        }

        // In non-interactive mode (from the command root), all required fields must be provided
        if non_interactive {
            debug!("Running in non-interactive mode");
            if self.commit_type.is_none() || self.message.is_none() {
                return Err(CliError::InputError(
                    "In non-interactive mode, --type and --message are required".to_string(),
                ));
            }
        }

        // Detect scopes from staged files early (for multi-package repositories)
        let (detected_scopes, available_scopes, allow_multiple_scopes, require_scope) =
            self.detect_scopes_from_config()?;

        // Handle commit type with auto-correction
        let commit_type = if let Some(commit_type) = &self.commit_type {
            if let Some(suggested) = suggest_commit_type(commit_type) {
                if suggested != commit_type {
                    info!("Auto-correcting commit type from '{commit_type}' to '{suggested}'");
                    debug!("Auto-corrected commit type from '{commit_type}' to '{suggested}'");
                }
                suggested.to_string()
            } else {
                return Err(CliError::InputError(format!(
                    "Invalid commit type '{}'. Valid types are: {}",
                    commit_type,
                    crate::config::COMMIT_TYPES.join(", ")
                )));
            }
        } else {
            input::select_commit_type()?
        };

        // Handle breaking change
        let breaking_change = if self.breaking_change {
            debug!("Breaking change flag is set");
            true
        } else if !non_interactive {
            input::confirm_breaking_change()?
        } else {
            false
        };

        // Handle scope with auto-detection and correction
        let scope = if let Some(scope) = &self.scope {
            // CLI flag provided - use it
            if !non_interactive {
                // In interactive mode, validate and potentially correct the scope
                input::validate_scope_input(scope)?
            } else {
                // In non-interactive mode, apply corrections automatically
                let corrected = auto_correct_scope(scope);
                if corrected != *scope {
                    info!("Auto-correcting scope from '{scope}' to '{corrected}'");
                }
                corrected
            }
        } else if !non_interactive {
            // Interactive mode - use smart scope detection
            input::select_detected_scopes(
                &detected_scopes,
                &available_scopes,
                allow_multiple_scopes,
            )?
        } else if detected_scopes.len() == 1 {
            // Non-interactive mode - auto-select single detected scope
            detected_scopes[0].clone()
        } else if detected_scopes.is_empty() && require_scope {
            // Multi-package repo requires scope but none detected
            return Err(CliError::InputError(
                "Scope is required for multi-package repository but could not be auto-detected. Please provide --scope flag.".to_string(),
            ));
        } else {
            String::new()
        };

        // Handle messages
        let short_message = match &self.message {
            Some(msg) if !msg.is_empty() => msg.clone(),
            _ if !non_interactive => input::input_short_message()?,
            _ => {
                return Err(CliError::InputError(
                    "Short message is required".to_string(),
                ))
            }
        };

        let long_message = match &self.long_message {
            Some(msg) => msg.clone(),
            None if !non_interactive => input::input_long_message()?,
            None => String::new(),
        };

        let full_message = git::format_commit_message(
            &commit_type,
            breaking_change,
            &scope,
            &short_message,
            &long_message,
        );

        debug!("Formatted commit message: {full_message}");

        // Run workflow orchestrator for multi-package repositories
        self.run_workflow_if_configured(&full_message)?;

        git::commit_changes(&full_message, self.amend)?;
        // fire off telemetry without making this function async
        if let Err(e) =
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(telemetry::posthog::publish_event(
                    "commit_created",
                    HashMap::from([
                        ("commit_type", Value::from(commit_type.as_str())),
                        (
                            "is_breaking_change",
                            Value::from(breaking_change.to_string()),
                        ),
                        ("as_scope", Value::from((!scope.is_empty()).to_string())),
                        ("len_scope", Value::from(scope.len())),
                        (
                            "as_short_message",
                            Value::from((!short_message.is_empty()).to_string()),
                        ),
                        ("len_short_message", Value::from(short_message.len())),
                        (
                            "as_long_message",
                            Value::from((!long_message.is_empty()).to_string()),
                        ),
                        ("len_long_message", Value::from(long_message.len())),
                    ]),
                ))
        {
            debug!("Telemetry error: {e:?}");
        }
        info!("Changes committed successfully! 🎉");
        Ok(())
    }
}

impl CommitCommand {
    /// Detect scopes from staged files and load available scopes from config
    fn detect_scopes_from_config(
        &self,
    ) -> Result<(Vec<String>, Vec<String>, bool, bool), CliError> {
        // Get current directory for config loading
        let current_dir = std::env::current_dir()
            .map_err(|e| CliError::InputError(format!("Failed to get current dir: {}", e)))?;

        // Try to load merged config
        let merged_config = match MergedConfig::load(&current_dir) {
            Ok(config) => config,
            Err(_) => {
                // No config - return empty detected scopes
                debug!("No repository config found for scope detection");
                return Ok((vec![], vec![], false, false));
            }
        };

        // Get repository config if it exists
        if let Some(repo_config) = &merged_config.repository {
            // Check if multi-package mode and auto_detect is enabled
            if !repo_config.scopes.auto_detect {
                debug!("Scope auto-detection disabled in config");
                return Ok((vec![], vec![], false, false));
            }

            debug!("Attempting to auto-detect scopes from staged files");

            // Create scope detector
            let detector = ScopeDetector::new(repo_config.clone(), &current_dir);

            // Detect scopes from staged files
            let detected_scopes = detector.detect_from_staged().unwrap_or_else(|e| {
                warn!("Scope detection failed: {}", e);
                vec![]
            });

            // Get available scopes
            let available_scopes = detector.suggest_scopes().unwrap_or_else(|e| {
                warn!("Failed to get scope suggestions: {}", e);
                vec![]
            });

            let allow_multiple = repo_config.scopes.allow_multiple_scopes;
            let require_scope = repo_config.scopes.require_scope_for_multi_package;

            if !detected_scopes.is_empty() {
                info!(
                    "Auto-detected {} scope(s): {}",
                    detected_scopes.len(),
                    detected_scopes.join(", ")
                );
            }

            Ok((
                detected_scopes,
                available_scopes,
                allow_multiple,
                require_scope,
            ))
        } else {
            // No repository config
            Ok((vec![], vec![], false, false))
        }
    }

    /// Run workflow orchestrator if repository config exists
    fn run_workflow_if_configured(&self, commit_message: &str) -> Result<(), CliError> {
        // Get repository root
        let repo = git::discover_repository()?;
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;

        // Try to load repository config
        let config = match RepositoryConfig::try_load(repo_path) {
            Ok(Some(config)) => config,
            Ok(None) => {
                debug!("No repository config found, skipping workflow orchestrator");
                return Ok(());
            }
            Err(e) => {
                warn!("Failed to load repository config: {}", e);
                return Ok(());
            }
        };

        debug!("Repository config loaded, running workflow orchestrator");

        // Create and configure orchestrator
        let orchestrator = WorkflowOrchestrator::new(repo_path, config);

        // Run workflow (detect scopes, calculate updates, apply changes)
        match orchestrator.run_workflow(commit_message, true) {
            Ok(result) => {
                if !result.scopes.is_empty() {
                    info!("Workflow detected scopes: {:?}", result.scopes);
                }
                if result.has_changes() {
                    info!(
                        "Applied {} version update(s) and {} dependency update(s)",
                        result.version_updates.len(),
                        result.dependency_updates.len()
                    );

                    // Stage the updated files
                    for file in &result.modified_files {
                        if let Err(e) = git::stage_file(file) {
                            warn!("Failed to stage {}: {}", file.display(), e);
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                warn!("Workflow orchestrator failed: {}", e);
                // Don't fail the commit if workflow fails
                Ok(())
            }
        }
    }
}
