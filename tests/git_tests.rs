mod common;

use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
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

fn write_executable_script(path: &std::path::Path, body: &str) {
    fs::write(path, body).expect("Failed to write script");
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("Failed to chmod script");
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
    let mut cmd = common::committy_cmd();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
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
        .success();

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
fn test_commit_accepts_message_file_in_non_interactive_mode() {
    let temp_dir = setup_git_repo();

    let test_file = temp_dir.path().join("test.txt");
    let message_file = temp_dir.path().join("commit-message.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    fs::write(
        &message_file,
        "feat(test)!: Test commit from file\n\nDetailed body",
    )
    .expect("Failed to write message file");
    StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    common::committy_cmd()
        .current_dir(&temp_dir)
        .env("COMMITTY_NONINTERACTIVE", "1")
        .env("CI", "1")
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("commit")
        .arg("--message-file")
        .arg(&message_file)
        .assert()
        .success();

    let git_log = StdCommand::new("git")
        .args(["log", "--format=%B", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert!(log_message.contains("feat(test)!: Test commit from file"));
    assert!(log_message.contains("Detailed body"));
}

#[test]
fn test_unstaged_changes() {
    let temp_dir = setup_git_repo();

    // Create but don't stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");

    let mut cmd = common::committy_cmd();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Test commit")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "No staged changes found\nFor help, run 'committy --help'",
        ));
}

#[test]
fn test_commit_without_git_config() {
    let temp_dir = setup_git_repo_without_config();
    let home_dir = tempdir().expect("Failed to create temp home directory");

    // Create and stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    let mut cmd = common::committy_cmd();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .env("GIT_COMMITTER_NAME", "")
        .env("GIT_COMMITTER_EMAIL", "")
        .env("GIT_AUTHOR_NAME", "")
        .env("GIT_AUTHOR_EMAIL", "")
        .env("HOME", home_dir.path()) // Use temp directory as HOME
        .arg("--non-interactive")
        .arg("commit")
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
    let mut cmd = common::committy_cmd();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
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
    let mut cmd = common::committy_cmd();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Amended commit")
        .arg("--amend")
        .assert()
        .success();

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

#[test]
fn test_amend_non_interactive_without_staged_changes() {
    let temp_dir = setup_git_repo();

    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Initial commit")
        .assert()
        .success();

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("amend")
        .arg("--type")
        .arg("fix")
        .arg("--message")
        .arg("Amended without staged changes")
        .assert()
        .success();

    let git_log = StdCommand::new("git")
        .args(["log", "--format=%s", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert_eq!(log_message.lines().count(), 1);
    assert!(log_message.contains("fix: Amended without staged changes"));
}

#[test]
fn test_commit_amend_without_staged_changes_matches_amend_command() {
    let temp_dir = setup_git_repo();

    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Initial commit")
        .assert()
        .success();

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--amend")
        .arg("--type")
        .arg("fix")
        .arg("--message")
        .arg("Amended via commit flag")
        .assert()
        .success();

    let git_log = StdCommand::new("git")
        .args(["log", "--format=%s", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert_eq!(log_message.lines().count(), 1);
    assert!(log_message.contains("fix: Amended via commit flag"));
}

#[test]
fn test_amend_dry_run_json_does_not_rewrite_commit() {
    let temp_dir = setup_git_repo();

    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Initial commit")
        .assert()
        .success();

    let before = StdCommand::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get HEAD before amend");

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("amend")
        .arg("--type")
        .arg("fix")
        .arg("--message")
        .arg("Preview amend")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["command"], serde_json::json!("amend"));
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["dry_run"], serde_json::json!(true));
    assert_eq!(v["message"], serde_json::json!("fix: Preview amend"));

    let after = StdCommand::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get HEAD after amend preview");

    assert_eq!(before.stdout, after.stdout);
}

#[test]
fn test_amend_respects_repo_commit_rules() {
    let temp_dir = setup_git_repo();

    fs::create_dir_all(temp_dir.path().join(".committy")).expect("Failed to create .committy");
    fs::write(
        temp_dir.path().join(".committy/config.toml"),
        r#"packages = []

[repository]
name = "amend-repo"
type = "single-package"

[versioning]
strategy = "independent"

[scopes]
auto_detect = false
require_scope_for_multi_package = false
allow_multiple_scopes = false

[commit_rules]
require_body = true
"#,
    )
    .expect("Failed to write config");

    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage test file");

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Initial commit")
        .arg("--long-message")
        .arg("Initial body")
        .assert()
        .success();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("amend")
        .arg("--type")
        .arg("fix")
        .arg("--message")
        .arg("Missing body")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .code(3);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(false));
    assert_eq!(
        v["errors"][0],
        serde_json::json!("Commit body is required by repository configuration")
    );
}

#[test]
fn test_commit_git_config_cli_override_changes_native_git_behavior() {
    let temp_dir = setup_git_repo();

    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");
    StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage initial file");

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("Initial commit")
        .assert()
        .success();

    fs::write(&test_file, "updated content").expect("Failed to update test file");
    StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage updated file");

    let hooks_dir = temp_dir.path().join("failing-hooks");
    fs::create_dir_all(&hooks_dir).expect("Failed to create hooks dir");
    write_executable_script(
        &hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'blocked by test hook' >&2\nexit 1\n",
    );

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("fix")
        .arg("--message")
        .arg("Blocked commit")
        .arg("--git-config")
        .arg(format!("core.hooksPath={}", hooks_dir.display()))
        .assert()
        .failure()
        .stderr(predicate::str::contains("blocked by test hook"));
}
