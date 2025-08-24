use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn lint_message_valid_text() {
    let mut cmd = Command::cargo_bin("committy").unwrap();
    cmd.args([
        "--non-interactive",
        "lint-message",
        "--message",
        "feat: add nice thing",
    ]);
    cmd.assert().success().stdout(predicate::str::contains("✅"));
}

#[test]
fn lint_message_invalid_text() {
    let mut cmd = Command::cargo_bin("committy").unwrap();
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
    let mut cmd = Command::cargo_bin("committy").unwrap();
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
    assert_eq!(v["ok"], true);
    assert_eq!(v["count"], 0);
}

#[test]
fn lint_message_invalid_json() {
    let mut cmd = Command::cargo_bin("committy").unwrap();
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
    assert_eq!(v["ok"], false);
    assert!(v["count"].as_u64().unwrap() >= 1);
}
