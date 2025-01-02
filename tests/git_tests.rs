mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_git_repo() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    // Initialize git repo
    let _ = StdCommand::new("git")
        .args(["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to initialize git repository");

    // Configure git user
    let _ = StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure git user name");

    let _ = StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure git user email");

    dir
}

fn setup_git_repo_without_config() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    // Initialize git repo without user config
    let _ = StdCommand::new("git")
        .args(["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to initialize git repository");

    // Explicitly unset any existing user config
    let _ = StdCommand::new("git")
        .args(["config", "--local", "--unset", "user.name"])
        .current_dir(&dir)
        .output();

    let _ = StdCommand::new("git")
        .args(["config", "--local", "--unset", "user.email"])
        .current_dir(&dir)
        .output();

    // Also unset any global config for the test
    let _ = StdCommand::new("git")
        .args(["config", "--global", "--unset", "user.name"])
        .current_dir(&dir)
        .output();

    let _ = StdCommand::new("git")
        .args(["config", "--global", "--unset", "user.email"])
        .current_dir(&dir)
        .output();

    dir
}

#[test]
fn test_commit_message_formatting() {
    let temp_dir = setup_git_repo();

    // Create and stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    // Test commit with scope and breaking change
    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--scope")
        .arg("test")
        .arg("--breaking-change")
        .arg("--message")
        .arg("Test commit")
        .arg("--long-message")
        .arg("Detailed description of the change")
        .assert()
        .success()
        .stdout(predicate::str::contains("feat(test)!: Test commit"));

    // Verify git log
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%B", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert!(log_message.contains("feat(test)!: Test commit"));
    assert!(log_message.contains("Detailed description of the change"));
}

#[test]
fn test_unstaged_changes() {
    let temp_dir = setup_git_repo();

    // Create but don't stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Test commit")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No staged changes found"));
}

#[test]
fn test_commit_without_git_config() {
    let temp_dir = setup_git_repo_without_config();

    // Create and stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .env("GIT_COMMITTER_NAME", "")
        .env("GIT_COMMITTER_EMAIL", "")
        .env("GIT_AUTHOR_NAME", "")
        .env("GIT_AUTHOR_EMAIL", "")
        .env("HOME", "/dev/null") // Prevent git from finding global config
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Test commit")
        .assert()
        .failure()
        .stderr(predicate::str::contains("user.name is not set"));
}

#[test]
fn test_commit_with_amend() {
    let temp_dir = setup_git_repo();

    // Create and stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    // Initial commit
    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Initial commit")
        .assert()
        .success();

    // Modify and stage the file
    fs::write(&test_file, "updated content").expect("Failed to update test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage updated file");

    // Amend commit with non-interactive mode
    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Amended commit")
        .arg("--amend")
        .assert()
        .success()
        .stdout(predicate::str::contains("feat: Amended commit"));

    // Verify git log shows only one commit
    let git_log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_output = String::from_utf8_lossy(&git_log.stdout);
    assert_eq!(log_output.lines().count(), 1);
    assert!(log_output.contains("feat: Amended commit"));
}
