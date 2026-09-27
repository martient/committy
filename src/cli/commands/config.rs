// Config command for validating and showing repository configuration

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::{
    CommitRulesConfig, RepositoryConfig, RepositoryMetadata, RepositoryType, VersioningConfig,
    VersioningStrategy,
};
use crate::error::CliError;
use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use std::fs;
use std::path::Path;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct ConfigCommand {
    #[structopt(subcommand)]
    pub subcommand: ConfigSubcommand,
}

#[derive(StructOpt)]
pub enum ConfigSubcommand {
    #[structopt(about = "Validate repository configuration")]
    Validate {
        #[structopt(short, long, help = "Show detailed information")]
        verbose: bool,
        #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
        output: String,
        #[structopt(
            long,
            help = "Repository path",
            default_value = ".",
            parse(from_os_str)
        )]
        repo_path: std::path::PathBuf,
    },
    #[structopt(about = "Show merged configuration (repository + user)")]
    Show {
        #[structopt(short, long, help = "Show detailed information including sources")]
        verbose: bool,
        #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
        output: String,
        #[structopt(
            long,
            help = "Repository path",
            default_value = ".",
            parse(from_os_str)
        )]
        repo_path: std::path::PathBuf,
    },
    #[structopt(about = "Scaffold a Committy-native config file")]
    Scaffold {
        #[structopt(short, long)]
        dry_run: bool,
        #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
        output: String,
        #[structopt(
            long,
            help = "Repository path",
            default_value = ".",
            parse(from_os_str)
        )]
        repo_path: std::path::PathBuf,
    },
}

impl Command for ConfigCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        match &self.subcommand {
            ConfigSubcommand::Validate {
                verbose,
                output,
                repo_path,
            } => {
                validate(repo_path, *verbose, output).map_err(|e| CliError::Generic(e.to_string()))
            }
            ConfigSubcommand::Show {
                verbose,
                output,
                repo_path,
            } => show(repo_path, *verbose, output).map_err(|e| CliError::Generic(e.to_string())),
            ConfigSubcommand::Scaffold {
                dry_run,
                output,
                repo_path,
            } => {
                scaffold(repo_path, *dry_run, output).map_err(|e| CliError::Generic(e.to_string()))
            }
        }
    }
}

/// Validate repository configuration
pub fn validate(repo_path: &Path, verbose: bool, output: &str) -> Result<()> {
    let config_path = RepositoryConfig::get_config_path(repo_path)?;
    let repo_config = RepositoryConfig::try_load(repo_path)?;

    if output == "json" {
        let warnings = repo_config
            .as_ref()
            .map(|config| collect_validation_warnings(repo_path, config))
            .unwrap_or_default();

        let payload = if let Some(config) = repo_config {
            json!({
                "command": "config",
                "mode": "validate",
                "ok": true,
                "dry_run": false,
                "errors": serde_json::Value::Null,
                "config_found": true,
                "single_package_mode": false,
                "repo_path": repo_path.display().to_string(),
                "config_path": config_path.display().to_string(),
                "repository": {
                    "name": config.repository.name,
                    "type": config.repository.repo_type,
                    "versioning_strategy": config.versioning.strategy,
                    "package_count": config.packages.len(),
                    "dependency_count": config.dependencies.len(),
                    "scope_mapping_count": config.scopes.mappings.len(),
                    "git_config_override_count": config.git.config_overrides.len(),
                },
                "packages": config.packages,
                "dependencies": config.dependencies,
                "scope_mappings": config.scopes.mappings,
                "git": config.git,
                "convention": config.convention,
                "release": config.release,
                "changelog": config.changelog,
                "warnings": warnings,
                "verbose": verbose,
            })
        } else {
            json!({
                "command": "config",
                "mode": "validate",
                "ok": true,
                "dry_run": false,
                "errors": serde_json::Value::Null,
                "config_found": false,
                "single_package_mode": true,
                "repo_path": repo_path.display().to_string(),
                "config_path": config_path.display().to_string(),
                "repository": serde_json::Value::Null,
                "packages": Vec::<serde_json::Value>::new(),
                "dependencies": Vec::<serde_json::Value>::new(),
                "scope_mappings": Vec::<serde_json::Value>::new(),
                "convention": serde_json::Value::Null,
                "release": serde_json::Value::Null,
                "changelog": serde_json::Value::Null,
                "warnings": Vec::<String>::new(),
                "verbose": verbose,
            })
        };

        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    println!("{}", "Validating repository configuration...".bold());
    println!();

    // Try to load repository config
    let repo_config = match repo_config {
        Some(config) => config,
        None => {
            println!("{}", "✓ No .committy/config.toml found".green());
            println!("{}", "  Running in single-package mode".dimmed());
            return Ok(());
        }
    };

    println!("{}", "✓ Configuration file found".green());
    println!("{}", "✓ Configuration is valid".green());
    println!();

    // Show summary
    println!("{}", "Configuration Summary:".bold());
    println!("  Repository: {}", repo_config.repository.name);
    println!("  Type: {:?}", repo_config.repository.repo_type);
    println!(
        "  Versioning Strategy: {:?}",
        repo_config.versioning.strategy
    );
    println!("  Packages: {}", repo_config.packages.len());
    println!("  Dependencies: {}", repo_config.dependencies.len());
    println!("  Scope Mappings: {}", repo_config.scopes.mappings.len());
    println!(
        "  Git Config Overrides: {}",
        repo_config.git.config_overrides.len()
    );
    println!(
        "  Convention Configured: {}",
        repo_config.convention.is_some()
    );
    println!("  Release Configured: {}", repo_config.release.is_some());
    println!(
        "  Changelog Configured: {}",
        repo_config.changelog.is_some()
    );
    println!();

    // Show packages
    if !repo_config.packages.is_empty() {
        println!("{}", "Packages:".bold());
        for pkg in &repo_config.packages {
            let mut flags = Vec::new();
            if pkg.primary {
                flags.push("primary".yellow().to_string());
            }
            if pkg.independent {
                flags.push("independent".cyan().to_string());
            }
            if let Some(ref sync_with) = pkg.sync_with {
                flags.push(format!("syncs with {}", sync_with).blue().to_string());
            }
            if pkg.workspace_member {
                flags.push("workspace member".magenta().to_string());
            }

            let flags_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" ({})", flags.join(", "))
            };

            println!(
                "  {} {} - {} at {}{}",
                "✓".green(),
                pkg.name.bold(),
                pkg.package_type,
                pkg.path,
                flags_str
            );

            if verbose {
                println!("    Version file: {}", pkg.version_file);
                println!("    Version field: {}", pkg.version_field);
                if let Some(ref desc) = pkg.description {
                    println!("    Description: {}", desc);
                }
            }
        }
        println!();
    }

    // Show dependencies
    if !repo_config.dependencies.is_empty() {
        println!("{}", "Dependencies:".bold());
        for dep in &repo_config.dependencies {
            println!("  {} → {} targets", dep.source.bold(), dep.targets.len());
            if verbose {
                for target in &dep.targets {
                    println!(
                        "    - {} (field: {}, strategy: {:?})",
                        target.file, target.field, target.strategy
                    );
                }
            }
        }
        println!();
    }

    // Show scope mappings
    if !repo_config.scopes.mappings.is_empty() && verbose {
        println!("{}", "Scope Mappings:".bold());
        for mapping in &repo_config.scopes.mappings {
            println!(
                "  {} → {} (package: {})",
                mapping.pattern,
                mapping.scope.cyan(),
                mapping.package
            );
        }
        println!();
    }

    if !repo_config.git.config_overrides.is_empty() || verbose {
        println!("{}", "Git Overrides:".bold());
        if repo_config.git.config_overrides.is_empty() {
            println!("  None");
        } else {
            for override_value in &repo_config.git.config_overrides {
                println!("  - {}", override_value);
            }
        }
        println!();
    }

    // Warnings
    let warnings = collect_validation_warnings(repo_path, &repo_config);

    if !warnings.is_empty() {
        println!("{}", "Warnings:".yellow().bold());
        for warning in warnings {
            println!("  {} {}", "⚠".yellow(), warning);
        }
        println!();
    }

    println!("{}", "✓ Validation complete".green().bold());
    Ok(())
}

/// Show merged configuration (repository + user)
pub fn show(repo_path: &Path, verbose: bool, output: &str) -> Result<()> {
    let merged = MergedConfig::load(repo_path)?;

    if output == "json" {
        let payload = json!({
            "command": "config",
            "mode": "show",
            "ok": true,
            "dry_run": false,
            "errors": serde_json::Value::Null,
            "repo_path": repo_path.display().to_string(),
            "multi_package": merged.is_multi_package(),
            "effective_patterns": {
                "major_regex": merged.get_major_regex(),
                "minor_regex": merged.get_minor_regex(),
                "patch_regex": merged.get_patch_regex(),
            },
            "effective_git_config_overrides": merged.get_git_config_overrides(),
            "effective_convention": merged.effective_convention(),
            "effective_release": merged.effective_release(),
            "effective_changelog": merged.effective_changelog(),
            "repository_config": merged.repository_config(),
            "user_config": merged.user_config(),
            "verbose": verbose,
        });
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    println!("{}", "Configuration Hierarchy:".bold());
    println!();

    // Repository config
    if let Some(repo_config) = merged.repository_config() {
        println!(
            "{}",
            "Repository Config (.committy/config.toml):".green().bold()
        );
        println!(
            "  Location: {}",
            RepositoryConfig::get_config_path(repo_path)?.display()
        );
        println!("  Repository: {}", repo_config.repository.name);
        println!("  Type: {:?}", repo_config.repository.repo_type);
        println!("  Versioning: {:?}", repo_config.versioning.strategy);

        if verbose {
            if let Some(ref rules) = repo_config.versioning.rules {
                println!("  Custom Regex Patterns:");
                if let Some(ref major) = rules.major_regex {
                    println!("    Major: {}", major);
                }
                if let Some(ref minor) = rules.minor_regex {
                    println!("    Minor: {}", minor);
                }
                if let Some(ref patch) = rules.patch_regex {
                    println!("    Patch: {}", patch);
                }
            }
            println!("  Git Config Overrides:");
            if repo_config.git.config_overrides.is_empty() {
                println!("    None");
            } else {
                for override_value in &repo_config.git.config_overrides {
                    println!("    {}", override_value);
                }
            }
        }
        println!();
    } else {
        println!("{}", "Repository Config: Not found".dimmed());
        println!("  Running in single-package mode");
        println!();
    }

    // User config
    println!(
        "{}",
        "User Config (~/.config/committy/config.toml):"
            .cyan()
            .bold()
    );
    let user_config = merged.user_config();
    println!("  Metrics Enabled: {}", user_config.metrics_enabled);

    if verbose {
        println!("  Regex Patterns:");
        println!("    Major: {}", user_config.major_regex);
        println!("    Minor: {}", user_config.minor_regex);
        println!("    Patch: {}", user_config.patch_regex);
        println!("  Git Config Overrides:");
        if user_config.git.config_overrides.is_empty() {
            println!("    None");
        } else {
            for override_value in &user_config.git.config_overrides {
                println!("    {}", override_value);
            }
        }
    }
    println!();

    // Effective config
    println!("{}", "Effective Configuration:".yellow().bold());
    println!("  Multi-package mode: {}", merged.is_multi_package());
    println!("  Major regex: {}", merged.get_major_regex());
    println!("  Minor regex: {}", merged.get_minor_regex());
    println!("  Patch regex: {}", merged.get_patch_regex());
    println!(
        "  Git config overrides: {}",
        if merged.get_git_config_overrides().is_empty() {
            "None".to_string()
        } else {
            merged.get_git_config_overrides().join(", ")
        }
    );
    println!("  Convention: {}", merged.effective_convention().name);
    println!(
        "  Release provider: {}",
        merged.effective_release().provider
    );
    println!(
        "  Changelog template: {}",
        merged.effective_changelog().template
    );

    Ok(())
}

pub fn scaffold(repo_path: &Path, dry_run: bool, output: &str) -> Result<()> {
    let config_path = RepositoryConfig::get_config_path(repo_path)?;
    let config = RepositoryConfig {
        repository: RepositoryMetadata {
            name: repo_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("repository")
                .to_string(),
            repo_type: RepositoryType::SinglePackage,
            description: Some("Committy-native repository config".to_string()),
        },
        versioning: VersioningConfig {
            strategy: VersioningStrategy::Independent,
            unified_version: None,
            rules: None,
        },
        packages: vec![],
        dependencies: vec![],
        scopes: Default::default(),
        commit_rules: CommitRulesConfig::default(),
        branch_rules: Default::default(),
        git: Default::default(),
        convention: Some(Default::default()),
        release: Some(Default::default()),
        changelog: Some(Default::default()),
        workspace: None,
    };

    if output == "json" {
        let payload = json!({
            "command": "config",
            "mode": "scaffold",
            "ok": true,
            "dry_run": dry_run,
            "config_path": config_path.display().to_string(),
            "config": config,
            "errors": serde_json::Value::Null,
        });
        println!("{}", serde_json::to_string(&payload)?);
    } else {
        println!("{}", toml::to_string_pretty(&config)?);
    }

    if !dry_run {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(config_path, toml::to_string_pretty(&config)?)?;
    }
    Ok(())
}

fn collect_validation_warnings(repo_path: &Path, repo_config: &RepositoryConfig) -> Vec<String> {
    let mut warnings = Vec::new();

    for pkg in &repo_config.packages {
        let pkg_path = repo_path.join(&pkg.path);
        let version_file_path = pkg_path.join(&pkg.version_file);
        if !version_file_path.exists() {
            warnings.push(format!(
                "Version file not found: {} (package: {})",
                pkg.version_file, pkg.name
            ));
        }
    }

    for dep in &repo_config.dependencies {
        for target in &dep.targets {
            let target_path = repo_path.join(&target.file);
            if !target_path.exists() {
                warnings.push(format!("Dependency target not found: {}", target.file));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::repository::{
        RepositoryConfig, RepositoryMetadata, RepositoryType, VersioningConfig, VersioningStrategy,
    };
    use tempfile::TempDir;

    #[test]
    fn test_validate_without_config() {
        let temp_dir = TempDir::new().unwrap();
        let result = validate(temp_dir.path(), false, "text");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_config() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal valid config
        let config = RepositoryConfig {
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
            git: Default::default(),
            convention: None,
            release: None,
            changelog: None,
            workspace: None,
        };

        config.save(temp_dir.path()).unwrap();

        let result = validate(temp_dir.path(), false, "text");
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_config() {
        let temp_dir = TempDir::new().unwrap();
        let result = show(temp_dir.path(), false, "text");
        assert!(result.is_ok());
    }
}
