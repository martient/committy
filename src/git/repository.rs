use crate::error::CliError;
use git2::{Config, Repository, StatusOptions, StatusShow};
use std::env;

pub fn discover_repository() -> Result<Repository, CliError> {
    let current_dir = env::current_dir()?;
    log::debug!("Starting repository discovery from: {current_dir:?}");

    match Repository::discover(&current_dir) {
        Ok(repo) => {
            // Get the absolute path to the repository root
            let repo_path = repo
                .path()
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .ok_or_else(|| {
                    CliError::GitError(git2::Error::from_str(
                        "Could not determine repository root directory",
                    ))
                })?;

            // Open a new repository instance using the absolute path
            match Repository::open(&repo_path) {
                Ok(new_repo) => Ok(new_repo),
                Err(e) => {
                    log::error!("Failed to open repository at {repo_path:?}: {e}");
                    Err(CliError::GitError(e))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to discover repository from {current_dir:?}: {e}");
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

/// List changed files in the repository. If `include_unstaged` is true,
/// include workdir modifications in addition to index changes.
pub fn list_changed_files(include_unstaged: bool) -> Result<Vec<String>, CliError> {
    let repo = discover_repository()?;
    let mut opts = StatusOptions::new();
    opts.include_ignored(false)
        .include_untracked(true)
        .include_unmodified(false)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true)
        .show(if include_unstaged {
            StatusShow::IndexAndWorkdir
        } else {
            StatusShow::Index
        });

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            set.insert(path.to_string());
        }
        // For renames, path may be None; try head_to_index or index_to_workdir
        if let Some(delta) = entry.head_to_index().or_else(|| entry.index_to_workdir()) {
            if let Some(new_file) = delta.new_file().path() {
                set.insert(new_file.to_string_lossy().to_string());
            }
            if let Some(old_file) = delta.old_file().path() {
                set.insert(old_file.to_string_lossy().to_string());
            }
        }
    }

    Ok(set.into_iter().collect())
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
