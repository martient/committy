use crate::error::CliError;
use git2::{Config, Repository, StatusOptions, StatusShow};
use std::env;

fn discover_repository() -> Result<Repository, CliError> {
    let current_dir = env::current_dir()?;
    Repository::discover(&current_dir).map_err(CliError::GitError)
}

pub fn has_staged_changes() -> Result<bool, CliError> {
    let repo = discover_repository()?;
    let mut opts = StatusOptions::new();
    opts.include_ignored(false)
        .include_untracked(false)
        .include_unmodified(false)
        .exclude_submodules(true)
        .show(StatusShow::Index);  // Only show index changes

    let statuses = repo.statuses(Some(&mut opts))?;

    // Check for any index changes (including deletions)
    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new()
            || status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
            || status.is_index_typechange()
        {
            return Ok(true);
        }
    }

    // If HEAD exists, check for staged deletions
    if let Ok(head) = repo.head() {
        if let Ok(head_commit) = head.peel_to_commit() {
            let head_tree = head_commit.tree()?;
            let index = repo.index()?;

            // Compare HEAD tree with index
            for entry in head_tree.iter() {
                if let Some(name) = entry.name() {
                    if index.get_path(std::path::Path::new(name), 0).is_none() {
                        return Ok(true);  // File exists in HEAD but not in index = staged deletion
                    }
                }
            }
        }
    } else {
        // No HEAD yet, check if there are any entries in the index
        let index = repo.index()?;
        if index.len() > 0 {
            return Ok(true);  // New repository with staged files
        }
    }

    Ok(false)
}

fn get_config_value(config: &Config, key: &str) -> Option<String> {
    match config.get_string(key) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

pub fn validate_git_config() -> Result<(), CliError> {
    let repo = discover_repository()?;
    let config = repo.config()?;

    // Try to get user.name from local or global config
    let name = get_config_value(&config, "user.name");
    if name.is_none() {
        return Err(CliError::GitConfigError("user.name is not set".to_string()));
    }

    // Try to get user.email from local or global config
    let email = get_config_value(&config, "user.email");
    if email.is_none() {
        return Err(CliError::GitConfigError(
            "user.email is not set".to_string(),
        ));
    }

    Ok(())
}
