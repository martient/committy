mod common;

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    common::setup_test_env();
    let dir = tempdir().unwrap();
    StdCommand::new("git")
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();
    dir
}

#[test]
fn hooks_install_dry_run_lists_files_without_writing() {
    let repo = setup_repo();
    let assert = common::committy_cmd()
        .current_dir(&repo)
        .args([
            "--non-interactive",
            "hooks",
            "install",
            "--with-ci",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let payload: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(payload["api_version"], 1);
    assert_eq!(payload["command"], "hooks");
    assert_eq!(payload["dry_run"], true);
    assert_eq!(payload["installed"], false);
    assert_eq!(payload["files"].as_array().unwrap().len(), 3);
    assert!(!repo.path().join(".git/hooks/commit-msg").exists());
    assert!(!repo.path().join(".github/workflows/committy.yml").exists());
}

#[test]
fn hooks_install_writes_executable_hooks_and_ci_workflow() {
    let repo = setup_repo();
    common::committy_cmd()
        .current_dir(&repo)
        .args([
            "--non-interactive",
            "hooks",
            "install",
            "--with-ci",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let commit_msg = repo.path().join(".git/hooks/commit-msg");
    let pre_push = repo.path().join(".git/hooks/pre-push");
    let workflow = repo.path().join(".github/workflows/committy.yml");
    assert!(fs::read_to_string(&commit_msg)
        .unwrap()
        .contains("hooks run commit-msg"));
    assert!(fs::read_to_string(&pre_push)
        .unwrap()
        .contains("hooks run pre-push"));
    assert!(fs::read_to_string(workflow)
        .unwrap()
        .contains("committy --non-interactive lint"));
    assert_ne!(
        fs::metadata(commit_msg).unwrap().permissions().mode() & 0o111,
        0
    );
}

#[test]
fn hooks_run_commit_msg_preserves_lint_exit_code() {
    let repo = setup_repo();
    let message = repo.path().join("COMMIT_EDITMSG");
    fs::write(&message, "not conventional\n").unwrap();

    common::committy_cmd()
        .current_dir(&repo)
        .arg("--non-interactive")
        .arg("hooks")
        .arg("run")
        .arg("commit-msg")
        .arg("--message-file")
        .arg(&message)
        .arg("--output")
        .arg("json")
        .assert()
        .code(3);
}

#[test]
fn hooks_run_pre_push_lints_outgoing_commits() {
    let repo = setup_repo();
    for (key, value) in [
        ("user.name", "Test User"),
        ("user.email", "test@example.com"),
    ] {
        StdCommand::new("git")
            .args(["config", key, value])
            .current_dir(&repo)
            .output()
            .unwrap();
    }
    fs::write(repo.path().join("tracked.txt"), "content\n").unwrap();
    StdCommand::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(&repo)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "not conventional", "--no-verify"])
        .current_dir(&repo)
        .output()
        .unwrap();
    let sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let input = format!(
        "refs/heads/test {} refs/heads/test {}\n",
        sha.trim(),
        "0".repeat(40)
    );

    let assert = common::committy_cmd()
        .current_dir(&repo)
        .args([
            "--non-interactive",
            "hooks",
            "run",
            "pre-push",
            "--output",
            "json",
        ])
        .write_stdin(input)
        .assert()
        .code(3);
    let payload: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(payload["hook"], "pre-push");
    assert!(payload["count"].as_u64().unwrap() > 0);
}
