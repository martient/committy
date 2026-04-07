use super::process::{run_git_with_input, GitCommandConfig};
use super::repository::{discover_repository, discover_repository_from};
use crate::error::CliError;
use std::path::Path;

/// Stage a file for commit
pub fn stage_file(file_path: &Path) -> Result<(), CliError> {
    let repo = if file_path.is_absolute() {
        discover_repository_from(file_path)?
    } else {
        discover_repository()?
    };
    let mut index = repo.index()?;

    // Convert absolute path to relative path from repo root
    let repo_path = repo
        .workdir()
        .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
    let relative_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);

    index.add_path(relative_path)?;
    index.write()?;
    Ok(())
}

#[allow(dead_code)]
pub fn commit_changes(message: &str, amend: bool) -> Result<(), CliError> {
    let repo = discover_repository()?;
    commit_changes_in_repo(&repo, message, amend, &GitCommandConfig::default())
}

#[allow(dead_code)]
pub fn commit_changes_in(path: &Path, message: &str, amend: bool) -> Result<(), CliError> {
    commit_changes_in_with_config(path, message, amend, &GitCommandConfig::default())
}

#[allow(dead_code)]
pub fn commit_changes_with_config(
    message: &str,
    amend: bool,
    git_command_config: &GitCommandConfig,
) -> Result<(), CliError> {
    let repo = discover_repository()?;
    commit_changes_in_repo(&repo, message, amend, git_command_config)
}

pub fn commit_changes_in_with_config(
    path: &Path,
    message: &str,
    amend: bool,
    git_command_config: &GitCommandConfig,
) -> Result<(), CliError> {
    let repo = discover_repository_from(path)?;
    commit_changes_in_repo(&repo, message, amend, git_command_config)
}

fn commit_changes_in_repo(
    repo: &git2::Repository,
    message: &str,
    amend: bool,
    git_command_config: &GitCommandConfig,
) -> Result<(), CliError> {
    let repo_path = repo
        .workdir()
        .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
    let mut args = vec!["commit", "-F", "-"];
    if amend {
        args.push("--amend");
    }

    run_git_with_input(
        repo_path,
        &args,
        message,
        "create commit",
        git_command_config,
    )?;

    Ok(())
}

pub fn format_commit_message(
    commit_type: &str,
    breaking_change: bool,
    scope: &str,
    short_message: &str,
    long_message: &str,
) -> String {
    let bang = if breaking_change { "!" } else { "" };
    let mut full_message = if scope.is_empty() {
        format!("{commit_type}{bang}: {short_message}")
    } else {
        format!("{commit_type}({scope}){bang}: {short_message}")
    };

    if !long_message.is_empty() {
        full_message = format!("{full_message}\n\n{long_message}");
    }

    full_message
}
