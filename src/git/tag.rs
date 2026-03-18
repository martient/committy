use crate::version::VersionManager;
use crate::{config, error::CliError};
use git2::{Oid, Repository};
use log::{debug, error, info};
use regex::Regex;
use semver::Version;
use std::path::Path;
use std::process::Command;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub struct TagGeneratorOptions {
    #[structopt(long, default_value = "minor", help = "Default bump strategy")]
    default_bump: String,

    #[structopt(long, help = "Without the prefix 'v'")]
    not_with_v: bool,

    #[structopt(
        long,
        default_value = "master,main",
        help = "Comma-separated list of release branches"
    )]
    release_branches: String,

    #[structopt(long, default_value = ".", help = "Source directory")]
    pub(crate) source: String,

    #[structopt(long, help = "Perform a dry run without creating tags")]
    dry_run: bool,

    #[structopt(
        long,
        default_value = "0.0.0",
        help = "Initial version if no tags exist"
    )]
    initial_version: String,

    #[structopt(long, help = "Create a pre-release version")]
    prerelease: bool,

    #[structopt(long, default_value = "beta", help = "Pre-release suffix")]
    prerelease_suffix: String,

    #[structopt(
        long,
        default_value = "#none",
        help = "Token to indicate no version bump"
    )]
    none_string_token: String,

    #[structopt(long, help = "Force tag creation even without changes")]
    force_without_change: bool,

    #[structopt(long, help = "Custom tag message")]
    tag_message: Option<String>,

    #[structopt(long, help = "Do not publish the new tag")]
    not_publish: bool,

    #[structopt(long, help = "Publish the new tag after calculation")]
    publish: bool,

    #[structopt(long, help = "Confirm publishing the tag to remote")]
    confirm_publish: bool,

    #[structopt(long, help = "Fetch tags from remote before calculation")]
    fetch: bool,

    #[structopt(
        long = "no-fetch",
        help = "Do not fetch tags from remote before calculation"
    )]
    no_fetch: bool,
}

impl TagGeneratorOptions {
    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn publish_requested(&self) -> bool {
        self.publish
    }

    pub fn confirm_publish(&self) -> bool {
        self.confirm_publish
    }

    pub fn will_publish_remote(&self) -> bool {
        self.publish && self.confirm_publish && !self.not_publish
    }
}

pub struct TagGenerator {
    default_bump: String,
    not_with_v: bool,
    release_branches: Vec<String>,
    source: String,
    dry_run: bool,
    initial_version: String,
    prerelease: bool,
    suffix: String,
    none_string_token: String,
    force_without_change: bool,
    tag_message: String,
    not_publish: bool,
    publish_remote: bool,
    fetch: bool,
    bump_config_files: bool,
    pub current_tag: String,
    pub new_tag: String,
    pub is_pre_release: bool,
}

impl TagGenerator {
    pub fn new(options: TagGeneratorOptions, allow_bump_config_files: bool) -> Self {
        TagGenerator {
            default_bump: options.default_bump,
            not_with_v: options.not_with_v,
            release_branches: options
                .release_branches
                .split(',')
                .map(String::from)
                .collect(),
            source: options.source,
            dry_run: options.dry_run,
            initial_version: options.initial_version,
            prerelease: options.prerelease,
            suffix: options.prerelease_suffix,
            none_string_token: options.none_string_token,
            force_without_change: options.force_without_change,
            tag_message: options.tag_message.unwrap_or_default(),
            not_publish: options.not_publish || !options.publish,
            publish_remote: options.publish && options.confirm_publish && !options.not_publish,
            // default to fetching unless --no-fetch is explicitly passed; --fetch enforces true
            fetch: if options.fetch {
                true
            } else {
                !options.no_fetch
            },
            bump_config_files: allow_bump_config_files,
            current_tag: String::new(),
            new_tag: String::new(),
            is_pre_release: false,
        }
    }

    fn should_fetch(&self) -> bool {
        self.fetch
    }

    fn should_publish_remote(&self) -> bool {
        self.publish_remote && !self.not_publish
    }

    pub fn run(&mut self) -> Result<(), CliError> {
        info!("🚀 Starting tag generation process");
        let repo = self.open_repository()?;
        let current_branch = self.get_current_branch(&repo)?;
        let pre_release = if !self.prerelease {
            self.is_pre_release(&current_branch)
        } else {
            self.prerelease
        };

        info!("📊 Current branch: {current_branch}");
        info!(
            "🏷️ Pre-release mode: {}",
            if pre_release { "Yes" } else { "No" }
        );
        debug!("Current branch: {current_branch}");
        debug!("Is pre-release: {pre_release}");

        self.current_tag = current_branch.clone();
        self.is_pre_release = pre_release;

        if self.should_fetch() {
            info!("🔄 Fetching tags from remote");
            self.fetch_tags(&repo)?;
        } else {
            debug!("Skipping remote tag fetch (fetch flag not set)");
        }

        let (tag, pre_tag) = self.get_latest_tags(&repo)?;
        let tag_commit = self.get_commit_for_tag(&repo, &tag)?;
        let current_commit = self.get_current_commit(&repo)?;

        info!("📌 Latest tag: {tag}, Latest pre-release tag: {pre_tag}");

        if self.should_skip_tagging(tag_commit, current_commit) {
            info!("⏭️ No new commits since previous tag. Skipping...");
            return Ok(());
        }

        self.new_tag = self.calculate_new_tag(&repo, &tag, &pre_tag, pre_release)?;
        info!("🆕 Calculated new tag: {}", self.new_tag);

        if self.dry_run {
            info!("🧪 Dry run: New tag would be {}", self.new_tag);
            return Ok(());
        }

        // Update version files and commit changes
        if self.bump_config_files {
            let updated_files = self.update_versions(&self.new_tag)?;
            if !updated_files.is_empty() {
                info!("📝 Updated version in files: {}", updated_files.join(", "));
                self.commit_version_changes(&repo, &self.new_tag, &updated_files)?;
                info!("✅ Committed version changes");
            }
        }

        self.create_and_push_tag(&repo, &self.new_tag)?;
        Ok(())
    }

    pub fn open_repository(&self) -> Result<Repository, CliError> {
        Repository::open(&self.source).map_err(CliError::from)
    }

    fn get_current_branch(&self, repo: &Repository) -> Result<String, CliError> {
        repo.head()?
            .shorthand()
            .map(String::from)
            .ok_or_else(|| CliError::Generic("Failed to get current branch".to_string()))
    }

    fn is_pre_release(&self, current_branch: &str) -> bool {
        !self.release_branches.iter().any(|b| {
            current_branch == b
                || (b.contains('*') && current_branch.starts_with(b.trim_end_matches('*')))
        })
    }

    fn fetch_tags(&self, repo: &Repository) -> Result<(), CliError> {
        debug!("Fetching tags from remote");
        match repo.find_remote("origin") {
            Ok(_) => {
                let repo_path = self.repo_root(repo)?;
                run_git(
                    repo_path,
                    &["fetch", "origin", "refs/tags/*:refs/tags/*"],
                    "fetch tags from remote",
                )
                .inspect_err(|e| error!("{e}"))
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => {
                debug!("No remote 'origin' found, skipping tag fetch");
                Ok(())
            }
            Err(e) => Err(CliError::from(e)),
        }
    }

    fn get_latest_tags(&self, repo: &Repository) -> Result<(String, String), CliError> {
        debug!("Getting latest tags");
        let tag_regex = regex::Regex::new(r"^v?[0-9]+\.[0-9]+\.[0-9]+$").unwrap();
        let pre_tag_regex = regex::Regex::new(&format!(
            r"^v?[0-9]+\.[0-9]+\.[0-9]+(-{}\.{{0,1}}[0-9]+)$",
            self.suffix
        ))
        .unwrap();

        let mut tags = repo
            .tag_names(None)?
            .iter()
            .filter_map(|t| t.map(String::from))
            .collect::<Vec<_>>();

        tags.sort_by(|a, b| self.compare_versions(b, a)); // Reverse the comparison order

        let tag = tags
            .iter()
            .find(|t| tag_regex.is_match(t))
            .cloned()
            .unwrap_or_else(|| self.initial_version.clone());
        let pre_tag = tags
            .iter()
            .find(|t| pre_tag_regex.is_match(t))
            .cloned()
            .unwrap_or_else(|| self.initial_version.clone());

        debug!("Latest regular tag: {tag}");
        debug!("Latest pre-release tag: {pre_tag}");

        Ok((tag, pre_tag))
    }

    fn compare_versions(&self, a: &str, b: &str) -> std::cmp::Ordering {
        debug!("Comparing versions: {a} and {b}");
        if a.contains("none") || b.contains("none") {
            return a.cmp(b);
        }
        match (
            Version::parse(a.trim_start_matches('v')),
            Version::parse(b.trim_start_matches('v')),
        ) {
            (Ok(a_version), Ok(b_version)) => a_version.cmp(&b_version),
            _ => a.cmp(b),
        }
    }

    fn get_commit_for_tag(&self, repo: &Repository, tag: &str) -> Result<Option<Oid>, CliError> {
        Ok(repo
            .revparse_single(tag)
            .ok()
            .and_then(|obj| obj.peel_to_commit().ok())
            .map(|commit| commit.id()))
    }

    fn get_current_commit(&self, repo: &Repository) -> Result<Oid, CliError> {
        repo.head()?
            .peel_to_commit()
            .map(|commit| commit.id())
            .map_err(CliError::from)
    }

    fn should_skip_tagging(&self, tag_commit: Option<Oid>, current_commit: Oid) -> bool {
        tag_commit.is_some_and(|commit| commit == current_commit && !self.force_without_change)
    }

    fn calculate_new_tag(
        &self,
        repo: &Repository,
        tag: &str,
        pre_tag: &str,
        pre_release: bool,
    ) -> Result<String, CliError> {
        debug!(
            "Calculating new tag. Current tag: {tag}, Pre-release tag: {pre_tag}, Is pre-release: {pre_release}"
        );
        use semver::Version as SemverVersion;

        if pre_release {
            // Parse both tags
            let reg_ver = SemverVersion::parse(tag.trim_start_matches('v'))
                .unwrap_or_else(|_| SemverVersion::new(0, 0, 0));
            let pre_ver = SemverVersion::parse(
                pre_tag
                    .trim_start_matches('v')
                    .split('-')
                    .next()
                    .unwrap_or(""),
            )
            .unwrap_or_else(|_| SemverVersion::new(0, 0, 0));

            // If pre_tag version is higher than regular tag, we're already on a pre-release
            // In this case, only increment the pre-release counter, don't apply bump
            if pre_ver > reg_ver {
                debug!("Pre-release tag {pre_tag} is ahead of regular tag {tag}, incrementing pre-release counter only");
                let log = self.get_commit_log(repo, pre_tag)?;

                // Check if there are any commits - if not, no new tag needed
                if log.trim().is_empty() {
                    return Err(CliError::Generic(
                        "No new commits since last pre-release tag".to_string(),
                    ));
                }

                // Just increment the pre-release counter
                let new_tag = self.calculate_pre_release_tag(&pre_ver, pre_tag);
                return Ok(if !self.not_with_v {
                    format!("v{new_tag}")
                } else {
                    new_tag
                });
            }

            // Pre-release is not ahead, apply bump from regular tag
            debug!("Starting new pre-release from regular tag {tag}");
            let log = self.get_commit_log(repo, tag)?;
            let bump: &str = self.determine_bump(&log)?;
            let mut new_version = SemverVersion::parse(tag.trim_start_matches('v'))
                .map_err(|e| CliError::SemVerError(e.to_string()))?;
            self.apply_bump(&mut new_version, bump);

            let new_tag = self.calculate_pre_release_tag(&new_version, pre_tag);
            Ok(if !self.not_with_v {
                format!("v{new_tag}")
            } else {
                new_tag
            })
        } else {
            // Regular release
            // Parse both tags to compare versions
            let reg_ver = SemverVersion::parse(tag.trim_start_matches('v'))
                .unwrap_or_else(|_| SemverVersion::new(0, 0, 0));
            let pre_ver = SemverVersion::parse(
                pre_tag
                    .trim_start_matches('v')
                    .split('-')
                    .next()
                    .unwrap_or(""),
            )
            .unwrap_or_else(|_| SemverVersion::new(0, 0, 0));

            // If pre-release version is higher than stable tag, promote it to stable
            if pre_ver > reg_ver {
                debug!(
                    "Pre-release tag {pre_tag} is ahead of regular tag {tag}, promoting to stable"
                );
                // Just remove the pre-release suffix to promote to stable
                Ok(if !self.not_with_v {
                    format!("v{}", pre_ver)
                } else {
                    pre_ver.to_string()
                })
            } else {
                // Normal bump from stable tag
                let log = self.get_commit_log(repo, tag)?;
                let bump: &str = self.determine_bump(&log)?;
                let mut new_version = SemverVersion::parse(tag.trim_start_matches('v'))
                    .map_err(|e| CliError::SemVerError(e.to_string()))?;
                self.apply_bump(&mut new_version, bump);

                Ok(if !self.not_with_v {
                    format!("v{}", new_version)
                } else {
                    new_version.to_string()
                })
            }
        }
    }

    fn determine_bump(&self, log: &str) -> Result<&str, CliError> {
        debug!("Determining bump from commit log");
        let cfg = config::Config::load().unwrap_or_default();
        let major_pattern =
            Regex::new(&cfg.major_regex).map_err(|e| CliError::RegexError(e.to_string()))?;
        let minor_pattern =
            Regex::new(&cfg.minor_regex).map_err(|e| CliError::RegexError(e.to_string()))?;
        let patch_pattern =
            Regex::new(&cfg.patch_regex).map_err(|e| CliError::RegexError(e.to_string()))?;

        if major_pattern.is_match(log) {
            Ok("major")
        } else if minor_pattern.is_match(log) {
            Ok("minor")
        } else if patch_pattern.is_match(log) {
            Ok("patch")
        } else if log.contains(&self.none_string_token) {
            Ok("none")
        } else {
            Ok(&self.default_bump)
        }
    }

    fn apply_bump(&self, version: &mut Version, bump: &str) {
        debug!("Applying bump: {bump} to version: {version}");
        match bump {
            "major" => {
                version.major += 1;
                version.minor = 0;
                version.patch = 0;
            }
            "minor" => {
                version.minor += 1;
                version.patch = 0;
            }
            "patch" => version.patch += 1,
            _ => {}
        }
        debug!("New version after bump: {version}");
    }

    fn update_versions(&self, new_version: &str) -> Result<Vec<String>, CliError> {
        let repo = self.open_repository()?;
        let repo_path = repo.workdir().ok_or_else(|| {
            let err = git2::Error::new(
                git2::ErrorCode::NotFound,
                git2::ErrorClass::Repository,
                "Repository has no working directory",
            );
            CliError::GitError(err)
        })?;

        // Change to the repository directory
        let old_dir = std::env::current_dir().map_err(CliError::IoError)?;
        std::env::set_current_dir(repo_path).map_err(CliError::IoError)?;

        let mut version_manager = VersionManager::new();
        version_manager.register_common_files()?;

        // Update all version files
        let updated_files = version_manager.update_all_versions(new_version)?;

        // Change back to the original directory
        std::env::set_current_dir(old_dir).map_err(CliError::IoError)?;

        // Convert PathBuf to String
        let updated_files: Vec<String> = updated_files
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        Ok(updated_files)
    }

    fn calculate_pre_release_tag(&self, new_version: &Version, pre_tag: &str) -> String {
        debug!(
            "Calculating pre-release tag. New version: {new_version}, Previous pre-tag: {pre_tag}"
        );
        debug!("{new_version}");
        debug!("{pre_tag}");

        let version_string = new_version.to_string();
        let pre_tag_without_v = pre_tag.trim_start_matches('v');

        if pre_tag_without_v.starts_with(&version_string) {
            let pre_release_regex =
                regex::Regex::new(&format!(r"-{}\.(\d+)$", self.suffix)).unwrap();
            if let Some(captures) = pre_release_regex.captures(pre_tag_without_v) {
                if let Some(pre_release_num) = captures.get(1) {
                    let next_num = pre_release_num.as_str().parse::<u64>().unwrap_or(0) + 1;
                    return format!("{}-{}.{}", new_version, self.suffix, next_num);
                }
            }
        }
        format!("{}-{}.0", new_version, self.suffix)
    }

    fn get_commit_log(&self, repo: &Repository, tag: &str) -> Result<String, CliError> {
        debug!("Getting commit log since tag: {tag}");
        let tag_commit = self.get_commit_for_tag(repo, tag)?;
        let head_commit = self.get_current_commit(repo)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_commit)?;
        if let Some(commit) = tag_commit {
            revwalk.hide(commit)?; // Only hide if we have a commit
        }

        let log = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .map(|commit| commit.message().unwrap_or("").to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let len = log.len();
        debug!("Commit log length: {len} characters");
        Ok(log)
    }

    fn commit_version_changes(
        &self,
        repo: &Repository,
        new_version: &str,
        updated_files: &[String],
    ) -> Result<(), CliError> {
        if updated_files.is_empty() {
            return Ok(());
        }

        let repo_path = self.repo_root(repo)?;
        let mut add_args = vec!["add", "--"];
        for file in updated_files {
            add_args.push(file.as_str());
        }
        run_git(repo_path, &add_args, "stage version updates")?;

        let version_without_v = new_version.trim_start_matches('v');
        let message = format!("chore: bump version to {version_without_v}");
        run_git(
            repo_path,
            &["commit", "-m", &message],
            "create version bump commit",
        )?;

        // Push the commit to remote only when publishing has been explicitly confirmed
        if !self.dry_run && self.should_publish_remote() {
            info!("🔄 Pushing version bump commit to remote");
            match repo.find_remote("origin") {
                Ok(_) => {
                    let current_branch = self.get_current_branch(repo)?;
                    run_git(
                        repo_path,
                        &[
                            "push",
                            "origin",
                            &format!("HEAD:refs/heads/{current_branch}"),
                        ],
                        "push version bump commit to remote",
                    )?;
                    debug!("Successfully pushed commit to remote branch {current_branch}");
                    info!("✅ Pushed version bump commit to remote branch {current_branch}");
                }
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    debug!("Remote 'origin' not found, skipping push");
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    pub fn create_and_push_tag(&self, repo: &Repository, new_tag: &str) -> Result<(), CliError> {
        debug!("Creating and pushing new tag: {new_tag}");
        let repo_path = self.repo_root(repo)?;

        let tag_message = if !self.tag_message.is_empty() {
            &self.tag_message
        } else {
            new_tag
        };

        run_git(
            repo_path,
            &["tag", "-a", new_tag, "-m", tag_message],
            "create tag",
        )?;

        // Only try to push when publishing has been explicitly confirmed
        if !self.dry_run && self.should_publish_remote() {
            match repo.find_remote("origin") {
                Ok(_) => {
                    run_git(
                        repo_path,
                        &["push", "origin", &format!("refs/tags/{new_tag}")],
                        "push tag to remote",
                    )?;
                    debug!("Successfully pushed tag {new_tag} to remote");
                }
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    debug!("Remote 'origin' not found, skipping push");
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    fn repo_root<'a>(&self, repo: &'a Repository) -> Result<&'a Path, CliError> {
        repo.workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))
    }
}

fn run_git(repo_path: &Path, args: &[&str], action: &str) -> Result<(), CliError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()
        .map_err(CliError::IoError)?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let detail = if stderr.is_empty() {
        format!("git {:?} failed", args)
    } else {
        stderr
    };
    Err(CliError::Generic(format!("Failed to {action}: {detail}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use tempfile::tempdir;

    #[test]
    fn test_calculate_new_tag_prefers_highest_version() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let signature = Signature::now("Test User", "test@example.com").unwrap();
        // Initial commit
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();
        // Tag v8.3.2 (regular)
        repo.tag(
            "v8.3.2",
            repo.head().unwrap().peel_to_commit().unwrap().as_object(),
            &signature,
            "Regular release",
            false,
        )
        .unwrap();
        // Tag v10.0.0-beta.1 (pre-release)
        repo.tag(
            "v10.0.0-beta.1",
            repo.head().unwrap().peel_to_commit().unwrap().as_object(),
            &signature,
            "Pre-release",
            false,
        )
        .unwrap();

        // Add a commit after the tags
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "chore: another commit",
            &tree,
            &[&parent],
        )
        .unwrap();

        let opts = TagGeneratorOptions {
            default_bump: "minor".to_string(),
            not_with_v: false,
            release_branches: "main,master".to_string(),
            source: ".".to_string(),
            dry_run: true,
            initial_version: "0.0.0".to_string(),
            prerelease: true,
            prerelease_suffix: "beta".to_string(),
            none_string_token: "#none".to_string(),
            force_without_change: false,
            tag_message: None,
            publish: false,
            confirm_publish: false,
            not_publish: true,
            fetch: false,
            no_fetch: true,
        };
        let gen = TagGenerator::new(opts, false);
        let (tag, pre_tag) = gen.get_latest_tags(&repo).unwrap();
        let new_tag = gen.calculate_new_tag(&repo, &tag, &pre_tag, true).unwrap();
        // Should continue from v10.0.0-beta.1, producing v10.0.0-beta.2
        assert!(
            new_tag.contains("v10.0.0-beta.2"),
            "new_tag was: {}",
            new_tag
        );
    }
}
