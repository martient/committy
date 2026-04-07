use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::RepositoryConfig;
use crate::convention::Convention;
use crate::error::CliError;
use crate::git;
use crate::input;
use crate::input::validation::{auto_correct_scope, suggest_commit_type};
use crate::linter::{allowed_commit_types_for_repo, check_message_format_for_repo};
use crate::scope::detector::ScopeDetector;
use crate::telemetry;
use crate::workflow::orchestrator::WorkflowOrchestrator;
use log::{debug, info, warn};
use serde::Serialize;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct CommitCommand {
    #[structopt(long = "type", help = "Type of commit (e.g., feat, fix, docs)")]
    pub(crate) commit_type: Option<String>,

    #[structopt(long, help = "Scope of the commit")]
    pub(crate) scope: Option<String>,

    #[structopt(long, help = "Short commit message")]
    pub(crate) message: Option<String>,

    #[structopt(long, help = "Long/detailed commit message")]
    pub(crate) long_message: Option<String>,

    #[structopt(
        long = "message-file",
        parse(from_os_str),
        help = "Read the full commit message from a file"
    )]
    pub(crate) message_file: Option<PathBuf>,

    #[structopt(long, help = "Mark this as a breaking change")]
    pub(crate) breaking_change: bool,

    #[structopt(long, help = "Amend the previous commit")]
    pub(crate) amend: bool,

    #[structopt(long, help = "Preview the commit without writing to git")]
    pub(crate) dry_run: bool,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    pub(crate) output: String,

    #[structopt(
        long = "git-config",
        help = "Pass through git -c key=value overrides (repeatable)"
    )]
    pub(crate) git_config: Vec<String>,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    pub(crate) repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct CommitCommandOutput {
    command: String,
    ok: bool,
    dry_run: bool,
    message: String,
    commit_type: String,
    scope: String,
    breaking_change: bool,
    errors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow: Option<crate::workflow::orchestrator::WorkflowResult>,
}

impl Command for CommitCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        self.execute_with_command_name(non_interactive, "commit")
    }
}

impl CommitCommand {
    pub(crate) fn default_interactive() -> Self {
        Self {
            commit_type: None,
            scope: None,
            message: None,
            long_message: None,
            message_file: None,
            breaking_change: false,
            amend: false,
            dry_run: false,
            output: "text".to_string(),
            git_config: vec![],
            repo_path: PathBuf::from("."),
        }
    }

    pub(crate) fn execute_with_command_name(
        &self,
        non_interactive: bool,
        command_name: &str,
    ) -> Result<(), CliError> {
        let repo = git::discover_repository_from(&self.repo_path)?;
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        let git_command_config = git::resolve_git_command_config(repo_path, &self.git_config)?;
        let convention = Convention::load(repo_path)?;
        let has_staged_changes = git::has_staged_changes_from(repo_path)?;
        if !self.amend && !has_staged_changes {
            return Err(CliError::NoStagedChanges);
        }
        let allowed_types = allowed_commit_types_for_repo(repo_path)
            .map_err(|e| CliError::Generic(e.to_string()))?;

        // In non-interactive mode (from the command root), all required fields must be provided
        if non_interactive {
            debug!("Running in non-interactive mode");
            if self.message_file.is_none() && (self.commit_type.is_none() || self.message.is_none())
            {
                return Err(CliError::InputError(
                    "In non-interactive mode, --type and --message are required".to_string(),
                ));
            }
        }

        if let Some(message_file) = &self.message_file {
            let full_message = std::fs::read_to_string(message_file).map_err(CliError::IoError)?;
            let metadata = message_metadata(&convention, &full_message);
            return self.finish_commit(
                repo_path,
                command_name,
                &git_command_config,
                full_message,
                metadata.commit_type,
                metadata.scope,
                metadata.breaking_change,
                metadata.short_message,
                metadata.long_message,
            );
        }

        // Detect scopes from staged files early (for multi-package repositories)
        let (detected_scopes, available_scopes, allow_multiple_scopes, require_scope) =
            self.detect_scopes_from_config(repo_path, has_staged_changes)?;

        // Handle commit type with auto-correction
        let commit_type = if let Some(commit_type) = &self.commit_type {
            if let Some(suggested) = suggest_commit_type(commit_type) {
                if allowed_types.iter().any(|allowed| allowed == suggested)
                    && suggested != commit_type
                {
                    info!("Auto-correcting commit type from '{commit_type}' to '{suggested}'");
                    debug!("Auto-corrected commit type from '{commit_type}' to '{suggested}'");
                    suggested.to_string()
                } else if allowed_types.iter().any(|allowed| allowed == commit_type) {
                    commit_type.clone()
                } else if allowed_types.iter().any(|allowed| allowed == suggested) {
                    suggested.to_string()
                } else {
                    return Err(CliError::InputError(format!(
                        "Invalid commit type '{}'. Valid types are: {}",
                        commit_type,
                        allowed_types.join(", ")
                    )));
                }
            } else if allowed_types.iter().any(|allowed| allowed == commit_type) {
                commit_type.clone()
            } else {
                return Err(CliError::InputError(format!(
                    "Invalid commit type '{}'. Valid types are: {}",
                    commit_type,
                    allowed_types.join(", ")
                )));
            }
        } else {
            input::select_commit_type_from(&allowed_types)?
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

        let full_message = convention.render_message(
            &commit_type,
            &scope,
            &short_message,
            &long_message,
            breaking_change,
        )?;

        self.finish_commit(
            repo_path,
            command_name,
            &git_command_config,
            full_message,
            commit_type,
            scope,
            breaking_change,
            short_message,
            long_message,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_commit(
        &self,
        repo_path: &Path,
        command_name: &str,
        git_command_config: &git::GitCommandConfig,
        full_message: String,
        commit_type: String,
        scope: String,
        breaking_change: bool,
        short_message: String,
        long_message: String,
    ) -> Result<(), CliError> {
        debug!("Formatted commit message: {full_message}");

        let validation_issues = check_message_format_for_repo(repo_path, &full_message)
            .map_err(|e| CliError::Generic(e.to_string()))?;
        if !validation_issues.is_empty() {
            let output = CommitCommandOutput {
                command: command_name.into(),
                ok: false,
                dry_run: self.dry_run,
                message: full_message,
                commit_type,
                scope,
                breaking_change,
                errors: Some(validation_issues.clone()),
                workflow: None,
            };
            self.print_output(&output, command_name);
            return Err(CliError::LintIssues(validation_issues.len()));
        }

        if !self.dry_run {
            // Validate git configuration before mutating git state
            git::validate_git_config_from(repo_path)?;
        }

        let workflow =
            self.execute_workflow_if_configured(repo_path, &full_message, !self.dry_run)?;

        let output = CommitCommandOutput {
            command: command_name.into(),
            ok: true,
            dry_run: self.dry_run,
            message: full_message.clone(),
            commit_type: commit_type.clone(),
            scope: scope.clone(),
            breaking_change,
            errors: None,
            workflow,
        };

        if self.dry_run {
            self.print_output(&output, command_name);
            return Ok(());
        }

        git::commit_changes_in_with_config(
            repo_path,
            &full_message,
            self.amend,
            git_command_config,
        )?;
        self.print_output(&output, command_name);
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
        if self.output != "json" {
            if self.amend {
                info!("Previous commit amended successfully! 🎉");
            } else {
                info!("Changes committed successfully! 🎉");
            }
        }
        Ok(())
    }
    /// Detect scopes from staged files and load available scopes from config
    fn detect_scopes_from_config(
        &self,
        repo_path: &Path,
        has_staged_changes: bool,
    ) -> Result<(Vec<String>, Vec<String>, bool, bool), CliError> {
        // Try to load merged config
        let merged_config = match MergedConfig::load(repo_path) {
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
            let detector = ScopeDetector::new(repo_config.clone(), repo_path);

            // Detect scopes from staged files
            let detected_scopes = if has_staged_changes {
                detector.detect_from_staged().unwrap_or_else(|e| {
                    warn!("Scope detection failed: {}", e);
                    vec![]
                })
            } else {
                vec![]
            };

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
    fn execute_workflow_if_configured(
        &self,
        repo_path: &Path,
        commit_message: &str,
        apply_changes: bool,
    ) -> Result<Option<crate::workflow::orchestrator::WorkflowResult>, CliError> {
        // Try to load repository config
        let config = match RepositoryConfig::try_load(repo_path) {
            Ok(Some(config)) => config,
            Ok(None) => {
                debug!("No repository config found, skipping workflow orchestrator");
                return Ok(None);
            }
            Err(e) => {
                warn!("Failed to load repository config: {}", e);
                return Ok(None);
            }
        };

        debug!("Repository config loaded, running workflow orchestrator");

        // Create and configure orchestrator
        let orchestrator = WorkflowOrchestrator::new(repo_path, config);

        // Run workflow (detect scopes, calculate updates, apply changes)
        match orchestrator.run_workflow(commit_message, apply_changes) {
            Ok(result) => {
                if !result.scopes.is_empty() {
                    info!("Workflow detected scopes: {:?}", result.scopes);
                }
                if apply_changes && result.has_changes() {
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
                Ok(Some(result))
            }
            Err(e) => {
                warn!("Workflow orchestrator failed: {}", e);
                // Don't fail the commit if workflow fails
                Ok(None)
            }
        }
    }

    fn print_output(&self, output: &CommitCommandOutput, command_name: &str) {
        if self.output == "json" {
            println!("{}", serde_json::to_string(output).unwrap());
            return;
        }

        if let Some(errors) = &output.errors {
            if command_name == "amend" {
                println!("Amend message validation failed:");
            } else {
                println!("Commit message validation failed:");
            }
            for issue in errors {
                println!("- {issue}");
            }
            println!();
            println!("{}", output.message);
            return;
        }

        if output.dry_run {
            if command_name == "amend" {
                println!("Would amend commit to:\n\n{}", output.message);
            } else {
                println!("Would create commit:\n\n{}", output.message);
            }
            if let Some(workflow) = &output.workflow {
                if !workflow.scopes.is_empty() {
                    println!("Detected scopes: {}", workflow.scopes.join(", "));
                }
                if workflow.has_changes() {
                    println!(
                        "Would apply {} version update(s) and {} dependency update(s)",
                        workflow.version_updates.len(),
                        workflow.dependency_updates.len()
                    );
                }
            }
            return;
        }

        if command_name == "amend" {
            println!("Commit amended successfully!");
        }
    }
}

impl Default for CommitCommand {
    fn default() -> Self {
        Self::default_interactive()
    }
}

struct CommitMessageMetadata {
    commit_type: String,
    scope: String,
    breaking_change: bool,
    short_message: String,
    long_message: String,
}

fn message_metadata(convention: &Convention, full_message: &str) -> CommitMessageMetadata {
    if let Some(parsed) = convention.parse_message(full_message) {
        return CommitMessageMetadata {
            commit_type: parsed.commit_type,
            scope: parsed.scope,
            breaking_change: parsed.breaking_change || full_message.contains("BREAKING CHANGE:"),
            short_message: parsed.description,
            long_message: parsed.body,
        };
    }

    let trimmed = full_message.trim();
    let short_message = trimmed.lines().next().unwrap_or("").to_string();
    let long_message = trimmed
        .split_once("\n\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();

    CommitMessageMetadata {
        commit_type: String::new(),
        scope: String::new(),
        breaking_change: full_message.contains("BREAKING CHANGE:"),
        short_message,
        long_message,
    }
}

#[cfg(test)]
mod tests {
    use super::{message_metadata, CommitCommand};
    use crate::config::convention::ConventionConfig;
    use crate::convention::Convention;
    use std::path::PathBuf;

    #[test]
    fn default_interactive_commit_command_uses_current_repo_and_text_output() {
        let command = CommitCommand::default_interactive();

        assert_eq!(command.output, "text");
        assert_eq!(command.repo_path, PathBuf::from("."));
        assert!(command.git_config.is_empty());
        assert!(!command.amend);
        assert!(!command.dry_run);
    }

    #[test]
    fn message_metadata_uses_convention_parsing_when_available() {
        let convention = Convention::from_config(ConventionConfig::default()).unwrap();
        let metadata = message_metadata(&convention, "feat(cli)!: add command\n\nLong body");

        assert_eq!(metadata.commit_type, "feat");
        assert_eq!(metadata.scope, "cli");
        assert!(metadata.breaking_change);
        assert_eq!(metadata.short_message, "add command");
        assert_eq!(metadata.long_message, "Long body");
    }
}
