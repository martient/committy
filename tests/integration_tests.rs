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
        .args(["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to initialize git repository");

    // Create a test file
    let test_file = dir.path().join("test.txt");
    std::fs::write(&test_file, "test content").expect("Failed to write test file");

    // Add the file to git
    let _ = StdCommand::new("git")
        .args(["add", "test.txt"])
        .current_dir(&dir)
        .output()
        .expect("Failed to add test file to git");

    // Configure git user for commits
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

#[test]
fn test_tag_prerelease_on_feature_branch_json() {
    let temp_dir = setup();

    // Commit staged file so index is clean
    let _ = StdCommand::new("git")
        .args(["commit", "-m", "feat: initial"]) // commit test.txt
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create initial commit");

    // Create and switch to a non-release branch to trigger pre-release mode automatically
    let _ = StdCommand::new("git")
        .args(["checkout", "-b", "feature/test-prerelease"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create and switch branch");

    // Another commit to have content after the base tag
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: add feature work"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create feature commit");

    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["pre_release"], serde_json::json!(true));
    // default suffix is beta
    assert_eq!(v["new_tag"], serde_json::json!("v0.1.0-beta.0"));

    cleanup(temp_dir);
}

#[test]
fn test_tag_prerelease_custom_suffix_rc_on_release_branch_json() {
    let temp_dir = setup();

    // Commit staged file so index is clean (stay on default branch: master/main)
    let _ = StdCommand::new("git")
        .args(["commit", "-m", "feat: initial"]) // commit test.txt
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create initial commit");

    // Force pre-release on release branch with custom suffix
    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--prerelease")
        .arg("--prerelease-suffix")
        .arg("rc")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["pre_release"], serde_json::json!(true));
    assert_eq!(v["new_tag"], serde_json::json!("v0.1.0-rc.0"));

    cleanup(temp_dir);
}

#[test]
fn test_tag_prerelease_increments_when_same_base_version() {
    let temp_dir = setup();

    // Commit staged file so index is clean
    let _ = StdCommand::new("git")
        .args(["commit", "-m", "feat: initial"]) // commit test.txt
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create initial commit");

    // Switch to prerelease branch
    let _ = StdCommand::new("git")
        .args(["checkout", "-b", "feature/pre-seq"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create and switch branch");

    // First run: create actual pre-release tag locally (no push)
    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--not-publish")
        // not dry-run so tag is created locally
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    // Add a commit that signals no bump (contains #none) and does NOT match bump regexes
    let _ = StdCommand::new("git")
        .args([
            "commit",
            "--allow-empty",
            "-m",
            "noop: keep prerelease base #none",
        ])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create #none commit");

    // Second run: should increment beta.0 -> beta.1 on same base version
    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["pre_release"], serde_json::json!(true));
    assert_eq!(v["new_tag"], serde_json::json!("v0.1.0-beta.1"));

    cleanup(temp_dir);
}

#[test]
fn test_verbosity_quiet_suppresses_info_logs() {
    let temp_dir = setup();

    // ensure at least one commit exists
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    // With -q, only errors should be logged; dry run should produce none
    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("-q")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    cleanup(temp_dir);
}

#[test]
fn test_verbosity_v_shows_debug_logs() {
    let temp_dir = setup();

    // ensure at least one commit exists
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    // With -v and --no-fetch, expect debug about skipping fetch due to flag
    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("-v")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Skipping remote tag fetch (fetch flag not set)",
        ));

    cleanup(temp_dir);
}

#[test]
fn test_fetch_flag_no_fetch_skips_fetch_path() {
    let temp_dir = setup();

    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("-v")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Skipping remote tag fetch (fetch flag not set)",
        ));

    cleanup(temp_dir);
}

#[test]
fn test_fetch_flag_fetch_attempts_fetch_path() {
    let temp_dir = setup();

    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    // With --fetch, we should log that we're fetching tags; since repo has no origin,
    // subsequent message may indicate skipping due to not found, but the "Fetching tags" info should appear
    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("-v")
        .arg("tag")
        .arg("--fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .assert()
        .success()
        .stderr(predicate::str::contains("Fetching tags from remote"));

    cleanup(temp_dir);
}

fn cleanup(temp_dir: tempfile::TempDir) {
    // Clean up the test repository
    let _ = StdCommand::new("rm")
        .args(["-rf", ".git"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to clean up git repository");

    let _ = StdCommand::new("rm")
        .args(["test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to clean up test file");
}

#[test]
fn test_commit_command_with_valid_input() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--scope")
        .arg("auth")
        .arg("--message")
        .arg("Add authentication feature")
        .assert()
        .success();

    // Verify git log
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%B", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert!(log_message.contains("feat(auth): Add authentication feature"));

    cleanup(temp_dir);
}

#[test]
fn test_lint_json_exit_code_and_payload() {
    let temp_dir = setup();

    // Make an invalid commit message to trigger lint issue
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "invalid message"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("lint")
        .arg("--repo-path")
        .arg(".")
        .arg("--output")
        .arg("json")
        .assert()
        .code(3); // stable lint exit code

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(false));
    assert_eq!(v["count"], serde_json::json!(1));

    cleanup(temp_dir);
}

#[test]
fn test_tag_json_dry_run_output_non_interactive() {
    let temp_dir = setup();

    // Create an initial commit so repo isn't empty and nothing is staged
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["new_tag"], serde_json::json!("v0.1.0"));

    cleanup(temp_dir);
}

#[test]
fn test_tag_respects_config_regex_override_fix_as_minor() {
    let temp_dir = setup();

    // Create an initial fix commit
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "fix: bug"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    // Point COMMITTY_CONFIG_DIR to isolated dir and write config overriding minor_regex to match fix
    let cfg_dir = temp_dir.path().join(".config-override");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let cfg_path = cfg_dir.join("config.toml");
    let config_toml = r#"
minor_regex = '(?im)^fix(?:\s*\([^)]*\))?:'
patch_regex = '(?im)^docs(?:\s*\([^)]*\))?:'  # ensure 'fix' doesn't match patch
major_regex = '(?im)^(breaking change:|feat(?:\s*\([^)]*\))?!:)'
"#;
    std::fs::write(&cfg_path, config_toml).unwrap();

    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("COMMITTY_CONFIG_DIR", &cfg_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    // With override, fix should result in minor bump from 0.0.0 -> 0.1.0
    assert_eq!(v["new_tag"], serde_json::json!("v0.1.0"));

    cleanup(temp_dir);
}

#[test]
fn test_tag_fix_default_is_patch() {
    let temp_dir = setup();

    // Create an initial fix commit
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "fix: bug"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    // Default config should treat fix as patch -> v0.0.1
    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["new_tag"], serde_json::json!("v0.0.1"));

    cleanup(temp_dir);
}

#[test]
fn test_commit_command_with_auto_correction() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feature")
        .arg("--scope")
        .arg("user@service")
        .arg("--message")
        .arg("Add user service")
        .assert()
        .success();

    // Verify git log
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%B", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert!(log_message.contains("feat(user-service): Add user service"));

    cleanup(temp_dir);
}

#[test]
fn test_commit_command_with_invalid_input() {
    let temp_dir = setup();

    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.current_dir(&temp_dir)
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
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
        .env("RUST_LOG", "info")
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--scope")
        .arg("auth")
        .arg("--breaking-change")
        .arg("--message")
        .arg("Breaking change in authentication")
        .assert()
        .success();

    // Verify git log
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%B", "-n", "1"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to get git log");

    let log_message = String::from_utf8_lossy(&git_log.stdout);
    assert!(log_message.contains("feat(auth)!: Breaking change in authentication"));

    cleanup(temp_dir);
}

#[test]
fn test_lint_text_output_success() {
    let temp_dir = setup();

    // Create a valid commit to ensure no lint issues
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("lint")
        .arg("--repo-path")
        .arg(".")
        .arg("--output")
        .arg("text")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "✅ All commits since the last tag follow the conventional commit format!",
        ));

    cleanup(temp_dir);
}

#[test]
fn test_lint_text_output_failure() {
    let temp_dir = setup();

    // Create an invalid commit message to trigger lint issues
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "invalid message"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("lint")
        .arg("--repo-path")
        .arg(".")
        .arg("--output")
        .arg("text")
        .assert()
        .code(3)
        .stdout(predicate::str::contains(
            "❌ Found 1 commit(s) with issues:",
        ));

    cleanup(temp_dir);
}

#[test]
fn test_tag_text_output_non_interactive() {
    let temp_dir = setup();

    // Ensure at least one commit so bump detection works
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "feat: initial"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to create commit");

    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-fetch")
        .arg("--dry-run")
        .arg("--not-publish")
        .arg("--output")
        .arg("text")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^v0\.1\.0\s*$").unwrap());

    cleanup(temp_dir);
}

#[test]
fn test_exit_code_1_on_no_staged_changes() {
    let temp_dir = setup();

    // Unstage any staged files to trigger NoStagedChanges
    let _ = StdCommand::new("git")
        .args(["reset"]) // unstage all
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to reset index");

    Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        // No subcommand
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No staged changes found"));

    cleanup(temp_dir);
}
