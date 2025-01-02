mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup() -> tempfile::TempDir {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp directory");

    // Initialize a git repository for testing
    let _ = StdCommand::new("git")
        .args(&["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to initialize git repository");

    // Create a test file
    let test_file = dir.path().join("test.txt");
    std::fs::write(&test_file, "test content").expect("Failed to write test file");

    // Add the file to git
    let _ = StdCommand::new("git")
        .args(&["add", "test.txt"])
        .current_dir(&dir)
        .output()
        .expect("Failed to add test file to git");

    // Configure git user for commits
    let _ = StdCommand::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure git user name");

    let _ = StdCommand::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure git user email");

    dir
}

fn cleanup(temp_dir: tempfile::TempDir) {
    // Clean up the test repository
    let _ = StdCommand::new("rm")
        .args(&["-rf", ".git"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to clean up git repository");

    let _ = StdCommand::new("rm")
        .args(&["test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to clean up test file");
}

#[test]
fn test_commit_command_with_valid_input() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--scope")
        .arg("auth")
        .arg("--message")
        .arg("Add authentication feature")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "feat(auth): Add authentication feature",
        ));

    cleanup(temp_dir);
}

#[test]
fn test_commit_command_with_auto_correction() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feature")
        .arg("--scope")
        .arg("user@service")
        .arg("--message")
        .arg("Add user service")
        .assert()
        .success()
        .stdout(predicate::str::contains("Auto-correcting commit type"))
        .stdout(predicate::str::contains("Auto-correcting scope"))
        .stdout(predicate::str::contains(
            "feat(userservice): Add user service",
        ));

    cleanup(temp_dir);
}

#[test]
fn test_commit_command_with_invalid_input() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("invalid")
        .arg("--message")
        .arg("Invalid commit")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid commit type"));

    cleanup(temp_dir);
}

#[test]
fn test_commit_with_breaking_change() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("commit")
        .arg("--non-interactive")
        .arg("--type")
        .arg("feat")
        .arg("--scope")
        .arg("auth")
        .arg("--breaking-change")
        .arg("--message")
        .arg("Breaking change in authentication")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "feat(auth)!: Breaking change in authentication",
        ));

    cleanup(temp_dir);
}
