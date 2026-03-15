// Packages command for listing and managing detected packages

use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::RepositoryConfig;
use crate::error::CliError;
use crate::packages::MultiPackageDetector;
use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct PackagesCommand {
    #[structopt(subcommand)]
    pub subcommand: PackagesSubcommand,
}

#[derive(StructOpt)]
pub enum PackagesSubcommand {
    #[structopt(about = "List all detected packages")]
    List {
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
        #[structopt(
            long,
            help = "Maximum depth to search for packages",
            default_value = "5"
        )]
        max_depth: usize,
    },
    #[structopt(about = "Show package version status and detect inconsistencies")]
    Status {
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
        #[structopt(long, help = "Exit with error code if issues found (for CI)")]
        check: bool,
    },
    #[structopt(about = "Synchronize package versions according to configuration")]
    Sync {
        #[structopt(short, long, help = "Show what would be done without making changes")]
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
    #[structopt(about = "Show packages changed in a git range")]
    Changed {
        #[structopt(long, help = "Git range (e.g., main..HEAD, HEAD~1)")]
        range: Option<String>,
        #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
        output: String,
        #[structopt(short, long)]
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

impl Command for PackagesCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        match &self.subcommand {
            PackagesSubcommand::List {
                verbose,
                output,
                repo_path,
                max_depth,
            } => list(repo_path, *verbose, *max_depth, output)
                .map_err(|e| CliError::Generic(e.to_string())),
            PackagesSubcommand::Status {
                verbose,
                output,
                repo_path,
                check,
            } => status(repo_path, *verbose, *check, output)
                .map_err(|e| CliError::Generic(e.to_string())),
            PackagesSubcommand::Sync {
                dry_run,
                output,
                repo_path,
            } => sync(repo_path, *dry_run, output).map_err(|e| CliError::Generic(e.to_string())),
            PackagesSubcommand::Changed {
                range,
                output,
                verbose,
                repo_path,
            } => changed(repo_path, range.as_deref(), output, *verbose)
                .map_err(|e| CliError::Generic(e.to_string())),
        }
    }
}

#[derive(Debug, Serialize)]
struct PackageListItem {
    name: String,
    manager: String,
    package_type: String,
    path: String,
    version: String,
    version_file: String,
    version_field: String,
    workspace: bool,
    workspace_members: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConfiguredPackageStatus {
    name: String,
    path: String,
    package_type: String,
    found: bool,
    version: Option<String>,
    manager: Option<String>,
    primary: bool,
    sync_with: Option<String>,
    issues: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SyncOperation {
    name: String,
    path: String,
    from_version: String,
    to_version: String,
    manager: String,
}

#[derive(Debug, Serialize)]
struct ChangedPackageJson {
    name: String,
    path: String,
    version: String,
    files_changed: usize,
    files: Vec<String>,
}

/// List all detected packages
pub fn list(repo_path: &Path, verbose: bool, max_depth: usize, output: &str) -> Result<()> {
    let detector = MultiPackageDetector::new().with_max_depth(max_depth);
    let packages = detector.detect_all(repo_path)?;

    if output == "json" {
        let package_items: Vec<_> = packages.iter().map(package_list_item).collect();
        let payload = json!({
            "command": "packages",
            "mode": "list",
            "ok": true,
            "dry_run": false,
            "errors": serde_json::Value::Null,
            "repo_path": repo_path.display().to_string(),
            "max_depth": max_depth,
            "total_packages": package_items.len(),
            "packages": package_items,
            "verbose": verbose,
        });
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    println!("{}", "Detecting packages...".bold());
    println!();

    if packages.is_empty() {
        println!("{}", "No packages detected".yellow());
        println!("  Try running with --max-depth to search deeper");
        return Ok(());
    }

    println!("{}", format!("Found {} package(s):", packages.len()).bold());
    println!();

    for pkg in &packages {
        let workspace_indicator = if pkg.is_workspace() {
            " [workspace]".magenta().to_string()
        } else {
            String::new()
        };

        println!(
            "  {} {} - {} at {}{}",
            "●".green(),
            pkg.name.bold(),
            pkg.manager.name().cyan(),
            pkg.path.display(),
            workspace_indicator
        );

        if verbose {
            println!("    Version: {}", pkg.version);
            println!("    Version file: {}", pkg.version_file);
            println!("    Version field: {}", pkg.version_field);
            println!("    Package type: {}", pkg.manager.package_type());

            if !pkg.workspace_members.is_empty() {
                println!("    Workspace members:");
                for member in &pkg.workspace_members {
                    println!("      - {}", member);
                }
            }
            println!();
        }
    }

    if !verbose {
        println!();
        println!("{}", "Tip: Use --verbose for more details".dimmed());
    }

    Ok(())
}

/// Show package version status
pub fn status(repo_path: &Path, verbose: bool, check: bool, output: &str) -> Result<()> {
    // Try to load repository config
    let repo_config = RepositoryConfig::try_load(repo_path)?;

    if repo_config.is_none() {
        if output == "json" {
            let payload = json!({
                "command": "packages",
                "mode": "status",
                "ok": true,
                "dry_run": false,
                "errors": serde_json::Value::Null,
                "configured": false,
                "repo_path": repo_path.display().to_string(),
                "issues": Vec::<String>::new(),
                "packages": Vec::<serde_json::Value>::new(),
                "unconfigured_packages": Vec::<serde_json::Value>::new(),
                "verbose": verbose,
                "check": check,
            });
            println!("{}", serde_json::to_string(&payload)?);
            return Ok(());
        }

        println!("{}", "Checking package status...".bold());
        println!();
        println!("{}", "No .committy/config.toml found".yellow());
        println!("  Running in single-package mode");
        println!("  Use 'committy init --multi-package' to enable multi-package support");
        return Ok(());
    }

    let config = repo_config.unwrap();

    // Detect actual packages
    let detector = MultiPackageDetector::new();
    let detected_packages = detector.detect_all(repo_path)?;

    let mut package_statuses = Vec::new();
    let mut unconfigured_packages = Vec::new();
    if output != "json" {
        println!("{}", "Checking package status...".bold());
        println!();
        println!("{}", "Package Status:".bold());
        println!();
    }

    let mut issues = Vec::new();

    // Check each configured package
    for cfg_pkg in &config.packages {
        let _pkg_path = repo_path.join(&cfg_pkg.path);

        // Find matching detected package
        let detected = detected_packages
            .iter()
            .find(|p| p.path == std::path::Path::new(&cfg_pkg.path));

        if let Some(detected_pkg) = detected {
            // Package found
            if output != "json" {
                let status_icon = "✓".green();
                println!(
                    "  {} {} - {} ({})",
                    status_icon,
                    cfg_pkg.name.bold(),
                    detected_pkg.version,
                    detected_pkg.manager.name()
                );

                if verbose {
                    println!("    Path: {}", cfg_pkg.path);
                    println!("    Type: {}", cfg_pkg.package_type);
                    if cfg_pkg.primary {
                        println!("    {}", "Primary package".yellow());
                    }
                    if let Some(ref sync_with) = cfg_pkg.sync_with {
                        println!("    Syncs with: {}", sync_with.cyan());
                    }
                    println!();
                }
            }

            // Check for version sync issues
            if let Some(ref sync_with) = cfg_pkg.sync_with {
                // Find the package we should sync with
                if let Some(sync_pkg) = config.packages.iter().find(|p| &p.name == sync_with) {
                    let _sync_pkg_path = repo_path.join(&sync_pkg.path);
                    if let Some(sync_detected) = detected_packages
                        .iter()
                        .find(|p| p.path == std::path::Path::new(&sync_pkg.path))
                    {
                        if detected_pkg.version != sync_detected.version {
                            issues.push(format!(
                                "{} version ({}) does not match {} version ({})",
                                cfg_pkg.name,
                                detected_pkg.version,
                                sync_with,
                                sync_detected.version
                            ));
                        }
                    }
                }
            }

            let mut package_issues = Vec::new();
            if let Some(ref sync_with) = cfg_pkg.sync_with {
                if let Some(sync_pkg) = config.packages.iter().find(|p| &p.name == sync_with) {
                    if let Some(sync_detected) = detected_packages
                        .iter()
                        .find(|p| p.path == std::path::Path::new(&sync_pkg.path))
                    {
                        if detected_pkg.version != sync_detected.version {
                            package_issues.push(format!(
                                "{} version ({}) does not match {} version ({})",
                                cfg_pkg.name,
                                detected_pkg.version,
                                sync_with,
                                sync_detected.version
                            ));
                        }
                    }
                }
            }

            package_statuses.push(ConfiguredPackageStatus {
                name: cfg_pkg.name.clone(),
                path: cfg_pkg.path.clone(),
                package_type: cfg_pkg.package_type.clone(),
                found: true,
                version: Some(detected_pkg.version.clone()),
                manager: Some(detected_pkg.manager.name().to_string()),
                primary: cfg_pkg.primary,
                sync_with: cfg_pkg.sync_with.clone(),
                issues: package_issues,
            });
        } else {
            // Package not found
            if output != "json" {
                let status_icon = "✗".red();
                println!(
                    "  {} {} - {}",
                    status_icon,
                    cfg_pkg.name.bold(),
                    "NOT FOUND".red()
                );
            }
            issues.push(format!(
                "Package '{}' not found at {}",
                cfg_pkg.name, cfg_pkg.path
            ));
            package_statuses.push(ConfiguredPackageStatus {
                name: cfg_pkg.name.clone(),
                path: cfg_pkg.path.clone(),
                package_type: cfg_pkg.package_type.clone(),
                found: false,
                version: None,
                manager: None,
                primary: cfg_pkg.primary,
                sync_with: cfg_pkg.sync_with.clone(),
                issues: vec![format!(
                    "Package '{}' not found at {}",
                    cfg_pkg.name, cfg_pkg.path
                )],
            });
        }
    }

    // Check for detected packages not in config
    for detected_pkg in &detected_packages {
        if !config
            .packages
            .iter()
            .any(|p| std::path::Path::new(&p.path) == detected_pkg.path)
        {
            unconfigured_packages.push(package_list_item(detected_pkg));
            if output != "json" {
                println!(
                    "  {} {} - {} ({})",
                    "⚠".yellow(),
                    detected_pkg.name.bold(),
                    detected_pkg.version,
                    "not in config".yellow()
                );
                if verbose {
                    println!("    Path: {}", detected_pkg.path.display());
                    println!();
                }
            }
        }
    }

    if output == "json" {
        let has_issues = !issues.is_empty();
        let payload = json!({
            "command": "packages",
            "mode": "status",
            "ok": !has_issues,
            "dry_run": false,
            "errors": serde_json::Value::Null,
            "configured": true,
            "repo_path": repo_path.display().to_string(),
            "issues": issues,
            "packages": package_statuses,
            "unconfigured_packages": unconfigured_packages,
            "verbose": verbose,
            "check": check,
        });
        println!("{}", serde_json::to_string(&payload)?);
        if check && has_issues {
            return Err(anyhow::anyhow!("Package status check failed"));
        }
        return Ok(());
    }

    println!();

    // Show issues
    if !issues.is_empty() {
        println!("{}", "Issues Found:".red().bold());
        for issue in &issues {
            println!("  {} {}", "✗".red(), issue);
        }
        println!();

        if check {
            return Err(anyhow::anyhow!("Package status check failed"));
        }
    } else {
        println!("{}", "✓ All packages are in sync".green().bold());
    }

    Ok(())
}

/// Synchronize package versions according to configuration (Q30: Essential - support --dry-run)
pub fn sync(repo_path: &Path, dry_run: bool, output: &str) -> Result<()> {
    // Load repository config
    let config = match RepositoryConfig::try_load(repo_path)? {
        Some(c) => c,
        None => {
            if output == "json" {
                let payload = json!({
                    "command": "packages",
                    "mode": "sync",
                    "ok": true,
                    "dry_run": dry_run,
                    "errors": serde_json::Value::Null,
                    "config_found": false,
                    "repo_path": repo_path.display().to_string(),
                    "operations": Vec::<serde_json::Value>::new(),
                    "updated": Vec::<String>::new(),
                    "failed": Vec::<String>::new(),
                });
                println!("{}", serde_json::to_string(&payload)?);
                return Ok(());
            }

            println!("{}", "Synchronizing package versions...".bold());
            println!();
            println!("{}", "No .committy/config.toml found".yellow());
            println!("  Use 'committy init --multi-package' to enable multi-package support");
            return Ok(());
        }
    };

    // Detect current packages
    let detector = MultiPackageDetector::new();
    let detected_packages = detector.detect_all(repo_path)?;

    let mut sync_operations = Vec::new();

    // Find packages that need syncing
    for cfg_pkg in &config.packages {
        if let Some(ref sync_with) = cfg_pkg.sync_with {
            // Find the package we should sync with
            let sync_target = config
                .packages
                .iter()
                .find(|p| &p.name == sync_with)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Package '{}' syncs with non-existent package '{}'",
                        cfg_pkg.name,
                        sync_with
                    )
                })?;

            // Get current versions
            let current_pkg = detected_packages
                .iter()
                .find(|p| p.path == std::path::Path::new(&cfg_pkg.path));

            let target_pkg = detected_packages
                .iter()
                .find(|p| p.path == std::path::Path::new(&sync_target.path));

            if let (Some(current), Some(target)) = (current_pkg, target_pkg) {
                if current.version != target.version {
                    sync_operations.push((
                        cfg_pkg.name.clone(),
                        cfg_pkg.path.clone(),
                        current.version.clone(),
                        target.version.clone(),
                        current.manager.clone(),
                    ));
                }
            }
        }
    }

    let operations_json: Vec<_> = sync_operations
        .iter()
        .map(|(name, path, old_ver, new_ver, manager)| SyncOperation {
            name: name.clone(),
            path: path.clone(),
            from_version: old_ver.clone(),
            to_version: new_ver.clone(),
            manager: manager.name().to_string(),
        })
        .collect();

    if sync_operations.is_empty() {
        if output == "json" {
            let payload = json!({
                "command": "packages",
                "mode": "sync",
                "ok": true,
                "dry_run": dry_run,
                "errors": serde_json::Value::Null,
                "config_found": true,
                "repo_path": repo_path.display().to_string(),
                "operations": operations_json,
                "updated": Vec::<String>::new(),
                "failed": Vec::<String>::new(),
            });
            println!("{}", serde_json::to_string(&payload)?);
            return Ok(());
        }

        println!("{}", "Synchronizing package versions...".bold());
        println!();
        println!("{}", "✓ All packages are already in sync".green().bold());
        return Ok(());
    }

    if output == "json" && dry_run {
        let payload = json!({
            "command": "packages",
            "mode": "sync",
            "ok": true,
            "dry_run": true,
            "errors": serde_json::Value::Null,
            "config_found": true,
            "repo_path": repo_path.display().to_string(),
            "operations": operations_json,
            "updated": Vec::<String>::new(),
            "failed": Vec::<String>::new(),
        });
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    if output != "json" {
        println!("{}", "Synchronizing package versions...".bold());
        println!();
        println!("{}", "Packages to sync:".bold());
        for (name, _path, old_ver, new_ver, _manager) in &sync_operations {
            println!(
                "  {} {} → {}",
                name.bold(),
                old_ver.dimmed(),
                new_ver.green()
            );
        }
        println!();
    }

    if dry_run {
        if output != "json" {
            println!("{}", "Dry run - no changes made".yellow());
        }
        return Ok(());
    }

    // Q29: Prompt user to commit
    if output != "json" {
        println!("{}", "Applying version updates...".bold());
    }

    // Apply updates
    use crate::packages::cargo::CargoDetector;
    use crate::packages::npm::NpmDetector;
    use crate::packages::types::PackageDetector;

    let mut updated = Vec::new();
    let mut failed = Vec::new();

    for (name, path, _old_ver, new_ver, manager) in &sync_operations {
        let pkg_path = repo_path.join(path);

        let result = match manager {
            crate::packages::types::PackageManager::Cargo { .. } => {
                CargoDetector.set_version(&pkg_path, new_ver)
            }
            crate::packages::types::PackageManager::Npm { .. }
            | crate::packages::types::PackageManager::Pnpm { .. }
            | crate::packages::types::PackageManager::Yarn { .. } => {
                NpmDetector.set_version(&pkg_path, new_ver)
            }
            _ => {
                let message = format!("{name} - unsupported package manager");
                failed.push(message.clone());
                if output != "json" {
                    println!("  {} {}", "⚠".yellow(), message);
                }
                continue;
            }
        };

        match result {
            Ok(_) => {
                updated.push(name.clone());
                if output != "json" {
                    println!("  {} {} updated", "✓".green(), name);
                }
            }
            Err(e) => {
                failed.push(format!("{name} failed: {e}"));
                if output != "json" {
                    println!("  {} {} failed: {}", "✗".red(), name, e);
                }
            }
        }
    }

    if output == "json" {
        let payload = json!({
            "command": "packages",
            "mode": "sync",
            "ok": failed.is_empty(),
            "dry_run": false,
            "errors": if failed.is_empty() { serde_json::Value::Null } else { json!(failed.clone()) },
            "config_found": true,
            "repo_path": repo_path.display().to_string(),
            "operations": operations_json,
            "updated": updated,
            "failed": failed,
        });
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "✓ Synchronization complete".green().bold());
    println!();
    println!("{}", "Remember to commit these changes:".dimmed());
    println!("  {}", "git add .".dimmed());
    println!(
        "  {}",
        "git commit -m \"chore: sync package versions\"".dimmed()
    );

    Ok(())
}

/// Show packages changed in a git range
pub fn changed(
    repo_path: &Path,
    range: Option<&str>,
    output_format: &str,
    verbose: bool,
) -> Result<()> {
    // Load repository config (for potential future use)
    let _merged_config = MergedConfig::load(repo_path)
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))
        .ok();

    // Get changed files in the git range
    let default_range = "HEAD~1..HEAD";
    let range_to_use = range.unwrap_or(default_range);
    let changed_files = get_changed_files(repo_path, range_to_use)?;

    if changed_files.is_empty() {
        if output_format == "json" {
            let result = json!({
                "command": "packages",
                "mode": "changed",
                "ok": true,
                "dry_run": false,
                "errors": serde_json::Value::Null,
                "packages": [],
                "total_packages": 0,
                "total_files": 0,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("{}", "No files changed in this range".yellow());
        }
        return Ok(());
    }

    // Detect packages and map files to them
    let detector = MultiPackageDetector::new();
    let all_packages = detector.detect_all(repo_path)?;

    let mut changed_packages: HashMap<String, ChangedPackageInfo> = HashMap::new();

    for file in &changed_files {
        // Find which package this file belongs to
        for pkg in &all_packages {
            if file.starts_with(&pkg.path) {
                changed_packages
                    .entry(pkg.name.clone())
                    .or_insert_with(|| ChangedPackageInfo {
                        name: pkg.name.clone(),
                        path: pkg.path.to_string_lossy().to_string(),
                        version: pkg.version.clone(),
                        files_changed: vec![],
                    })
                    .files_changed
                    .push(file.clone());
                break;
            }
        }
    }

    // Output results
    if output_format == "json" {
        output_json(&changed_packages, &changed_files)?;
    } else {
        output_text(&changed_packages, &changed_files, verbose)?;
    }

    Ok(())
}

/// Information about a changed package
#[derive(Debug, Clone)]
pub struct ChangedPackageInfo {
    pub name: String,
    pub path: String,
    pub version: String,
    pub files_changed: Vec<PathBuf>,
}

/// Get changed files in a git range
fn get_changed_files(repo_path: &Path, range: &str) -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["diff", range, "--name-only"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to get git diff: {}", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let files = String::from_utf8(output.stdout)?
        .lines()
        .map(PathBuf::from)
        .collect();

    Ok(files)
}

/// Output changed packages in text format
fn output_text(
    packages: &HashMap<String, ChangedPackageInfo>,
    files: &[PathBuf],
    verbose: bool,
) -> Result<()> {
    if packages.is_empty() {
        println!("{}", "No packages changed".yellow());
        return Ok(());
    }

    println!(
        "{}",
        format!("Packages changed: {}", packages.len())
            .bold()
            .green()
    );
    println!();

    for pkg in packages.values() {
        println!(
            "  {} {} - {}",
            "●".green(),
            pkg.name.bold(),
            pkg.version.cyan()
        );
        if verbose {
            println!("    Path: {}", pkg.path);
            println!("    Files changed: {}", pkg.files_changed.len());
            for file in &pkg.files_changed {
                println!("      - {}", file.display().to_string().dimmed());
            }
        }
    }

    println!();
    println!("Total files changed: {}", files.len());

    Ok(())
}

/// Output changed packages in JSON format
fn output_json(packages: &HashMap<String, ChangedPackageInfo>, files: &[PathBuf]) -> Result<()> {
    let packages_json: Vec<_> = packages.values().map(changed_package_json).collect();

    let result = json!({
        "command": "packages",
        "mode": "changed",
        "ok": true,
        "dry_run": false,
        "errors": serde_json::Value::Null,
        "packages": packages_json,
        "total_packages": packages.len(),
        "total_files": files.len(),
    });

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

fn package_list_item(pkg: &crate::packages::types::PackageInfo) -> PackageListItem {
    PackageListItem {
        name: pkg.name.clone(),
        manager: pkg.manager.name().to_string(),
        package_type: pkg.manager.package_type().to_string(),
        path: pkg.path.display().to_string(),
        version: pkg.version.clone(),
        version_file: pkg.version_file.clone(),
        version_field: pkg.version_field.clone(),
        workspace: pkg.is_workspace(),
        workspace_members: pkg.workspace_members.clone(),
    }
}

fn changed_package_json(pkg: &ChangedPackageInfo) -> ChangedPackageJson {
    ChangedPackageJson {
        name: pkg.name.clone(),
        path: pkg.path.clone(),
        version: pkg.version.clone(),
        files_changed: pkg.files_changed.len(),
        files: pkg
            .files_changed
            .iter()
            .map(|file| file.to_string_lossy().to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_list_no_packages() {
        let temp_dir = TempDir::new().unwrap();
        let result = list(temp_dir.path(), false, 5, "text");
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_with_package() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-package"
version = "1.0.0"
edition = "2021"
        "#,
        )
        .unwrap();

        let result = list(temp_dir.path(), false, 5, "text");
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_without_config() {
        let temp_dir = TempDir::new().unwrap();
        let result = status(temp_dir.path(), false, false, "text");
        assert!(result.is_ok());
    }
}
