mod common;

use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn write_root_cargo_package(dir: &std::path::Path, name: &str, version: &str) {
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "{version}"
edition = "2021"
"#
        ),
    )
    .expect("Failed to write Cargo.toml");
}

fn write_nested_cargo_package(dir: &std::path::Path, name: &str, version: &str) {
    fs::create_dir_all(dir).expect("Failed to create package directory");
    write_root_cargo_package(dir, name, version);
}

fn write_single_package_config(dir: &std::path::Path, repo_name: &str, package_name: &str) {
    fs::create_dir_all(dir.join(".committy")).expect("Failed to create .committy");
    fs::write(
        dir.join(".committy/config.toml"),
        format!(
            r#"[repository]
name = "{repo_name}"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "{package_name}"
type = "rust-cargo"
path = "."
version_file = "Cargo.toml"
version_field = "version"
"#
        ),
    )
    .expect("Failed to write .committy/config.toml");
}

fn write_sync_config(dir: &std::path::Path) {
    fs::create_dir_all(dir.join(".committy")).expect("Failed to create .committy");
    fs::write(
        dir.join(".committy/config.toml"),
        r#"[repository]
name = "workspace"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "pkg-a"
type = "rust-cargo"
path = "pkg-a"
version_file = "Cargo.toml"
version_field = "version"

[[packages]]
name = "pkg-b"
type = "rust-cargo"
path = "pkg-b"
version_file = "Cargo.toml"
version_field = "version"
sync_with = "pkg-a"
"#,
    )
    .expect("Failed to write sync config");
}

#[test]
fn test_init_dry_run_json_is_clean_stdout() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_root_cargo_package(dir.path(), "init-test", "0.1.0");

    let assert = common::committy_cmd()
        .current_dir(&dir)
        .arg("--non-interactive")
        .arg("init")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(payload["command"], Value::String("init".into()));
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(payload["dry_run"], Value::Bool(true));
    assert_eq!(payload["created"], Value::Bool(false));
    assert_eq!(
        payload["config"]["repository"],
        Value::String("my-repo".into())
    );
    assert!(
        !dir.path().join(".committy/config.toml").exists(),
        "dry-run must not create the config file"
    );
}

#[test]
fn test_config_validate_json_is_clean_stdout() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_root_cargo_package(dir.path(), "config-test", "0.1.0");
    write_single_package_config(dir.path(), "config-repo", "config-test");

    let assert = common::committy_cmd()
        .current_dir(&dir)
        .arg("--non-interactive")
        .arg("config")
        .arg("validate")
        .arg("--repo-path")
        .arg(".")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(payload["command"], Value::String("config".into()));
    assert_eq!(payload["mode"], Value::String("validate".into()));
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(payload["config_found"], Value::Bool(true));
    assert_eq!(
        payload["repository"]["name"],
        Value::String("config-repo".into())
    );
}

#[test]
fn test_config_show_json_is_clean_stdout() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_root_cargo_package(dir.path(), "config-show", "0.1.0");
    write_single_package_config(dir.path(), "show-repo", "config-show");

    let assert = common::committy_cmd()
        .current_dir(&dir)
        .arg("--non-interactive")
        .arg("config")
        .arg("show")
        .arg("--repo-path")
        .arg(".")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(payload["command"], Value::String("config".into()));
    assert_eq!(payload["mode"], Value::String("show".into()));
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(
        payload["repository_config"]["repository"]["name"],
        Value::String("show-repo".into())
    );
    assert!(payload["user_config"].is_object());
}

#[test]
fn test_packages_list_json_is_clean_stdout() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_root_cargo_package(dir.path(), "packages-list", "0.1.0");

    let assert = common::committy_cmd()
        .current_dir(&dir)
        .arg("--non-interactive")
        .arg("packages")
        .arg("list")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(payload["command"], Value::String("packages".into()));
    assert_eq!(payload["mode"], Value::String("list".into()));
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(payload["total_packages"], Value::Number(1.into()));
    assert_eq!(
        payload["packages"][0]["name"],
        Value::String("packages-list".into())
    );
}

#[test]
fn test_packages_status_check_json_failure_is_clean_stdout() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_nested_cargo_package(&dir.path().join("pkg-a"), "pkg-a", "1.0.0");
    write_nested_cargo_package(&dir.path().join("pkg-b"), "pkg-b", "0.9.0");
    write_sync_config(dir.path());

    let assert = common::committy_cmd()
        .current_dir(&dir)
        .arg("--non-interactive")
        .arg("packages")
        .arg("status")
        .arg("--check")
        .arg("--output")
        .arg("json")
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(payload["command"], Value::String("packages".into()));
    assert_eq!(payload["mode"], Value::String("status".into()));
    assert_eq!(payload["ok"], Value::Bool(false));
    assert_eq!(payload["check"], Value::Bool(true));
    assert_eq!(payload["issues"].as_array().unwrap().len(), 1);
}

#[test]
fn test_packages_sync_dry_run_json_is_clean_stdout() {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp dir");
    write_nested_cargo_package(&dir.path().join("pkg-a"), "pkg-a", "1.0.0");
    write_nested_cargo_package(&dir.path().join("pkg-b"), "pkg-b", "0.9.0");
    write_sync_config(dir.path());

    let assert = common::committy_cmd()
        .current_dir(&dir)
        .arg("--non-interactive")
        .arg("packages")
        .arg("sync")
        .arg("--dry-run")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let payload: Value = serde_json::from_str(stdout.trim()).unwrap();

    assert_eq!(payload["command"], Value::String("packages".into()));
    assert_eq!(payload["mode"], Value::String("sync".into()));
    assert_eq!(payload["ok"], Value::Bool(true));
    assert_eq!(payload["dry_run"], Value::Bool(true));
    assert_eq!(payload["operations"].as_array().unwrap().len(), 1);
    assert_eq!(
        payload["operations"][0]["name"],
        Value::String("pkg-b".into())
    );
}
