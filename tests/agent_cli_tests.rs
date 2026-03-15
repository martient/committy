mod common;

use serde_json::Value;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp directory");

    StdCommand::new("git")
        .args(["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to initialize git repository");

    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure git user name");

    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure git user email");

    let file = dir.path().join("tracked.txt");
    fs::write(&file, "initial\n").expect("Failed to write tracked file");
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&dir)
        .output()
        .expect("Failed to stage tracked file");
    StdCommand::new("git")
        .args(["commit", "-m", "feat: initial"])
        .current_dir(&dir)
        .output()
        .expect("Failed to create initial commit");

    dir
}

fn write_commit_rules_config(dir: &std::path::Path, body: &str) {
    fs::create_dir_all(dir.join(".committy")).expect("Failed to create .committy");
    fs::write(
        dir.join(".committy/config.toml"),
        format!(
            r#"packages = []

[repository]
name = "agent-repo"
type = "single-package"

[versioning]
strategy = "independent"

[scopes]
auto_detect = false
require_scope_for_multi_package = false
allow_multiple_scopes = false

[commit_rules]
{body}
"#
        ),
    )
    .expect("Failed to write commit rules config");
}

#[test]
fn test_branch_dry_run_json_outputs_plan() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("branch")
        .arg("--name")
        .arg("feat-agent-plan")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();

    assert_eq!(v["command"], Value::String("branch".into()));
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(v["dry_run"], Value::Bool(true));
    assert_eq!(v["branch_name"], Value::String("feat-agent-plan".into()));
    assert_eq!(v["branch_type"], Value::String("feat".into()));
    assert_eq!(v["ticket"], Value::String(String::new()));
    assert_eq!(v["subject"], Value::String("agent-plan".into()));
    assert_eq!(v["would_create"], Value::Bool(true));
    assert_eq!(v["would_checkout"], Value::Bool(false));

    let branches = StdCommand::new("git")
        .args(["branch", "--list", "feat-agent-plan"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to list branches");
    assert!(
        String::from_utf8_lossy(&branches.stdout).trim().is_empty(),
        "dry-run must not create the branch"
    );
}

#[test]
fn test_branch_structured_dry_run_json_outputs_plan() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("branch")
        .arg("--type")
        .arg("feat")
        .arg("--ticket")
        .arg("AI42")
        .arg("--subject")
        .arg("agent flow")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();

    assert_eq!(v["command"], Value::String("branch".into()));
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(v["dry_run"], Value::Bool(true));
    assert_eq!(
        v["branch_name"],
        Value::String("feat-AI42-agent_flow".into())
    );
    assert_eq!(v["branch_type"], Value::String("feat".into()));
    assert_eq!(v["ticket"], Value::String("AI42".into()));
    assert_eq!(v["subject"], Value::String("agent_flow".into()));
    assert_eq!(v["would_checkout"], Value::Bool(false));
}

#[test]
fn test_branch_structured_flags_conflict_with_name() {
    let temp_dir = setup_repo();

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("branch")
        .arg("--name")
        .arg("feat-agent-plan")
        .arg("--type")
        .arg("feat")
        .arg("--subject")
        .arg("agent")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Use either --name or structured branch flags",
        ));
}

#[test]
fn test_branch_dry_run_json_supports_repo_path_without_chdir() {
    let temp_dir = setup_repo();
    let runner_dir = tempdir().expect("Failed to create runner directory");

    let assert = common::committy_cmd()
        .current_dir(&runner_dir)
        .arg("--non-interactive")
        .arg("branch")
        .arg("--repo-path")
        .arg(temp_dir.path())
        .arg("--name")
        .arg("feat-agent-plan")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(v["branch_name"], Value::String("feat-agent-plan".into()));
}

#[test]
fn test_commit_dry_run_json_does_not_create_commit() {
    let temp_dir = setup_repo();
    let file = temp_dir.path().join("tracked.txt");
    fs::write(&file, "changed\n").expect("Failed to update tracked file");
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage tracked file");

    let before = StdCommand::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to count commits");

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--scope")
        .arg("agent")
        .arg("--message")
        .arg("preview agent commit")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["command"], Value::String("commit".into()));
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(v["dry_run"], Value::Bool(true));
    assert_eq!(
        v["message"],
        Value::String("feat(agent): preview agent commit".into())
    );

    let after = StdCommand::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to count commits");

    assert_eq!(
        before.stdout, after.stdout,
        "dry-run must not create commits"
    );
}

#[test]
fn test_commit_dry_run_json_supports_repo_path_without_chdir() {
    let temp_dir = setup_repo();
    let runner_dir = tempdir().expect("Failed to create runner directory");
    let file = temp_dir.path().join("tracked.txt");
    fs::write(&file, "changed\n").expect("Failed to update tracked file");
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage tracked file");

    let assert = common::committy_cmd()
        .current_dir(&runner_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--repo-path")
        .arg(temp_dir.path())
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("preview agent commit")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["command"], Value::String("commit".into()));
    assert_eq!(v["ok"], Value::Bool(true));
}

#[test]
fn test_commit_dry_run_json_respects_require_body_rule() {
    let temp_dir = setup_repo();
    write_commit_rules_config(temp_dir.path(), "require_body = true");

    let file = temp_dir.path().join("tracked.txt");
    fs::write(&file, "changed\n").expect("Failed to update tracked file");
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage tracked file");

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("feat")
        .arg("--message")
        .arg("preview without body")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .code(3);

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["command"], Value::String("commit".into()));
    assert_eq!(v["ok"], Value::Bool(false));
    assert_eq!(v["dry_run"], Value::Bool(true));
    assert_eq!(
        v["errors"][0],
        Value::String("Commit body is required by repository configuration".into())
    );
}

#[test]
fn test_commit_dry_run_json_accepts_custom_commit_type_from_repo_rules() {
    let temp_dir = setup_repo();
    write_commit_rules_config(
        temp_dir.path(),
        r#"allowed_types = ["feat"]

[[commit_rules.custom_types]]
name = "wip"
description = "Work in progress"
bump = "none""#,
    );

    let file = temp_dir.path().join("tracked.txt");
    fs::write(&file, "changed again\n").expect("Failed to update tracked file");
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to stage tracked file");

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("commit")
        .arg("--type")
        .arg("wip")
        .arg("--message")
        .arg("preview custom type")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["ok"], Value::Bool(true));
    assert_eq!(v["commit_type"], Value::String("wip".into()));
    assert_eq!(
        v["message"],
        Value::String("wip: preview custom type".into())
    );
}

#[test]
fn test_tag_publish_requires_confirmation() {
    let temp_dir = setup_repo();

    common::committy_cmd()
        .current_dir(&temp_dir)
        .arg("--non-interactive")
        .arg("tag")
        .arg("--name")
        .arg("v1.2.3")
        .arg("--publish")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Publishing a tag requires --confirm-publish",
        ));

    let tags = StdCommand::new("git")
        .args(["tag", "--list", "v1.2.3"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to list tags");
    assert!(
        String::from_utf8_lossy(&tags.stdout).trim().is_empty(),
        "tag should not be created when publish confirmation is missing"
    );
}

#[test]
fn test_tag_dry_run_json_supports_repo_path_without_chdir() {
    let temp_dir = setup_repo();
    let runner_dir = tempdir().expect("Failed to create runner directory");

    let assert = common::committy_cmd()
        .current_dir(&runner_dir)
        .arg("--non-interactive")
        .arg("tag")
        .arg("--repo-path")
        .arg(temp_dir.path())
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(v["command"], Value::String("tag".into()));
    assert_eq!(v["ok"], Value::Bool(true));
}
