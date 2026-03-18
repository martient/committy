use anyhow::Result;
use git2::{Oid, Repository};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::repository::{CommitRulesConfig, RepositoryConfig};

pub struct CommitLinter {
    repo: Repository,
    rules: CommitRulesConfig,
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
        let rules = repo
            .workdir()
            .map(load_commit_rules)
            .transpose()?
            .unwrap_or_default();
        Ok(CommitLinter { repo, rules })
    }

    pub fn check_commits_since_last_tag(&self) -> Result<Vec<CommitIssue>> {
        let mut issues = Vec::new();

        // Get HEAD commit
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
                // Repository is empty, no commits to check
                return Ok(Vec::new());
            }
            Err(e) => return Err(e.into()),
        };

        let head_commit = head.peel_to_commit()?;

        // Create revwalk
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(head_commit.id())?;

        // If there's a tag, only check commits since that tag
        if let Some(tag_commit) = self.get_last_tag_commit(head_commit.id())? {
            revwalk.hide(tag_commit)?;
        }

        // Check each commit
        for commit_id in revwalk {
            let commit_id = commit_id?;
            let commit = self.repo.find_commit(commit_id)?;

            let message = commit.message().unwrap_or("").trim();
            for issue in check_message_format_with_rules(message, &self.rules) {
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

/// Lint a single commit message string using the same rules as repository linting.
/// Returns a list of issue descriptions; empty if the message passes all checks.
#[allow(dead_code)]
pub fn check_message_format(message: &str) -> Vec<String> {
    check_message_format_with_rules(message, &CommitRulesConfig::default())
}

pub fn check_message_format_for_repo(repo_path: &Path, message: &str) -> Result<Vec<String>> {
    let rules = load_commit_rules(repo_path)?;
    Ok(check_message_format_with_rules(message, &rules))
}

pub fn allowed_commit_types_for_repo(repo_path: &Path) -> Result<Vec<String>> {
    let rules = load_commit_rules(repo_path)?;
    Ok(allowed_commit_types(&rules))
}

fn load_commit_rules(repo_path: &Path) -> Result<CommitRulesConfig> {
    let config_root = discover_config_root(repo_path);
    let config_path = RepositoryConfig::get_config_path(&config_root)?;

    if !config_path.exists() {
        return Ok(CommitRulesConfig::default());
    }

    let content = fs::read_to_string(&config_path)?;
    let config: RepositoryConfig = toml::from_str(&content)?;
    Ok(config.commit_rules)
}

fn discover_config_root(repo_path: &Path) -> PathBuf {
    let mut current = if repo_path.is_file() {
        repo_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo_path.to_path_buf())
    } else {
        repo_path.to_path_buf()
    };

    loop {
        if current.join(".committy").join("config.toml").exists() || current.join(".git").exists() {
            return current;
        }

        let Some(parent) = current.parent() else {
            return repo_path.to_path_buf();
        };
        current = parent.to_path_buf();
    }
}

fn check_message_format_with_rules(message: &str, rules: &CommitRulesConfig) -> Vec<String> {
    let mut issues = Vec::new();

    let message = message.trim();
    let first_line = message.lines().next().unwrap_or("");
    let allowed_types = allowed_commit_types(rules);
    let type_pattern = format!(
        r"(?:{})",
        allowed_types
            .iter()
            .map(|item| regex::escape(item))
            .collect::<Vec<_>>()
            .join("|")
    );

    // Conventional commit regex parts
    let scope_pattern = r"(?:\([a-z0-9-]+\))?";
    let breaking_change = r"(?:!)?"; // Optional breaking change indicator
    let separator = r"\: ";
    let description = r".+";
    let full_pattern =
        format!("^{type_pattern}{scope_pattern}{breaking_change}{separator}{description}$");
    let commit_regex = Regex::new(&full_pattern).unwrap();

    // Check if commit message follows conventional commit format
    if !commit_regex.is_match(first_line) {
        let issue = if !first_line.contains(": ") {
            "Missing ': ' separator between type/scope and description".to_string()
        } else if !extract_commit_type(first_line)
            .map(|candidate| {
                allowed_types
                    .iter()
                    .any(|commit_type| commit_type == candidate)
            })
            .unwrap_or(false)
        {
            let types = allowed_types.join(", ");
            format!("Commit type must be one of: {types}")
        } else if first_line.contains("(") && !first_line.contains(")") {
            "Unclosed scope parenthesis".to_string()
        } else if first_line.contains(")") && !first_line.contains("(") {
            "Unopened scope parenthesis".to_string()
        } else if first_line.contains("()") {
            "Empty scope parenthesis".to_string()
        } else {
            "Commit message format should be: <type>(<scope>): <description>".to_string()
        };
        issues.push(issue);
        return issues; // Match behavior of repo linting: when format is invalid, do not report length issues
    }

    // Check minimum length
    if first_line.len() < 10 {
        let len = first_line.len();
        issues.push(format!(
            "Commit message is too short (got {len} characters, minimum is 10)"
        ));
    }

    // Check maximum length of first line
    if first_line.len() > rules.max_subject_length {
        let len = first_line.len();
        issues.push(format!(
            "First line of commit message is too long (got {len} characters, maximum is {})",
            rules.max_subject_length
        ));
    }

    let body_lines: Vec<_> = message.lines().skip(1).collect();
    let has_body_content = body_lines.iter().any(|line| !line.trim().is_empty());

    if rules.require_body && !has_body_content {
        issues.push("Commit body is required by repository configuration".to_string());
    }

    for (idx, line) in body_lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > rules.max_body_line_length {
            issues.push(format!(
                "Body line {} is too long (got {} characters, maximum is {})",
                idx + 2,
                line.len(),
                rules.max_body_line_length
            ));
        }
    }

    issues
}

fn allowed_commit_types(rules: &CommitRulesConfig) -> Vec<String> {
    let mut types = if rules.allowed_types.is_empty() {
        crate::config::COMMIT_TYPES
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
    } else {
        rules.allowed_types.clone()
    };

    for custom in &rules.custom_types {
        if !types.iter().any(|item| item == &custom.name) {
            types.push(custom.name.clone());
        }
    }

    types
}

fn extract_commit_type(first_line: &str) -> Option<&str> {
    let separator_index = first_line.find(": ")?;
    let header = &first_line[..separator_index];
    let type_end = header.find(['(', '!']).unwrap_or(header.len());
    Some(&header[..type_end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};

    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, Repository) {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Configure test user
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
        let (temp_dir, repo) = setup_test_repo();

        // Test regular commit
        create_commit(&repo, "feat(api): add new endpoint");
        // Test commit with breaking change
        create_commit(&repo, "feat(api)!: breaking change");
        // Test commit without scope
        create_commit(&repo, "docs: update readme");

        let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
        let issues = linter.check_commits_since_last_tag().unwrap();
        assert!(issues.is_empty(), "Expected no issues but got: {issues:?}");
    }

    #[test]
    fn test_invalid_commit_type() {
        let (temp_dir, repo) = setup_test_repo();
        create_commit(&repo, "invalid: this should fail");

        let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
        let issues = linter.check_commits_since_last_tag().unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].issue.contains("Commit type must be one of:"));
    }

    #[test]
    fn test_missing_separator() {
        let (temp_dir, repo) = setup_test_repo();
        create_commit(&repo, "feat missing separator");

        let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
        let issues = linter.check_commits_since_last_tag().unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].issue.contains("Missing ': ' separator"));
    }

    #[test]
    fn test_invalid_scope_parentheses() {
        let (temp_dir, repo) = setup_test_repo();
        create_commit(&repo, "feat(: missing closing parenthesis");

        let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
        let issues = linter.check_commits_since_last_tag().unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].issue.contains("Unclosed scope parenthesis"));
    }

    #[test]
    fn test_empty_scope() {
        let (temp_dir, repo) = setup_test_repo();
        create_commit(&repo, "feat(): empty scope");

        let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
        let issues = linter.check_commits_since_last_tag().unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].issue.contains("Empty scope parenthesis"));
    }

    #[test]
    fn test_multiple_commits() {
        let (temp_dir, repo) = setup_test_repo();
        create_commit(&repo, "feat: valid commit");
        create_commit(&repo, "invalid: invalid type");
        create_commit(&repo, "fix(): empty scope");

        let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
        let issues = linter.check_commits_since_last_tag().unwrap();
        assert_eq!(issues.len(), 2);
    }
}
