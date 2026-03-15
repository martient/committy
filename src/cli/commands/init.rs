use crate::cli::Command;
use crate::config::repository::{
    CommitRulesConfig, PackageConfig, RepositoryConfig, RepositoryMetadata, RepositoryType,
    ScopeConfig, VersioningConfig, VersioningStrategy,
};
use crate::error::CliError;
use crate::packages::detector::MultiPackageDetector;
use colored::Colorize;
use log::{debug, info};
use serde_json::json;
use std::fs;
use std::path::Path;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct InitCommand {
    #[structopt(long, help = "Initialize multi-package support")]
    #[allow(dead_code)]
    pub multi_package: bool,

    #[structopt(short, long, help = "Show what would be created (don't write files)")]
    pub dry_run: bool,

    #[structopt(
        long,
        help = "Output format: text or json",
        default_value = "text",
        possible_values = &["text", "json"]
    )]
    pub output: String,
}

impl Command for InitCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        debug!("InitCommand::execute called");

        // Check if already initialized
        if Path::new(".committy/config.toml").exists() {
            if non_interactive {
                return Err(CliError::Generic(
                    "Repository already initialized with .committy/config.toml".to_string(),
                ));
            } else if self.output != "json" {
                println!(
                    "{}",
                    "Warning: .committy/config.toml already exists".yellow()
                );
                // In interactive mode, we still proceed but will prompt for confirmation later
            }
        }

        if self.output != "json" {
            println!("{}", "Detecting packages in repository...".bold());
        }
        let current_dir = std::env::current_dir()
            .map_err(|e| CliError::InputError(format!("Failed to get current directory: {}", e)))?;
        let detector = MultiPackageDetector::new().with_max_depth(5);
        let detected_packages = detector
            .detect_all(&current_dir)
            .map_err(|e| CliError::Generic(format!("Package detection failed: {}", e)))?;

        if detected_packages.is_empty() {
            if self.output == "json" {
                self.output_json_result(
                    None,
                    false,
                    false,
                    false,
                    Some(vec![
                        "No packages detected. Create at least one package before initializing."
                            .to_string(),
                    ]),
                )?;
            } else {
                println!(
                    "{}",
                    "No packages detected. Create at least one package before initializing."
                        .yellow()
                );
            }
            return Ok(());
        }

        if self.output != "json" {
            println!(
                "{}",
                format!("Detected {} package(s):", detected_packages.len()).green()
            );
            for pkg in &detected_packages {
                println!("  ● {} ({})", pkg.name.bold(), pkg.manager.name());
            }
            println!();
        }

        // Gather configuration
        let repo_name = if non_interactive {
            "my-repo".to_string()
        } else {
            println!("Repository name (default: my-repo):");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::InputError(format!("Failed to read input: {}", e)))?;
            let trimmed = input.trim().to_string();
            if trimmed.is_empty() {
                "my-repo".to_string()
            } else {
                trimmed
            }
        };

        let strategy = if non_interactive {
            VersioningStrategy::Independent
        } else {
            println!("Versioning strategy (independent/unified/hybrid) [default: independent]:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::InputError(format!("Failed to read input: {}", e)))?;
            let trimmed = input.trim().to_lowercase();
            match trimmed.as_str() {
                "unified" => VersioningStrategy::Unified,
                "hybrid" => VersioningStrategy::Hybrid,
                _ => VersioningStrategy::Independent,
            }
        };

        let auto_detect_scopes = if non_interactive {
            true
        } else {
            println!("Enable automatic scope detection? [default: yes]:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::InputError(format!("Failed to read input: {}", e)))?;
            let trimmed = input.trim().to_lowercase();
            trimmed != "no" && trimmed != "n"
        };

        // Build configuration
        let config = RepositoryConfig {
            repository: RepositoryMetadata {
                name: repo_name.clone(),
                description: Some("Multi-package repository".to_string()),
                repo_type: RepositoryType::MultiPackage,
            },
            versioning: VersioningConfig {
                strategy: strategy.clone(),
                unified_version: if strategy == VersioningStrategy::Unified {
                    Some("0.1.0".to_string())
                } else {
                    None
                },
                rules: None,
            },
            packages: detected_packages
                .iter()
                .map(|pkg| PackageConfig {
                    name: pkg.name.clone(),
                    package_type: pkg.manager.package_type().to_string(),
                    path: pkg.path.to_string_lossy().to_string(),
                    version_file: pkg.version_file.clone(),
                    version_field: pkg.version_field.clone(),
                    primary: false,
                    sync_with: None,
                    independent: false,
                    workspace_member: false,
                    description: None,
                })
                .collect(),
            dependencies: vec![],
            scopes: ScopeConfig {
                auto_detect: auto_detect_scopes,
                require_scope_for_multi_package: false,
                allow_multiple_scopes: false,
                scope_separator: ",".to_string(),
                mappings: vec![],
            },
            commit_rules: CommitRulesConfig::default(),
            workspace: None,
        };

        // Validate configuration
        config
            .validate(&current_dir)
            .map_err(|e| CliError::Generic(format!("Configuration validation failed: {}", e)))?;

        // Serialize to TOML
        let toml_string = toml::to_string_pretty(&config)
            .map_err(|e| CliError::Generic(format!("Failed to serialize config: {}", e)))?;

        // Display configuration
        if self.output != "json" {
            println!("{}", "Configuration to be created:".bold());
            println!("{}", toml_string);
            println!();
        }

        // Handle dry-run
        if self.dry_run {
            if self.output == "json" {
                self.output_json_result(Some(&config), false, true, false, None)?;
            } else {
                println!("{}", "Dry run - no files created".yellow());
            }
            return Ok(());
        }

        // Confirm before writing (interactive mode)
        if !non_interactive {
            println!("Create .committy/config.toml? (yes/no) [default: yes]:");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::InputError(format!("Failed to read input: {}", e)))?;
            let trimmed = input.trim().to_lowercase();
            if trimmed == "no" || trimmed == "n" {
                if self.output == "json" {
                    self.output_json_result(Some(&config), false, false, true, None)?;
                } else {
                    println!("{}", "Cancelled".yellow());
                }
                return Ok(());
            }
        }

        // Write file
        fs::create_dir_all(".committy").map_err(|e| {
            CliError::Generic(format!("Failed to create .committy directory: {}", e))
        })?;

        fs::write(".committy/config.toml", &toml_string).map_err(|e| {
            CliError::Generic(format!("Failed to write .committy/config.toml: {}", e))
        })?;

        if self.output == "json" {
            self.output_json_result(Some(&config), true, true, false, None)?;
        } else {
            println!(
                "{}",
                "✓ Configuration created: .committy/config.toml"
                    .green()
                    .bold()
            );
            println!(
                "{}",
                "Tip: Run 'committy config validate' to verify".dimmed()
            );
        }

        info!("Repository initialized successfully");
        Ok(())
    }
}

impl InitCommand {
    fn output_json_result(
        &self,
        config: Option<&RepositoryConfig>,
        created: bool,
        ok: bool,
        cancelled: bool,
        errors: Option<Vec<String>>,
    ) -> Result<(), CliError> {
        let config_json = config.map(|config| {
            json!({
                "repository": config.repository.name,
                "repository_type": config.repository.repo_type,
                "versioning_strategy": config.versioning.strategy,
                "packages": config.packages,
                "scope_mappings": config.scopes.mappings,
            })
        });

        let result = json!({
            "command": "init",
            "ok": ok,
            "dry_run": self.dry_run,
            "errors": errors,
            "created": created,
            "cancelled": cancelled,
            "config": config_json,
            "path": ".committy/config.toml",
        });
        println!(
            "{}",
            serde_json::to_string(&result)
                .map_err(|e| CliError::Generic(format!("Failed to serialize JSON: {}", e)))?
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_command_creation() {
        let cmd = InitCommand {
            multi_package: true,
            dry_run: true,
            output: "text".to_string(),
        };
        assert!(cmd.multi_package);
        assert!(cmd.dry_run);
        assert_eq!(cmd.output, "text");
    }
}
