use std::env;

use crate::{config, error::CliError};
use git2::{FetchOptions, Oid, RemoteCallbacks, Repository};
use log::{debug, error, info};
use regex::Regex;
use semver::Version;
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
    source: String,

    #[structopt(long, help = "Perform a dry run without creating tags")]
    dry_run: bool,

    #[structopt(long, help = "Use Git API for tagging")]
    git_api_tagging: bool,

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
    force_without_changes: bool,

    #[structopt(long, help = "Custom tag message")]
    tag_message: Option<String>,

    #[structopt(long, help = "Do not publish the new tag")]
    not_publish: bool,
}

pub struct TagGenerator {
    default_bump: String,
    not_with_v: bool,
    release_branches: Vec<String>,
    source: String,
    dry_run: bool,
    git_api_tagging: bool,
    initial_version: String,
    prerelease: bool,
    suffix: String,
    none_string_token: String,
    force_without_changes: bool,
    tag_message: String,
    not_publish: bool,
}

impl TagGenerator {
    pub fn new(options: TagGeneratorOptions) -> Self {
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
            git_api_tagging: options.git_api_tagging,
            initial_version: options.initial_version,
            prerelease: options.prerelease,
            suffix: options.prerelease_suffix,
            none_string_token: options.none_string_token,
            force_without_changes: options.force_without_changes,
            tag_message: options.tag_message.unwrap_or_default(),
            not_publish: options.not_publish,
        }
    }

    pub fn run(&self) -> Result<(), CliError> {
        info!("🚀 Starting tag generation process");
        let repo = self.open_repository()?;
        let current_branch = self.get_current_branch(&repo)?;
        let pre_release = if !self.prerelease {
            self.is_pre_release(&current_branch)
        } else {
            self.prerelease
        };

        info!("📊 Current branch: {}", current_branch);
        info!(
            "🏷️ Pre-release mode: {}",
            if pre_release { "Yes" } else { "No" }
        );
        debug!("Current branch: {}", current_branch);
        debug!("Is pre-release: {}", pre_release);

        info!("🔄 Fetching tags from remote");
        self.fetch_tags(&repo)?;

        let (tag, pre_tag) = self.get_latest_tags(&repo)?;
        let tag_commit = self.get_commit_for_tag(&repo, &tag)?;
        let current_commit = self.get_current_commit(&repo)?;

        info!(
            "📌 Latest tag: {}, Latest pre-release tag: {}",
            tag, pre_tag
        );

        if self.should_skip_tagging(tag_commit, current_commit) {
            info!("⏭️ No new commits since previous tag. Skipping...");
            return Ok(());
        }

        let new_tag = self.calculate_new_tag(&repo, &tag, &pre_tag, pre_release)?;
        info!("🆕 Calculated new tag: {}", new_tag);

        if self.dry_run {
            info!("🧪 Dry run: New tag would be {}", new_tag);
            return Ok(());
        }

        self.create_and_push_tag(&repo, &new_tag)?;
        Ok(())
    }

    fn open_repository(&self) -> Result<Repository, CliError> {
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
        let mut remote = repo.find_remote("origin")?;

        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                std::path::Path::new(&format!("{}/.ssh/id_rsa", std::env::var("HOME").unwrap())),
                None,
            )
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote.fetch(&["refs/tags/*:refs/tags/*"], Some(&mut fetch_options), None)
            .map_err(|e| {
                error!("Failed to fetch tags from remote: {}", e);
                match e.code() {
                    git2::ErrorCode::Auth => {
                        error!("Authentication error. Please ensure your credentials are set up correctly.");
                        error!("For SSH: Ensure your SSH key is added to the ssh-agent or located at ~/.ssh/id_rsa");
                        error!("For HTTPS: Check your Git credential helper or use a personal access token.");
                        error!("Debug info: SSH_AUTH_SOCK={:?}, HOME={:?}", env::var("SSH_AUTH_SOCK"), env::var("HOME"));
                        error!("Remote URL: {:?}", remote.url());
                    },
                    _ => error!("Unexpected error occurred. Please check your network connection and repository permissions."),
                }
                CliError::from(e)
            })
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

        debug!("Latest regular tag: {}", tag);
        debug!("Latest pre-release tag: {}", pre_tag);

        Ok((tag, pre_tag))
    }

    fn compare_versions(&self, a: &str, b: &str) -> std::cmp::Ordering {
        debug!("Comparing versions: {} and {}", a, b);
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

    fn get_commit_for_tag(&self, repo: &Repository, tag: &str) -> Result<Oid, CliError> {
        repo.revparse_single(tag)?
            .peel_to_commit()
            .map(|commit| commit.id())
            .map_err(CliError::from)
    }

    fn get_current_commit(&self, repo: &Repository) -> Result<Oid, CliError> {
        repo.head()?
            .peel_to_commit()
            .map(|commit| commit.id())
            .map_err(CliError::from)
    }

    fn should_skip_tagging(&self, tag_commit: Oid, current_commit: Oid) -> bool {
        tag_commit == current_commit && !self.force_without_changes
    }

    fn calculate_new_tag(
        &self,
        repo: &Repository,
        tag: &str,
        pre_tag: &str,
        pre_release: bool,
    ) -> Result<String, CliError> {
        debug!(
            "Calculating new tag. Current tag: {}, Pre-release tag: {}, Is pre-release: {}",
            tag, pre_tag, pre_release
        );
        let log = self.get_commit_log(repo, tag)?;
        let bump: &str = self.determine_bump(&log)?;

        let mut new_version = Version::parse(tag.trim_start_matches('v'))
            .map_err(|e| CliError::SemVerError(e.to_string()))?;

        self.apply_bump(&mut new_version, bump);

        let new_tag = if pre_release {
            self.calculate_pre_release_tag(&new_version, pre_tag)
        } else {
            new_version.to_string()
        };

        Ok(if !self.not_with_v {
            format!("v{}", new_tag)
        } else {
            new_tag
        })
    }

    fn determine_bump(&self, log: &str) -> Result<&str, CliError> {
        debug!("Determining bump from commit log");
        let major_pattern =
            Regex::new(config::MAJOR_REGEX).map_err(|e| CliError::RegexError(e.to_string()))?;
        let minor_pattern =
            Regex::new(config::MINOR_REGEX).map_err(|e| CliError::RegexError(e.to_string()))?;
        let patch_pattern =
            Regex::new(config::PATCH_REGEX).map_err(|e| CliError::RegexError(e.to_string()))?;

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
        debug!("Applying bump: {} to version: {}", bump, version);
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
        debug!("New version after bump: {}", version);
    }

    fn calculate_pre_release_tag(&self, new_version: &Version, pre_tag: &str) -> String {
        debug!(
            "Calculating pre-release tag. New version: {}, Previous pre-tag: {}",
            new_version, pre_tag
        );
        debug!("{}", &new_version.to_string());
        debug!("{}", pre_tag);

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
        debug!("Getting commit log since tag: {}", tag);
        let tag_commit = self.get_commit_for_tag(repo, tag)?;
        let head_commit = self.get_current_commit(repo)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_commit)?;
        revwalk.hide(tag_commit)?;

        let log = revwalk
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| repo.find_commit(oid).ok())
            .map(|commit| commit.message().unwrap_or("").to_string())
            .collect::<Vec<_>>()
            .join("\n");

        debug!("Commit log length: {} characters", log.len());
        Ok(log)
    }

    fn create_and_push_tag(&self, repo: &Repository, new_tag: &str) -> Result<(), CliError> {
        debug!("Creating and pushing new tag: {}", new_tag);
        let head = repo.head()?.peel_to_commit()?;
        let signature = repo.signature()?;

        if self.tag_message.is_empty() {
            repo.tag(new_tag, &head.into_object(), &signature, "", false)?;
        } else {
            repo.tag(
                new_tag,
                &head.into_object(),
                &signature,
                &self.tag_message,
                true,
            )?;
        }

        if self.not_publish {
            info!("🧪 Skip publishing as requested...")
        } else if self.git_api_tagging {
            unimplemented!("GitHub API tagging not implemented yet");
        } else {
            let mut remote = repo.find_remote("origin")?;
            debug!("Pushing tag {} to remote", new_tag);

            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_url, username_from_url, _allowed_types| {
                git2::Cred::ssh_key(
                    username_from_url.unwrap_or("git"),
                    None,
                    std::path::Path::new(&format!(
                        "{}/.ssh/id_rsa",
                        std::env::var("HOME").unwrap()
                    )),
                    None,
                )
            });

            let mut push_options = git2::PushOptions::new();
            push_options.remote_callbacks(callbacks);

            match remote.push(
                &[&format!("refs/tags/{}", new_tag)],
                Some(&mut push_options),
            ) {
                Ok(_) => debug!("Successfully pushed tag {} to remote", new_tag),
                Err(e) => {
                    error!("Failed to push tag {} to remote: {}", new_tag, e);
                    if e.code() == git2::ErrorCode::Auth {
                        error!(
                            "Authentication error. Please ensure your SSH key is set up correctly."
                        );
                        error!("You may need to add your SSH key to the ssh-agent or use HTTPS with a personal access token.");
                    }
                    return Err(CliError::GitError(e));
                }
            }
        }

        info!("✅ Tag generation completed successfully, {}", new_tag);
        Ok(())
    }
}
