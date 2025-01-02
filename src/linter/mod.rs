use anyhow::{anyhow, Result};
use git2::{ObjectType, Repository, Tag};
use regex::Regex;

pub struct CommitLinter {
    repo: Repository,
}

#[derive(Debug)]
pub struct CommitIssue {
    pub commit_id: String,
    pub message: String,
    pub issue: String,
}

impl CommitLinter {
    pub fn new(repo_path: &str) -> Result<Self> {
        let repo = Repository::open(repo_path)?;
        Ok(CommitLinter { repo })
    }

    pub fn check_commits_since_last_tag(&self) -> Result<Vec<CommitIssue>> {
        let mut issues = Vec::new();

        // Get the last tag
        let last_tag = self.get_last_tag()?;
        let tag_commit = match last_tag {
            Some(tag) => tag.target()?.peel_to_commit()?,
            None => return Err(anyhow!("No tags found in repository")),
        };

        // Get HEAD commit
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;

        // Create revwalk
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(head_commit.id())?;
        revwalk.hide(tag_commit.id())?;

        // Conventional commit regex
        let commit_regex = Regex::new(
            r"^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(\([a-z0-9-]+\))?: .+",
        )
        .unwrap();

        // Check each commit
        for commit_id in revwalk {
            let commit_id = commit_id?;
            let commit = self.repo.find_commit(commit_id)?;

            let message = commit.message().unwrap_or("").trim();

            // Check if commit message follows conventional commit format
            if !commit_regex.is_match(message) {
                issues.push(CommitIssue {
                    commit_id: commit_id.to_string(),
                    message: message.to_string(),
                    issue: "Commit message does not follow conventional commit format".to_string(),
                });
                continue;
            }

            // Check minimum length
            if message.len() < 10 {
                issues.push(CommitIssue {
                    commit_id: commit_id.to_string(),
                    message: message.to_string(),
                    issue: "Commit message is too short".to_string(),
                });
            }

            // Check maximum length of first line
            if let Some(first_line) = message.lines().next() {
                if first_line.len() > 72 {
                    issues.push(CommitIssue {
                        commit_id: commit_id.to_string(),
                        message: message.to_string(),
                        issue:
                            "First line of commit message is too long (should be <= 72 characters)"
                                .to_string(),
                    });
                }
            }
        }

        Ok(issues)
    }

    fn get_last_tag(&self) -> Result<Option<Tag>> {
        let mut tags = Vec::new();
        self.repo.tag_foreach(|id, _| {
            if let Ok(obj) = self.repo.find_object(id, Some(ObjectType::Tag)) {
                if let Ok(tag) = obj.into_tag() {
                    tags.push(tag);
                }
            }
            true
        })?;

        // Sort tags by time
        tags.sort_by_key(|b| b.tagger().unwrap().when());

        Ok(tags.into_iter().next())
    }
}
