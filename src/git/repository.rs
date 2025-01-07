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
        .include_untracked(true)
        .include_unmodified(false)
        .exclude_submodules(true)
        .show(StatusShow::Index);

    let statuses = repo.statuses(Some(&mut opts))?;
    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new()
            || status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
        {
            return Ok(true);
        }
    }

    // If no status changes found and HEAD exists, check the index against HEAD
    if let Ok(head) = repo.head() {
        if let Ok(head) = head.peel_to_commit() {
            let head_tree = head.tree()?;
            let index = repo.index()?;

            // Compare index with HEAD tree
            for entry in index.iter() {
                let path = std::str::from_utf8(&entry.path).unwrap_or("");
                if let Ok(tree_entry) = head_tree.get_path(std::path::Path::new(path)) {
                    // Entry exists in HEAD, check if it's different in the index
                    if tree_entry.id() != entry.id {
                        return Ok(true);
                    }
                } else {
                    // Entry doesn't exist in HEAD, it's new
                    return Ok(true);
                }
            }

            // Check for deletions by comparing HEAD tree with index
            for entry in head_tree.iter() {
                let path = entry.name().unwrap_or("");
                if index.get_path(std::path::Path::new(path), 0).is_none() {
                    return Ok(true);
                }
            }
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
