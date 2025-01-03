use crate::error::CliError;
use git2::{Config, Repository};

pub fn has_staged_changes() -> Result<bool, CliError> {
    let repo = Repository::open(".")?;
    let statuses = repo.statuses(None)?;
    Ok(statuses
        .iter()
        .any(|s| s.status().is_index_new() || s.status().is_index_modified()))
}

fn get_config_value(config: &Config, key: &str) -> Option<String> {
    match config.get_string(key) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

pub fn validate_git_config() -> Result<(), CliError> {
    let repo = Repository::open(".")?;
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
