use assert_cmd::Command;
use git2::{Repository, Signature};
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn setup_test_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Configure test user
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // Create initial commit
    let signature = Signature::now("Test User", "test@example.com").unwrap();
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

    dir
}

#[test]
fn test_tag_with_message() {
    let dir = setup_test_repo();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("tag")
        .arg("--name")
        .arg("v1.0.0")
        .arg("--tag-message")
        .arg("First release");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Tag v1.0.0 created successfully!"));

    // Verify tag exists
    let repo = Repository::open(dir.path()).unwrap();
    assert!(repo.revparse_single("refs/tags/v1.0.0").is_ok());
}

#[test]
fn test_tag_without_message() {
    let dir = setup_test_repo();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("tag")
        .arg("--name")
        .arg("v1.0.0");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Tag v1.0.0 created successfully!"));

    // Verify tag exists
    let repo = Repository::open(dir.path()).unwrap();
    assert!(repo.revparse_single("refs/tags/v1.0.0").is_ok());
}

#[test]
fn test_pre_release_continues_from_highest_version() {
    let dir = setup_test_repo();
    let repo = Repository::open(dir.path()).unwrap();
    let signature = Signature::now("Test User", "test@example.com").unwrap();
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

    // Create a new commit after the tags so the tag command has something to process
    {
        let file_path = dir.path().join("chore.txt");
        fs::write(&file_path, "chore change").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("chore.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "misc: new commit after tags #none",
            &tree,
            &[&parent],
        )
        .unwrap();
    }

    // Run the tag command in pre-release mode (should produce v10.0.0-beta.2)
    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run");
    // Output should show the calculated new tag as v10.0.0-beta.2
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("v10.0.0-beta.2"));
}

#[test]
fn test_tag_with_staged_changes() {
    let dir = setup_test_repo();

    // Create a file with staged changes
    let test_file = dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    let repo = Repository::open(dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("test.txt")).unwrap();
    index.write().unwrap();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("tag")
        .arg("--name")
        .arg("v1.0.0");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Please commit your staged changes before doing that",
    ));
}
