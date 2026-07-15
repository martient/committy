use crate::config::git::validate_git_config_overrides;
use crate::config::hierarchy::MergedConfig;
use crate::error::CliError;
use std::path::Path;
use std::process::{Command, Output, Stdio};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitCommandConfig {
    config_overrides: Vec<String>,
}

impl GitCommandConfig {
    pub fn new(config_overrides: Vec<String>) -> Result<Self, CliError> {
        validate_git_config_overrides(&config_overrides)
            .map_err(|e| CliError::InputError(e.to_string()))?;
        Ok(Self { config_overrides })
    }

    pub fn config_overrides(&self) -> &[String] {
        &self.config_overrides
    }
}

pub fn resolve_git_command_config(
    repo_path: &Path,
    cli_overrides: &[String],
) -> Result<GitCommandConfig, CliError> {
    let mut config_overrides = MergedConfig::load(repo_path)
        .ok()
        .map(|merged| merged.get_git_config_overrides())
        .unwrap_or_default();
    config_overrides.extend(cli_overrides.iter().cloned());
    GitCommandConfig::new(config_overrides)
}

pub fn run_git(
    repo_path: &Path,
    args: &[&str],
    action: &str,
    git_command_config: &GitCommandConfig,
) -> Result<(), CliError> {
    run_git_capture(repo_path, args, action, git_command_config).map(|_| ())
}

pub fn run_git_capture(
    repo_path: &Path,
    args: &[&str],
    action: &str,
    git_command_config: &GitCommandConfig,
) -> Result<Output, CliError> {
    let output = git_command(repo_path, git_command_config)
        .args(args)
        .output()
        .map_err(CliError::IoError)?;

    if output.status.success() {
        return Ok(output);
    }

    Err(format_git_failure(args, action, output.stderr))
}

pub fn run_git_with_input(
    repo_path: &Path,
    args: &[&str],
    input: &str,
    action: &str,
    git_command_config: &GitCommandConfig,
) -> Result<(), CliError> {
    let mut child = git_command(repo_path, git_command_config)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CliError::IoError)?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(input.as_bytes())
            .map_err(CliError::IoError)?;
    }

    let output = child.wait_with_output().map_err(CliError::IoError)?;
    if output.status.success() {
        return Ok(());
    }

    Err(format_git_failure(args, action, output.stderr))
}

fn git_command(repo_path: &Path, git_command_config: &GitCommandConfig) -> Command {
    let mut command = Command::new("git");
    command.current_dir(repo_path);

    for config_override in git_command_config.config_overrides() {
        command.arg("-c").arg(config_override);
    }

    command
}

fn format_git_failure(args: &[&str], action: &str, stderr: Vec<u8>) -> CliError {
    let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
    let detail = if stderr.is_empty() {
        format!("git {:?} failed", args)
    } else {
        stderr
    };
    CliError::Generic(format!("Failed to {action}: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::{resolve_git_command_config, GitCommandConfig};
    use crate::config::git::GitConfig;
    use crate::config::repository::{
        RepositoryConfig, RepositoryMetadata, RepositoryType, VersioningConfig, VersioningStrategy,
    };
    use crate::config::Config;
    use serial_test::serial;
    use std::env;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_git_command_config_rejects_invalid_override() {
        assert!(GitCommandConfig::new(vec!["missing-equals".to_string()]).is_err());
    }

    #[test]
    #[serial]
    fn test_resolve_git_command_config_uses_user_repo_then_cli_precedence() {
        let temp_home = tempdir().unwrap();
        let repo_dir = tempdir().unwrap();

        env::set_var("COMMITTY_CONFIG_DIR", temp_home.path());

        let user_config = Config {
            last_update_check: chrono::DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00")
                .unwrap(),
            metrics_enabled: true,
            last_metrics_reminder: chrono::DateTime::parse_from_rfc3339(
                "2025-01-08T17:39:49+01:00",
            )
            .unwrap(),
            user_id: Uuid::new_v4().to_string(),
            major_regex: crate::config::MAJOR_REGEX.to_string(),
            minor_regex: crate::config::MINOR_REGEX.to_string(),
            patch_regex: crate::config::PATCH_REGEX.to_string(),
            git: GitConfig {
                config_overrides: vec!["user.value=1".to_string(), "shared.value=user".to_string()],
            },
            convention: None,
            release: None,
            changelog: None,
        };
        user_config.save().unwrap();

        let repo_config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: "test".to_string(),
                repo_type: RepositoryType::SinglePackage,
                description: None,
            },
            versioning: VersioningConfig {
                strategy: VersioningStrategy::Independent,
                unified_version: None,
                rules: None,
            },
            packages: vec![],
            dependencies: vec![],
            scopes: Default::default(),
            commit_rules: Default::default(),
            branch_rules: Default::default(),
            git: GitConfig {
                config_overrides: vec!["repo.value=1".to_string(), "shared.value=repo".to_string()],
            },
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        };
        repo_config.save(repo_dir.path()).unwrap();

        let git_command_config = resolve_git_command_config(
            repo_dir.path(),
            &["cli.value=1".to_string(), "shared.value=cli".to_string()],
        )
        .unwrap();

        assert_eq!(
            git_command_config.config_overrides(),
            &[
                "user.value=1".to_string(),
                "shared.value=user".to_string(),
                "repo.value=1".to_string(),
                "shared.value=repo".to_string(),
                "cli.value=1".to_string(),
                "shared.value=cli".to_string(),
            ]
        );
    }
}
