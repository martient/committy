use anyhow::Result;
use git2::{Oid, Repository};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use crate::convention::Convention;

pub struct CommitLinter {
    repo: Repository,
    convention: Convention,
}

#[derive(Debug, Serialize)]
pub struct CommitIssue {
    pub commit_id: String,
    pub message: String,
    pub issue: String,
}

impl CommitLinter {
    pub fn new(repo_path: &str) -> Result<Self> {
        let repo = Repository::open(repo_path)?;
        let convention = repo
            .workdir()
            .map(Convention::load)
            .transpose()?
            .unwrap_or_else(|| Convention::from_config(Default::default()).unwrap());
        Ok(Self { repo, convention })
    }

    pub fn check_commits_since_last_tag(&self) -> Result<Vec<CommitIssue>> {
        self.check_rev_range(None, None)
    }

    pub fn check_rev_range(
        &self,
        from_ref: Option<&str>,
        to_ref: Option<&str>,
    ) -> Result<Vec<CommitIssue>> {
        let mut issues = Vec::new();
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let target_commit = if let Some(to_ref) = to_ref {
            self.repo.revparse_single(to_ref)?.peel_to_commit()?
        } else {
            head.peel_to_commit()?
        };

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(target_commit.id())?;

        if let Some(from_ref) = from_ref {
            if let Ok(obj) = self.repo.revparse_single(from_ref) {
                if let Ok(commit) = obj.peel_to_commit() {
                    revwalk.hide(commit.id())?;
                }
            }
        } else if let Some(tag_commit) = self.get_last_tag_commit(target_commit.id())? {
            revwalk.hide(tag_commit)?;
        }

        for commit_id in revwalk {
            let commit_id = commit_id?;
            let commit = self.repo.find_commit(commit_id)?;
            let message = commit.message().unwrap_or("").trim();
            for issue in self.convention.validate_message(message) {
                issues.push(CommitIssue {
                    commit_id: commit_id.to_string(),
                    message: message.to_string(),
                    issue,
                });
            }
        }

        Ok(issues)
    }

    fn get_last_tag_commit(&self, head_commit: Oid) -> Result<Option<Oid>> {
        let mut tag_commits: HashMap<Oid, Vec<String>> = HashMap::new();
        let tag_names = self.repo.tag_names(None)?;

        for tag_name in tag_names.iter().flatten() {
            let ref_name = format!("refs/tags/{tag_name}");
            let obj = match self.repo.revparse_single(&ref_name) {
                Ok(obj) => obj,
                Err(_) => continue,
            };
            let commit = match obj.peel_to_commit() {
                Ok(commit) => commit,
                Err(_) => continue,
            };

            if commit.id() == head_commit
                || self.repo.graph_descendant_of(head_commit, commit.id())?
            {
                tag_commits
                    .entry(commit.id())
                    .or_default()
                    .push(tag_name.to_string());
            }
        }

        if tag_commits.is_empty() {
            return Ok(None);
        }

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(head_commit)?;
        for commit_id in revwalk {
            let commit_id = commit_id?;
            if tag_commits.contains_key(&commit_id) {
                return Ok(Some(commit_id));
            }
        }
        Ok(None)
    }
}

#[allow(dead_code)]
pub fn check_message_format(message: &str) -> Vec<String> {
    Convention::from_config(Default::default())
        .map(|convention| convention.validate_message(message))
        .unwrap_or_else(|_| vec!["Invalid convention configuration".to_string()])
}

pub fn check_message_format_for_repo(repo_path: &Path, message: &str) -> Result<Vec<String>> {
    let convention = Convention::load(repo_path)?;
    Ok(convention.validate_message(message))
}

pub fn allowed_commit_types_for_repo(repo_path: &Path) -> Result<Vec<String>> {
    let convention = Convention::load(repo_path)?;
    Ok(convention.allowed_types())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        (temp_dir, repo)
    }

    fn create_commit(repo: &Repository, message: &str) {
        let signature = Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();

        let parent_commit;
        let parents = if let Ok(head) = repo.head() {
            parent_commit = repo.find_commit(head.target().unwrap()).unwrap();
            vec![&parent_commit]
        } else {
            vec![]
        };

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .unwrap();
    }

    #[test]
    fn test_valid_commit_message() {
        let (_temp_dir, repo) = setup_test_repo();
        create_commit(&repo, "feat(api): add new endpoint");
        let linter = CommitLinter::new(repo.workdir().unwrap().to_str().unwrap()).unwrap();
        assert!(linter.check_commits_since_last_tag().unwrap().is_empty());
    }
}
