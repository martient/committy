use crate::error::CliError;
use git2::{Config, Repository, StatusOptions, StatusShow};
use std::env;
use log;

pub fn discover_repository() -> Result<Repository, CliError> {
    let current_dir = env::current_dir()?;
    log::debug!("Starting repository discovery from: {:?}", current_dir);
    
    match Repository::discover(&current_dir) {
        Ok(repo) => Ok(repo),
        Err(e) => {
            log::error!("Failed to discover repository from {:?}: {}", current_dir, e);
            Err(CliError::GitError(git2::Error::from_str(
                "Could not find Git repository in current directory or any parent directories",
            )))
        }
    }
}

pub fn has_staged_changes() -> Result<bool, CliError> {
    let repo = discover_repository()?;
    let mut opts = StatusOptions::new();
    opts.include_ignored(false)
        .include_untracked(false)
        .include_unmodified(false)
        .exclude_submodules(true)
        .show(StatusShow::Index);

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

    // If we get here, there are no staged changes
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
