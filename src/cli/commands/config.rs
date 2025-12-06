// Config command for validating and showing repository configuration

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::RepositoryConfig;
use crate::error::CliError;
use anyhow::Result;
use colored::Colorize;
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
            ConfigSubcommand::Validate { verbose, repo_path } => {
                validate(repo_path, *verbose).map_err(|e| CliError::Generic(e.to_string()))
            }
            ConfigSubcommand::Show { verbose, repo_path } => {
                show(repo_path, *verbose).map_err(|e| CliError::Generic(e.to_string()))
            }
        }
    }
}

/// Validate repository configuration
pub fn validate(repo_path: &Path, verbose: bool) -> Result<()> {
    println!("{}", "Validating repository configuration...".bold());
    println!();

    // Try to load repository config
    let repo_config = match RepositoryConfig::try_load(repo_path)? {
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

    // Warnings
    let mut warnings = Vec::new();

    // Check for packages without version files
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

    // Check for dependency targets
    for dep in &repo_config.dependencies {
        for target in &dep.targets {
            let target_path = repo_path.join(&target.file);
            if !target_path.exists() {
                warnings.push(format!("Dependency target not found: {}", target.file));
            }
        }
    }

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
pub fn show(repo_path: &Path, verbose: bool) -> Result<()> {
    let merged = MergedConfig::load(repo_path)?;

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
    }
    println!();

    // Effective config
    println!("{}", "Effective Configuration:".yellow().bold());
    println!("  Multi-package mode: {}", merged.is_multi_package());
    println!("  Major regex: {}", merged.get_major_regex());
    println!("  Minor regex: {}", merged.get_minor_regex());
    println!("  Patch regex: {}", merged.get_patch_regex());

    Ok(())
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
        let result = validate(temp_dir.path(), false);
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
            workspace: None,
        };

        config.save(temp_dir.path()).unwrap();

        let result = validate(temp_dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_config() {
        let temp_dir = TempDir::new().unwrap();
        let result = show(temp_dir.path(), false);
        assert!(result.is_ok());
    }
}
