use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;
use git2::{Repository, Signature};

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
    ).unwrap();
    
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
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Please commit your staged changes before doing that"));
}
