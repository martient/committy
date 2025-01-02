use committy::linter::CommitLinter;
use git2::{Repository, Signature};
use tempfile::TempDir;
mod common;

fn setup_test_repo() -> (TempDir, Repository) {
    let temp_dir = TempDir::new().unwrap();
    let repo = Repository::init(temp_dir.path()).unwrap();

    // Configure test user
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    (temp_dir, repo)
}

fn create_commit(repo: &Repository, message: &str) -> git2::Oid {
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
    .unwrap()
}

fn create_tag(repo: &Repository, commit_id: git2::Oid, tag_name: &str) {
    let obj = repo.find_object(commit_id, None).unwrap();
    let signature = Signature::now("Test User", "test@example.com").unwrap();
    repo.tag(tag_name, &obj, &signature, "test tag", false)
        .unwrap();
}

#[test]
fn test_linter_with_tags() {
    common::setup_test_env();
    let (temp_dir, repo) = setup_test_repo();

    // Create some initial commits and a tag
    let commit1 = create_commit(&repo, "feat: initial commit");
    create_tag(&repo, commit1, "v0.1.0");

    // Create commits after the tag
    create_commit(&repo, "feat: valid feature");
    create_commit(&repo, "invalid commit message");
    create_commit(&repo, "fix(scope): valid fix");

    let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
    let issues = linter.check_commits_since_last_tag().unwrap();

    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("invalid commit message"));
}

#[test]
fn test_linter_with_multiple_tags() {
    common::setup_test_env();
    let (temp_dir, repo) = setup_test_repo();

    // Create initial commits and tag
    let commit1 = create_commit(&repo, "feat: initial commit");
    create_tag(&repo, commit1, "v0.1.0");

    // Create more commits and another tag
    let commit2 = create_commit(&repo, "feat: another feature");
    create_tag(&repo, commit2, "v0.2.0");

    // Create commits after the latest tag
    create_commit(&repo, "feat: valid feature");
    create_commit(&repo, "chore: cleanup");
    create_commit(&repo, "invalid: wrong type");

    let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
    let issues = linter.check_commits_since_last_tag().unwrap();

    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("invalid: wrong type"));
}

#[test]
fn test_linter_with_no_tags() {
    common::setup_test_env();
    let (temp_dir, repo) = setup_test_repo();

    // Create commits without any tags
    create_commit(&repo, "feat: valid feature");
    create_commit(&repo, "fix: valid fix");
    create_commit(&repo, "docs: valid docs");
    create_commit(&repo, "invalid message");

    let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
    let issues = linter.check_commits_since_last_tag().unwrap();

    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("invalid message"));
}

#[test]
fn test_linter_with_complex_scopes() {
    common::setup_test_env();
    let (temp_dir, repo) = setup_test_repo();

    create_commit(&repo, "feat(api): valid feature with scope");
    create_commit(&repo, "fix(core-module): valid fix with complex scope");
    create_commit(&repo, "feat(api: invalid scope");
    create_commit(&repo, "fix(api)): extra closing parenthesis");

    let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
    let issues = linter.check_commits_since_last_tag().unwrap();

    assert_eq!(issues.len(), 2);
    assert!(issues
        .iter()
        .any(|i| i.issue.contains("Unclosed scope parenthesis")));
    assert!(issues.iter().any(|i| i.message.contains("fix(api))")));
}

#[test]
fn test_linter_with_empty_repo() {
    common::setup_test_env();
    let (temp_dir, _repo) = setup_test_repo();

    let linter = CommitLinter::new(temp_dir.path().to_str().unwrap()).unwrap();
    let issues = linter.check_commits_since_last_tag().unwrap();

    assert!(issues.is_empty());
}
