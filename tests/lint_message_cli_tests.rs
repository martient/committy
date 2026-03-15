mod common;

use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn write_commit_rules_config(dir: &std::path::Path, body: &str) {
    fs::create_dir_all(dir.join(".committy")).expect("Failed to create .committy");
    fs::write(
        dir.join(".committy/config.toml"),
        format!(
            r#"packages = []

[repository]
name = "lint-repo"
type = "single-package"

[versioning]
strategy = "independent"

[commit_rules]
{body}
"#
        ),
    )
    .expect("Failed to write config");
}

#[test]
fn lint_message_valid_text() {
    let mut cmd = common::committy_cmd();
    cmd.args([
        "--non-interactive",
        "lint-message",
        "--message",
        "feat: add nice thing",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("✅"));
}

#[test]
fn lint_message_invalid_text() {
    let mut cmd = common::committy_cmd();
    cmd.args([
        "--non-interactive",
        "lint-message",
        "--message",
        "invalid header",
    ]);
    // Exit code 3 for lint issues
    let assert = cmd.assert();
    #[cfg(unix)]
    assert.code(3).stdout(predicate::str::contains("❌"));
    #[cfg(windows)]
    assert.failure().stdout(predicate::str::contains("❌"));
}

#[test]
fn lint_message_valid_json() {
    let mut cmd = common::committy_cmd();
    cmd.args([
        "--non-interactive",
        "lint-message",
        "--message",
        "fix(core): correct minor bug",
        "--output",
        "json",
    ]);
    let output = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8(output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["command"], "lint-message");
    assert_eq!(v["ok"], true);
    assert_eq!(v["dry_run"], false);
    assert_eq!(v["count"], 0);
}

#[test]
fn lint_message_invalid_json() {
    let mut cmd = common::committy_cmd();
    cmd.args([
        "--non-interactive",
        "lint-message",
        "--message",
        "fix: a",
        "--output",
        "json",
    ]);
    let assert = cmd.assert();
    #[cfg(unix)]
    let output = assert.code(3).get_output().stdout.clone();
    #[cfg(windows)]
    let output = assert.failure().get_output().stdout.clone();

    let s = String::from_utf8(output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["command"], "lint-message");
    assert_eq!(v["ok"], false);
    assert!(v["count"].as_u64().unwrap() >= 1);
}

#[test]
fn lint_message_uses_repo_commit_rules() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_commit_rules_config(
        dir.path(),
        r#"allowed_types = ["feat"]

[[commit_rules.custom_types]]
name = "wip"
description = "Work in progress"
bump = "none""#,
    );

    let mut cmd = common::committy_cmd();
    cmd.current_dir(&dir).args([
        "--non-interactive",
        "lint-message",
        "--repo-path",
        ".",
        "--message",
        "wip: checkpoint",
        "--output",
        "json",
    ]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8(output).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["count"], 0);
}
