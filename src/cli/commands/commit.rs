use serde_json::Value;
use std::collections::HashMap;

use crate::cli::Command;
use crate::error::CliError;
use crate::git;
use crate::input;
use crate::input::validation::{auto_correct_scope, suggest_commit_type};
use crate::telemetry;
use log::{debug, info};
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

        // Handle scope with auto-correction
        let scope = if let Some(scope) = &self.scope {
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
            input::input_scope()?
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
