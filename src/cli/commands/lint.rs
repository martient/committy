use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::output::{MachineContext, API_VERSION};
use crate::cli::Command;
use crate::error::CliError;
use crate::linter::{check_message_format_for_repo, CommitIssue, CommitLinter};

#[derive(Debug, StructOpt)]
pub struct LintCommand {
    /// Path to the git repository (defaults to current directory)
    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,

    /// Output format: text or json
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    /// Check one commit message string instead of repository history
    #[structopt(long)]
    message: Option<String>,

    /// Check a commit message file instead of repository history
    #[structopt(long)]
    file: Option<String>,

    /// Validate a git rev range like v1.0.0..HEAD
    #[structopt(long = "rev-range")]
    rev_range: Option<String>,

    /// Validate commits after this ref
    #[structopt(long = "from-ref")]
    from_ref: Option<String>,

    /// Validate commits up to this ref
    #[structopt(long = "to-ref")]
    to_ref: Option<String>,

    /// Treat an empty or comment-only commit message as a valid aborted commit
    #[structopt(long = "allow-abort")]
    allow_abort: bool,
}

impl Command for LintCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        if self.message.is_some() || self.file.is_some() {
            return self.execute_message_mode();
        }

        let linter = CommitLinter::new(
            self.repo_path
                .to_str()
                .ok_or_else(|| CliError::InputError("Invalid repo path".to_string()))?,
        )
        .map_err(|e| CliError::Generic(e.to_string()))?;

        let (from_ref, to_ref) = if let Some(rev_range) = &self.rev_range {
            let (from, to) = rev_range
                .split_once("..")
                .ok_or_else(|| CliError::InputError("Expected rev range like a..b".to_string()))?;
            (Some(from.to_string()), Some(to.to_string()))
        } else {
            (self.from_ref.clone(), self.to_ref.clone())
        };

        let issues = if from_ref.is_none() && to_ref.is_none() {
            linter
                .check_commits_since_last_tag()
                .map_err(|e| CliError::Generic(e.to_string()))?
        } else {
            linter
                .check_rev_range(from_ref.as_deref(), to_ref.as_deref())
                .map_err(|e| CliError::Generic(e.to_string()))?
        };
        print_history_output(&self.output, &issues)?;
        if issues.is_empty() {
            Ok(())
        } else {
            Err(CliError::LintIssues(issues.len()))
        }
    }

    fn machine_context(&self) -> Option<MachineContext> {
        (self.output == "json").then_some(MachineContext {
            command: "lint",
            dry_run: false,
        })
    }
}

impl LintCommand {
    fn execute_message_mode(&self) -> Result<(), CliError> {
        let mut message = if let Some(message) = &self.message {
            message.clone()
        } else if let Some(file) = &self.file {
            fs::read_to_string(file).map_err(|e| CliError::Generic(e.to_string()))?
        } else {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| CliError::Generic(e.to_string()))?;
            buffer
        };

        if self.allow_abort {
            message = strip_comment_lines(&message);
            if message.trim().is_empty() {
                print_message_output(&self.output, &[])?;
                return Ok(());
            }
        }

        let issues = check_message_format_for_repo(&self.repo_path, &message)
            .map_err(|e| CliError::Generic(e.to_string()))?;
        print_message_output(&self.output, &issues)?;
        if issues.is_empty() {
            Ok(())
        } else {
            Err(CliError::LintIssues(issues.len()))
        }
    }
}

fn strip_comment_lines(message: &str) -> String {
    message
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_history_output(output: &str, issues: &[CommitIssue]) -> Result<(), CliError> {
    if output == "json" {
        #[derive(Serialize)]
        struct LintOutput<'a> {
            api_version: u8,
            command: &'static str,
            ok: bool,
            dry_run: bool,
            count: usize,
            issues: &'a [CommitIssue],
            errors: Option<Vec<String>>,
        }
        let payload = LintOutput {
            api_version: API_VERSION,
            command: "lint",
            ok: issues.is_empty(),
            dry_run: false,
            count: issues.len(),
            issues,
            errors: None,
        };
        println!(
            "{}",
            serde_json::to_string(&payload).map_err(|e| CliError::Generic(e.to_string()))?
        );
    } else if issues.is_empty() {
        println!("✅ All commits passed conventional commit validation");
    } else {
        println!("❌ Found {} commit(s) with issues:", issues.len());
        for issue in issues {
            println!("\nCommit: {}", issue.commit_id);
            println!("Message: {}", issue.message);
            println!("Issue: {}", issue.issue);
        }
    }
    Ok(())
}

fn print_message_output(output: &str, issues: &[String]) -> Result<(), CliError> {
    if output == "json" {
        #[derive(Serialize)]
        struct LintMessageOutput<'a> {
            api_version: u8,
            command: &'static str,
            ok: bool,
            dry_run: bool,
            count: usize,
            issues: &'a [String],
            errors: Option<Vec<String>>,
        }
        let payload = LintMessageOutput {
            api_version: API_VERSION,
            command: "lint",
            ok: issues.is_empty(),
            dry_run: false,
            count: issues.len(),
            issues,
            errors: None,
        };
        println!(
            "{}",
            serde_json::to_string(&payload).map_err(|e| CliError::Generic(e.to_string()))?
        );
    } else if issues.is_empty() {
        println!("✅ Commit message is valid!");
    } else {
        println!("❌ Found {} issue(s):", issues.len());
        for issue in issues {
            println!("- {issue}");
        }
    }
    Ok(())
}
