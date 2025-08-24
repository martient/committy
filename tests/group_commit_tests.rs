mod common;

use assert_cmd::Command;
use serde_json::Value;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp directory");

    // Initialize a git repository for testing
    let _ = StdCommand::new("git")
        .args(["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to initialize git repository");

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

    // Ensure a HEAD exists for operations that reference HEAD
    let _ = StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "chore: init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to create initial commit");

    dir
}

#[test]
fn test_group_commit_apply_without_auto_stage_only_staged_committed() {
    let temp_dir = setup_repo();

    // Create one staged (docs) and one unstaged (code)
    let docs_file = temp_dir.path().join("docs/CHANGELOG.md");
    std::fs::create_dir_all(docs_file.parent().unwrap()).unwrap();
    std::fs::write(&docs_file, "# Changelog\n").unwrap();

    // Stage docs file
    let _ = StdCommand::new("git")
        .args(["add", "docs/CHANGELOG.md"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage docs file");

    // Create an unstaged code file
    let code_file = temp_dir.path().join("src/only_unstaged.rs");
    std::fs::create_dir_all(code_file.parent().unwrap()).unwrap();
    std::fs::write(&code_file, "pub fn x() {}\n").unwrap();

    // Apply without include-unstaged and without auto-stage
    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("group-commit")
        .arg("--mode")
        .arg("apply")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], Value::Bool(true));
    let commits = v["commits"].as_array().expect("commits array");
    // Only docs group should be committed
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["group"], Value::String("docs".into()));

    // Ensure the unstaged code file is still uncommitted (exists and not tracked)
    assert!(temp_dir.path().join("src/only_unstaged.rs").exists());
    let ls = StdCommand::new("git")
        .args(["ls-files", "--error-unmatch", "src/only_unstaged.rs"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run git ls-files");
    assert!(!ls.status.success(), "unstaged file should not be tracked");
}

#[test]
fn test_group_commit_apply_with_push_sets_pushed_false_without_remote() {
    let temp_dir = setup_repo();

    // Create a simple staged change
    let docs_file = temp_dir.path().join("docs/PUSH.md");
    std::fs::create_dir_all(docs_file.parent().unwrap()).unwrap();
    std::fs::write(&docs_file, "Push test\n").unwrap();
    let _ = StdCommand::new("git")
        .args(["add", "docs/PUSH.md"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage PUSH.md");

    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("group-commit")
        .arg("--mode")
        .arg("apply")
        .arg("--push")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["mode"], Value::String("apply".into()));
    // No remote -> push should fail and be false
    assert_eq!(v["pushed"], Value::Bool(false));
}

#[test]
fn test_group_commit_plan_json_offline() {
    let temp_dir = setup_repo();

    // Create staged changes across two groups: Docs and Code
    let docs_file = temp_dir.path().join("docs/README.md");
    std::fs::create_dir_all(docs_file.parent().unwrap()).unwrap();
    std::fs::write(&docs_file, "# Docs\n").unwrap();

    let code_file = temp_dir.path().join("src/main.rs");
    std::fs::create_dir_all(code_file.parent().unwrap()).unwrap();
    std::fs::write(&code_file, "fn main() {}\n").unwrap();

    // Stage files
    let _ = StdCommand::new("git")
        .args(["add", "--all"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage files");

    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("group-commit")
        .arg("--mode")
        .arg("plan")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();

    assert_eq!(v["command"], Value::String("group-commit".into()));
    assert_eq!(v["mode"], Value::String("plan".into()));
    assert_eq!(v["ok"], Value::Bool(true));

    let groups = v["groups"].as_array().expect("groups array");
    assert!(groups.len() >= 2, "expected at least 2 groups, got {}", groups.len());

    // Ensure Docs and Code groups present
    let mut seen_docs = false;
    let mut seen_code = false;
    for g in groups {
        if g["name"] == Value::String("docs".into()) {
            seen_docs = true;
            // Check suggested message has expected type prefix
            let msg = g["suggested_message"].as_str().unwrap_or("");
            assert!(msg.starts_with("docs:"), "docs message should start with 'docs:', got: {}", msg);
        }
        if g["name"] == Value::String("code".into()) {
            seen_code = true;
            let msg = g["suggested_message"].as_str().unwrap_or("");
            assert!(msg.starts_with("chore:"), "code default type is 'chore:', got: {}", msg);
        }
    }
    assert!(seen_docs && seen_code, "Docs and Code groups should be present");
}

#[test]
fn test_group_commit_apply_auto_stage_creates_commits() {
    let temp_dir = setup_repo();

    // Create unstaged changes across two groups: Docs and Code
    let docs_file = temp_dir.path().join("docs/README.md");
    std::fs::create_dir_all(docs_file.parent().unwrap()).unwrap();
    std::fs::write(&docs_file, "# Docs\n").unwrap();

    let code_file = temp_dir.path().join("src/lib.rs");
    std::fs::create_dir_all(code_file.parent().unwrap()).unwrap();
    std::fs::write(&code_file, "pub fn hello() {}\n").unwrap();

    // Run apply with auto-stage
    let assert = Command::cargo_bin("committy")
        .unwrap()
        .current_dir(&temp_dir)
        .env("RUST_LOG", "off")
        .arg("--non-interactive")
        .arg("group-commit")
        .arg("--mode")
        .arg("apply")
        .arg("--include-unstaged")
        .arg("--auto-stage")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();

    assert_eq!(v["mode"], Value::String("apply".into()));
    assert_eq!(v["ok"], Value::Bool(true));

    let commits = v["commits"].as_array().expect("commits array");
    assert!(commits.len() >= 2, "expected at least 2 commits, got {}", commits.len());
    for c in commits {
        assert_eq!(c["ok"], Value::Bool(true));
        assert!(c["sha"].as_str().is_some(), "sha should be present");
        assert!(c["message"].as_str().unwrap_or("").len() > 0);
    }

    // Verify git log has at least 2 new commits after init
    let log = StdCommand::new("git")
        .args(["log", "--format=%s", "-n", "3"]) // init + two group commits
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to read git log");
    let log_s = String::from_utf8_lossy(&log.stdout);
    assert!(log_s.contains("update docs") || log_s.contains("docs:"));
    assert!(log_s.contains("misc maintenance") || log_s.contains("chore:"));
}
