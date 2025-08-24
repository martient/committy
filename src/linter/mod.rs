use anyhow::Result;
use git2::{ObjectType, Repository, Tag};
use regex::Regex;
use serde::Serialize;

pub struct CommitLinter {
    repo: Repository,
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
        Ok(CommitLinter { repo })
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
        if let Ok(Some(tag)) = self.get_last_tag() {
            let tag_commit = tag.target()?.peel_to_commit()?;
            revwalk.hide(tag_commit.id())?;
        }

        // Conventional commit regex parts
        let type_pattern = format!(r"(?:{})", crate::config::COMMIT_TYPES.join("|"));
        let scope_pattern = r"(?:\([a-z0-9-]+\))?";
        let breaking_change = r"(?:!)?"; // Optional breaking change indicator
        let separator = r"\: ";
        let description = r".+";
        let full_pattern =
            format!("^{type_pattern}{scope_pattern}{breaking_change}{separator}{description}$");
        let commit_regex = Regex::new(&full_pattern).unwrap();

        // Check each commit
        for commit_id in revwalk {
            let commit_id = commit_id?;
            let commit = self.repo.find_commit(commit_id)?;

            let message = commit.message().unwrap_or("").trim();
            let first_line = message.lines().next().unwrap_or("");

            // Check if commit message follows conventional commit format
            if !commit_regex.is_match(first_line) {
                let issue = if !first_line.contains(": ") {
                    "Missing ': ' separator between type/scope and description".to_string()
                } else if !crate::config::COMMIT_TYPES
                    .iter()
                    .any(|t| first_line.starts_with(t))
                {
                    let types = crate::config::COMMIT_TYPES.join(", ");
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

                issues.push(CommitIssue {
                    commit_id: commit_id.to_string(),
                    message: message.to_string(),
                    issue,
                });
                continue;
            }

            // Check minimum length
            if first_line.len() < 10 {
                let len = first_line.len();
                issues.push(CommitIssue {
                    commit_id: commit_id.to_string(),
                    message: message.to_string(),
                    issue: format!(
                        "Commit message is too short (got {len} characters, minimum is 10)"
                    ),
                });
            }

            // Check maximum length of first line
            if first_line.len() > 72 {
                let len = first_line.len();
                issues.push(CommitIssue {
                    commit_id: commit_id.to_string(),
                    message: message.to_string(),
                    issue: format!(
                        "First line of commit message is too long (got {len} characters, maximum is 72)"
                    ),
                });
            }
        }

        Ok(issues)
    }

    fn get_last_tag(&'_ self) -> Result<Option<Tag<'_>>> {
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

/// Lint a single commit message string using the same rules as repository linting.
/// Returns a list of issue descriptions; empty if the message passes all checks.
pub fn check_message_format(message: &str) -> Vec<String> {
    let mut issues = Vec::new();

    let message = message.trim();
    let first_line = message.lines().next().unwrap_or("");

    // Conventional commit regex parts
    let type_pattern = format!(r"(?:{})", crate::config::COMMIT_TYPES.join("|"));
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
        } else if !crate::config::COMMIT_TYPES
            .iter()
            .any(|t| first_line.starts_with(t))
        {
            let types = crate::config::COMMIT_TYPES.join(", ");
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
    if first_line.len() > 72 {
        let len = first_line.len();
        issues.push(format!(
            "First line of commit message is too long (got {len} characters, maximum is 72)"
        ));
    }

    issues
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
