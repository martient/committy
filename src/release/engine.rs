use std::path::{Path, PathBuf};
use std::process::Command;

use git2::Repository;
use minijinja::{context, Environment};
use semver::{Prerelease, Version};
use serde::Serialize;

use crate::config::changelog::ChangelogConfig;
use crate::config::hierarchy::MergedConfig;
use crate::config::release::ReleaseConfig;
use crate::config::repository::BumpType;
use crate::convention::Convention;
use crate::error::CliError;
use crate::git::{
    commit_changes_in_with_config, resolve_git_command_config, run_git, GitCommandConfig,
};
use crate::release::changelog::{
    collect_entries, render_changelog, write_changelog, ChangelogEntry, ChangelogPlan,
};
use crate::release::providers::{resolve_provider, ProjectVersion, ProviderKind};

#[derive(Debug, Clone, Serialize)]
pub struct BumpPlanVersion {
    pub name: String,
    pub current: String,
    pub next: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BumpPlan {
    pub provider: String,
    pub current_version: Option<String>,
    pub next_version: Option<String>,
    pub bump: String,
    pub dry_run: bool,
    pub read_only_provider: bool,
    pub tag_names: Vec<String>,
    pub updated_files: Vec<String>,
    pub commit_message: String,
    pub publish: bool,
    pub changelog: ChangelogPlan,
    pub commits: Vec<ChangelogEntry>,
    pub package_versions: Vec<BumpPlanVersion>,
}

pub struct ReleaseEngine {
    repo_path: PathBuf,
    merged: MergedConfig,
    convention: Convention,
    release: ReleaseConfig,
    changelog: ChangelogConfig,
    git_command_config: GitCommandConfig,
}

impl ReleaseEngine {
    pub fn load(repo_path: &Path, cli_git_config: &[String]) -> Result<Self, CliError> {
        let merged = MergedConfig::load(repo_path).map_err(|e| CliError::Generic(e.to_string()))?;
        let convention = Convention::from_config(merged.effective_convention())?;
        let release = merged.effective_release();
        let changelog = merged.effective_changelog();
        let git_command_config = resolve_git_command_config(repo_path, cli_git_config)?;

        Ok(Self {
            repo_path: repo_path.to_path_buf(),
            merged,
            convention,
            release,
            changelog,
            git_command_config,
        })
    }

    pub fn convention(&self) -> &Convention {
        &self.convention
    }

    pub fn release_config(&self) -> &ReleaseConfig {
        &self.release
    }

    pub fn changelog_config(&self) -> &ChangelogConfig {
        &self.changelog
    }

    pub fn provider_kind(&self) -> Result<ProviderKind, CliError> {
        resolve_provider(&self.repo_path, &self.merged)
    }

    pub fn project_version(&self) -> Result<ProjectVersion, CliError> {
        let provider = self.provider_kind()?;
        provider.read(&self.repo_path, self.merged.repository_config())
    }

    pub fn plan_bump(
        &self,
        override_bump: Option<BumpType>,
        prerelease: bool,
        dry_run: bool,
        publish_requested: bool,
        from_ref: Option<&str>,
        to_ref: Option<&str>,
    ) -> Result<BumpPlan, CliError> {
        let provider = self.provider_kind()?;
        let project_version = provider.read(&self.repo_path, self.merged.repository_config())?;
        let (commits, previous_ref) = collect_entries(
            &self.repo_path,
            &self.convention,
            from_ref,
            to_ref,
            &self.changelog,
        )?;
        let bump = override_bump.unwrap_or_else(|| detect_bump(&self.convention, &commits));
        let bump_label = bump_label(&bump).to_string();

        let mut package_versions = vec![];
        let mut updated_files = provider
            .managed_files(&self.repo_path, self.merged.repository_config())
            .into_iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();

        let (current_version, next_version, tag_names) = if provider == ProviderKind::MultiPackage {
            let Some(project_versions) = project_version.package_versions.clone() else {
                return Err(CliError::InputError(
                    "Multi-package provider requires package versions".to_string(),
                ));
            };
            let mut tag_names = vec![];
            for (name, version) in project_versions {
                let next = calculate_next_version(
                    &version,
                    bump.clone(),
                    prerelease,
                    &self.release.prerelease_suffix,
                )?;
                package_versions.push(BumpPlanVersion {
                    name: name.clone(),
                    current: version,
                    next: next.clone(),
                });
                tag_names.push(format!("{name}@{next}"));
            }
            let current = project_version.version.clone();
            let next = package_versions.first().map(|item| item.next.clone());
            if self.merged.repository_config().is_some() && updated_files.is_empty() {
                updated_files = self
                    .merged
                    .repository_config()
                    .into_iter()
                    .flat_map(|config| config.packages.iter())
                    .map(|pkg| {
                        self.repo_path
                            .join(&pkg.path)
                            .join(&pkg.version_file)
                            .display()
                            .to_string()
                    })
                    .collect();
            }
            (current, next, tag_names)
        } else {
            let current = project_version
                .version
                .clone()
                .unwrap_or_else(|| "0.0.0".to_string());
            let next = calculate_next_version(
                &current,
                bump.clone(),
                prerelease,
                &self.release.prerelease_suffix,
            )?;
            let tag = render_release_template(&self.release.tag_format, &next)?;
            (Some(current), Some(next), vec![tag])
        };

        let next_for_output = next_version.clone().unwrap_or_else(|| "0.0.0".to_string());
        let rendered_changelog = render_changelog(
            &self.changelog,
            &next_for_output,
            previous_ref.as_deref(),
            &commits,
        )?;
        let commit_message =
            render_release_template(&self.release.bump_commit_message, &next_for_output)?;
        let changelog = ChangelogPlan {
            previous_ref,
            next_ref: Some(to_ref.unwrap_or("HEAD").to_string()),
            output_file: self.changelog.output_file.clone(),
            rendered: rendered_changelog,
            entry_count: commits.len(),
        };

        Ok(BumpPlan {
            provider: provider.id().to_string(),
            current_version,
            next_version,
            bump: bump_label,
            dry_run,
            read_only_provider: provider.is_read_only(),
            tag_names,
            updated_files,
            commit_message,
            publish: publish_requested || self.release.publish,
            changelog,
            commits,
            package_versions,
        })
    }

    pub fn apply_bump(&self, plan: &BumpPlan, confirm_publish: bool) -> Result<(), CliError> {
        if plan.publish && !confirm_publish {
            return Err(CliError::InputError(
                "Publishing a release requires --confirm-publish".to_string(),
            ));
        }

        self.run_hooks(&self.release.pre_bump_hooks)?;

        let provider = self.provider_kind()?;
        let mut staged_files = vec![];
        if !provider.is_read_only() {
            if provider == ProviderKind::MultiPackage {
                let Some(config) = self.merged.repository_config() else {
                    return Err(CliError::InputError(
                        "multi-package provider requires repository config".to_string(),
                    ));
                };
                for version in &plan.package_versions {
                    let file = crate::release::providers::write_package_version(
                        &self.repo_path,
                        config,
                        &version.name,
                        &version.next,
                    )?;
                    staged_files.push(file);
                }
            } else if let Some(next_version) = &plan.next_version {
                staged_files.extend(provider.write(
                    &self.repo_path,
                    self.merged.repository_config(),
                    next_version,
                )?);
            }
        }

        if let Some(output_file) = &plan.changelog.output_file {
            let path = self.repo_path.join(output_file);
            write_changelog(&path, &plan.changelog.rendered)?;
            staged_files.push(path);
        }

        if !staged_files.is_empty() {
            let mut add_args = vec!["add", "--"];
            for file in &staged_files {
                let relative = file
                    .strip_prefix(&self.repo_path)
                    .unwrap_or(file)
                    .to_string_lossy()
                    .to_string();
                add_args.push(Box::leak(relative.into_boxed_str()));
            }
            run_git(
                &self.repo_path,
                &add_args,
                "stage release files",
                &self.git_command_config,
            )?;
            commit_changes_in_with_config(
                &self.repo_path,
                &plan.commit_message,
                false,
                &self.git_command_config,
            )?;
        }

        for tag_name in &plan.tag_names {
            if self.release.signed_tag {
                run_git(
                    &self.repo_path,
                    &["tag", "-s", tag_name, "-m", tag_name],
                    "create signed tag",
                    &self.git_command_config,
                )?;
            } else if self.release.annotated_tag {
                run_git(
                    &self.repo_path,
                    &["tag", "-a", tag_name, "-m", tag_name],
                    "create annotated tag",
                    &self.git_command_config,
                )?;
            } else {
                run_git(
                    &self.repo_path,
                    &["tag", tag_name],
                    "create lightweight tag",
                    &self.git_command_config,
                )?;
            }
        }

        if plan.publish {
            if Repository::open(&self.repo_path)
                .map_err(CliError::from)?
                .find_remote("origin")
                .is_ok()
            {
                run_git(
                    &self.repo_path,
                    &["push", "origin", "HEAD"],
                    "push release commit",
                    &self.git_command_config,
                )?;
                for tag_name in &plan.tag_names {
                    run_git(
                        &self.repo_path,
                        &["push", "origin", &format!("refs/tags/{tag_name}")],
                        "push release tag",
                        &self.git_command_config,
                    )?;
                }
            }
        }

        self.run_hooks(&self.release.post_bump_hooks)?;
        Ok(())
    }

    fn run_hooks(&self, hooks: &[String]) -> Result<(), CliError> {
        for hook in hooks {
            let status = Command::new("sh")
                .arg("-lc")
                .arg(hook)
                .current_dir(&self.repo_path)
                .status()
                .map_err(CliError::IoError)?;
            if !status.success() {
                return Err(CliError::Generic(format!("Release hook failed: {hook}")));
            }
        }
        Ok(())
    }
}

fn detect_bump(convention: &Convention, commits: &[ChangelogEntry]) -> BumpType {
    let mut current = BumpType::None;
    for commit in commits {
        let mut message = commit.commit_type.clone();
        if !commit.scope.is_empty() {
            message.push('(');
            message.push_str(&commit.scope);
            message.push(')');
        }
        if commit.breaking_change {
            message.push('!');
        }
        message.push_str(": ");
        message.push_str(&commit.description);
        if !commit.body.is_empty() {
            message = format!("{message}\n\n{}", commit.body);
        }
        let bump = convention.bump_for_message(&message);
        if bump_rank(&bump) > bump_rank(&current) {
            current = bump;
        }
    }
    if matches!(current, BumpType::None) {
        BumpType::Patch
    } else {
        current
    }
}

fn bump_rank(bump: &BumpType) -> usize {
    match bump {
        BumpType::Major => 3,
        BumpType::Minor => 2,
        BumpType::Patch => 1,
        BumpType::None => 0,
    }
}

fn bump_label(bump: &BumpType) -> &'static str {
    match bump {
        BumpType::Major => "major",
        BumpType::Minor => "minor",
        BumpType::Patch => "patch",
        BumpType::None => "none",
    }
}

fn calculate_next_version(
    current_version: &str,
    bump: BumpType,
    prerelease: bool,
    prerelease_suffix: &str,
) -> Result<String, CliError> {
    let parsed = Version::parse(current_version.trim_start_matches('v'))
        .map_err(|e| CliError::SemVerError(e.to_string()))?;
    if prerelease {
        if !parsed.pre.is_empty() {
            let next = increment_existing_prerelease(&parsed, prerelease_suffix)?;
            return Ok(next.to_string());
        }
        let bumped = apply_bump(parsed, bump);
        let mut next = bumped.clone();
        next.pre = Prerelease::new(&format!("{prerelease_suffix}.0"))
            .map_err(|e| CliError::SemVerError(e.to_string()))?;
        return Ok(next.to_string());
    }
    if !parsed.pre.is_empty() {
        let mut stable = parsed.clone();
        stable.pre = Prerelease::EMPTY;
        return Ok(stable.to_string());
    }
    Ok(apply_bump(parsed, bump).to_string())
}

fn increment_existing_prerelease(
    parsed: &Version,
    prerelease_suffix: &str,
) -> Result<Version, CliError> {
    let mut next = parsed.clone();
    let pre = parsed.pre.as_str();
    let prefix = format!("{prerelease_suffix}.");
    if let Some(number) = pre.strip_prefix(&prefix) {
        let incremented = number.parse::<u64>().unwrap_or(0) + 1;
        next.pre = Prerelease::new(&format!("{prerelease_suffix}.{incremented}"))
            .map_err(|e| CliError::SemVerError(e.to_string()))?;
        Ok(next)
    } else {
        next.pre = Prerelease::new(&format!("{prerelease_suffix}.0"))
            .map_err(|e| CliError::SemVerError(e.to_string()))?;
        Ok(next)
    }
}

fn apply_bump(mut version: Version, bump: BumpType) -> Version {
    match bump {
        BumpType::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
        BumpType::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        BumpType::Patch => version.patch += 1,
        BumpType::None => {}
    }
    version
}

pub fn render_release_template(template: &str, version: &str) -> Result<String, CliError> {
    let mut env = Environment::new();
    env.add_template("release", template)
        .map_err(|e| CliError::Generic(e.to_string()))?;
    env.get_template("release")
        .map_err(|e| CliError::Generic(e.to_string()))?
        .render(context! {
            version => version,
        })
        .map(|rendered| rendered.trim().to_string())
        .map_err(|e| CliError::Generic(e.to_string()))
}
