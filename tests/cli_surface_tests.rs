mod common;

use serde_json::Value;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    common::setup_test_env();
    let dir = tempdir().expect("Failed to create temp dir");
    StdCommand::new("git")
        .args(["init"])
        .current_dir(&dir)
        .output()
        .expect("Failed to init git repo");
    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure user.name");
    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&dir)
        .output()
        .expect("Failed to configure user.email");
    dir
}

fn commit_all(dir: &std::path::Path, message: &str) {
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .expect("Failed to stage files");
    StdCommand::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .expect("Failed to create commit");
}

fn tag_head(dir: &std::path::Path, tag: &str) {
    StdCommand::new("git")
        .args(["tag", "-a", tag, "-m", tag])
        .current_dir(dir)
        .output()
        .expect("Failed to tag head");
}

fn write_cargo(dir: &std::path::Path, version: &str) {
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "parity-test"
version = "{version}"
edition = "2021"
"#
        ),
    )
    .expect("Failed to write Cargo.toml");
}

fn write_package_json(dir: &std::path::Path, version: &str) {
    fs::write(
        dir.join("package.json"),
        format!(
            r#"{{
  "name": "parity-test",
  "version": "{version}"
}}
"#
        ),
    )
    .expect("Failed to write package.json");
}

fn write_composer(dir: &std::path::Path, version: &str) {
    fs::write(
        dir.join("composer.json"),
        format!(
            r#"{{
  "name": "acme/parity-test",
  "version": "{version}"
}}
"#
        ),
    )
    .expect("Failed to write composer.json");
}

fn write_pyproject_project(dir: &std::path::Path, version: &str, extra: &str) {
    fs::write(
        dir.join("pyproject.toml"),
        format!(
            r#"[project]
name = "parity-test"
version = "{version}"

{extra}
"#
        ),
    )
    .expect("Failed to write pyproject.toml");
}

fn write_pyproject_poetry(dir: &std::path::Path, version: &str) {
    fs::write(
        dir.join("pyproject.toml"),
        format!(
            r#"[tool.poetry]
name = "parity-test"
version = "{version}"
description = "test"
authors = ["Test <test@example.com>"]
"#
        ),
    )
    .expect("Failed to write pyproject.toml");
}

fn parse_stdout(assert: assert_cmd::assert::Assert) -> Value {
    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(output.trim()).unwrap()
}

fn write_multi_package_config(dir: &std::path::Path, package_versions: &[(&str, &str)]) {
    fs::create_dir_all(dir.join(".committy")).unwrap();
    let mut config = String::from(
        r#"[repository]
name = "workspace"
type = "multi-package"

[versioning]
strategy = "independent"

"#,
    );

    for (name, _version) in package_versions {
        config.push_str(&format!(
            r#"[[packages]]
name = "{name}"
type = "rust-cargo"
path = "{name}"
version_file = "Cargo.toml"
version_field = "package.version"

"#
        ));
    }

    fs::write(dir.join(".committy/config.toml"), config).unwrap();

    for (name, version) in package_versions {
        fs::create_dir_all(dir.join(name)).unwrap();
        fs::write(
            dir.join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2021\"\n"),
        )
        .unwrap();
    }
}

#[test]
fn test_config_scaffold_dry_run_json_exposes_new_sections() {
    let dir = setup_repo();

    let payload = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("config")
            .arg("scaffold")
            .arg("--dry-run")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );

    assert_eq!(payload["command"], Value::String("config".into()));
    assert_eq!(payload["mode"], Value::String("scaffold".into()));
    assert!(payload["config"]["convention"].is_object());
    assert!(payload["config"]["release"].is_object());
    assert!(payload["config"]["changelog"].is_object());
}

#[test]
fn test_schema_example_info_ls_and_version_json() {
    let dir = setup_repo();
    write_cargo(dir.path(), "0.1.0");
    fs::write(dir.path().join("src.rs"), "fn main() {}\n").unwrap();
    commit_all(dir.path(), "feat(cli): initial");
    tag_head(dir.path(), "v0.1.0");

    let schema = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("schema")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(schema["command"], Value::String("schema".into()));
    assert_eq!(schema["api_version"], Value::from(1));
    assert_eq!(
        schema["convention"],
        Value::String("conventional-commits".into())
    );
    assert!(schema["types"]
        .as_array()
        .unwrap()
        .contains(&Value::from("feat")));
    let feat = schema["type_definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "feat")
        .unwrap();
    assert_eq!(feat["description"], "A new feature");
    assert!(feat["contexts"]
        .as_array()
        .unwrap()
        .contains(&Value::from("branch")));
    assert!(schema["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["name"] == "branch.preview"
            && item["description"].as_str().unwrap().contains("without")));

    let example = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("example")
            .arg("--count")
            .arg("2")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(example["examples"].as_array().unwrap().len(), 2);

    let info = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("info")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(info["provider"], Value::String("cargo".into()));

    let ls = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("ls")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert!(ls["providers"].as_array().unwrap().len() >= 7);

    let version = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("version")
            .arg("--project")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(
        version["project"]["provider"],
        Value::String("cargo".into())
    );
    assert_eq!(version["project"]["version"], Value::String("0.1.0".into()));
}

#[test]
fn test_lint_allow_abort_and_rev_range_json() {
    let dir = setup_repo();
    write_cargo(dir.path(), "0.1.0");
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(cli): initial");
    tag_head(dir.path(), "v0.1.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "fix(cli): repair workflow");

    let message_file = dir.path().join("COMMIT_EDITMSG");
    fs::write(&message_file, "# Please enter the commit message\n").unwrap();

    let allow_abort = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("lint")
            .arg("--file")
            .arg(&message_file)
            .arg("--allow-abort")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(allow_abort["ok"], Value::Bool(true));

    let rev_range = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("lint")
            .arg("--rev-range")
            .arg("v0.1.0..HEAD")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(rev_range["count"], Value::Number(0.into()));
}

fn assert_bump_provider(write_manifest: impl Fn(&std::path::Path, &str), expected_provider: &str) {
    let dir = setup_repo();
    write_manifest(dir.path(), "0.1.0");
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(cli): initial");
    tag_head(dir.path(), "v0.1.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "feat(cli): add release parity");

    let payload = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("bump")
            .arg("--dry-run")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );

    assert_eq!(
        payload["plan"]["provider"],
        Value::String(expected_provider.to_string())
    );
    assert_eq!(
        payload["plan"]["next_version"],
        Value::String("0.2.0".into())
    );
    assert!(!payload["plan"]["tag_names"].as_array().unwrap().is_empty());
}

#[test]
fn test_bump_dry_run_for_builtin_providers() {
    assert_bump_provider(write_cargo, "cargo");
    assert_bump_provider(write_package_json, "npm");
    assert_bump_provider(write_composer, "composer");
    assert_bump_provider(
        |dir, version| write_pyproject_project(dir, version, ""),
        "pep621",
    );
    assert_bump_provider(write_pyproject_poetry, "poetry");
    assert_bump_provider(
        |dir, version| write_pyproject_project(dir, version, "[tool.uv]\nmanaged = true"),
        "uv",
    );
}

#[test]
fn test_bump_dry_run_for_scm_provider() {
    let dir = setup_repo();
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(cli): initial");
    tag_head(dir.path(), "v0.1.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "fix(cli): patch release");

    let payload = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("bump")
            .arg("--dry-run")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );

    assert_eq!(payload["plan"]["provider"], Value::String("scm".into()));
    assert_eq!(
        payload["plan"]["next_version"],
        Value::String("0.1.1".into())
    );
}

#[test]
fn test_bump_dry_run_uses_semver_latest_tag_and_preserves_breaking_bump() {
    let dir = setup_repo();
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat: initial");
    tag_head(dir.path(), "v2.0.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "feat: second");
    tag_head(dir.path(), "v10.0.0");
    fs::write(dir.path().join("tracked.txt"), "three\n").unwrap();
    commit_all(dir.path(), "feat!: breaking api");

    let payload = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("bump")
            .arg("--dry-run")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );

    assert_eq!(
        payload["plan"]["current_version"],
        Value::String("10.0.0".into())
    );
    assert_eq!(
        payload["plan"]["next_version"],
        Value::String("11.0.0".into())
    );
    assert_eq!(payload["plan"]["bump"], Value::String("major".into()));
    assert_eq!(
        payload["plan"]["changelog"]["previous_ref"],
        Value::String("v10.0.0".into())
    );
    assert_eq!(
        payload["plan"]["changelog"]["entry_count"],
        Value::Number(1.into())
    );
}

#[test]
fn test_bump_apply_updates_cargo_and_changelog() {
    let dir = setup_repo();
    write_cargo(dir.path(), "0.1.0");
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(cli): initial");
    tag_head(dir.path(), "v0.1.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "fix(cli): patch release");

    common::committy_cmd()
        .current_dir(&dir)
        .arg("bump")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let cargo = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(cargo.contains("version = \"0.1.1\""));
    let changelog = fs::read_to_string(dir.path().join("CHANGELOG.md")).unwrap();
    assert!(changelog.contains("0.1.1"));
}

#[test]
fn test_bump_publish_requires_confirm_before_mutating_repo() {
    let dir = setup_repo();
    write_cargo(dir.path(), "0.1.0");
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(cli): initial");
    tag_head(dir.path(), "v0.1.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "fix(cli): patch release");

    common::committy_cmd()
        .current_dir(&dir)
        .arg("bump")
        .arg("--publish")
        .assert()
        .failure();

    let cargo = fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(cargo.contains("version = \"0.1.0\""));

    let head = StdCommand::new("git")
        .args(["log", "--format=%s", "-n", "1"])
        .current_dir(&dir)
        .output()
        .expect("Failed to read git log");
    assert_eq!(
        String::from_utf8_lossy(&head.stdout).trim(),
        "fix(cli): patch release"
    );

    let tags = StdCommand::new("git")
        .args(["tag", "--list"])
        .current_dir(&dir)
        .output()
        .expect("Failed to list tags");
    assert_eq!(String::from_utf8_lossy(&tags.stdout).trim(), "v0.1.0");
}

#[test]
fn test_multi_package_version_command_and_bump_dry_run() {
    let dir = setup_repo();
    write_multi_package_config(dir.path(), &[("pkg-a", "1.0.0"), ("pkg-b", "1.0.0")]);
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(pkg-a): initial");
    tag_head(dir.path(), "v1.0.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "feat(pkg-a): add release planning");

    let version = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("version")
            .arg("--project")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(
        version["project"]["provider"],
        Value::String("multi-package".into())
    );
    assert_eq!(
        version["project"]["package_versions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let bump = parse_stdout(
        common::committy_cmd()
            .current_dir(&dir)
            .arg("bump")
            .arg("--dry-run")
            .arg("--output")
            .arg("json")
            .assert()
            .success(),
    );
    assert_eq!(
        bump["plan"]["provider"],
        Value::String("multi-package".into())
    );
    assert_eq!(
        bump["plan"]["package_versions"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn test_multi_package_bump_apply_preserves_independent_versions() {
    let dir = setup_repo();
    write_multi_package_config(dir.path(), &[("pkg-a", "1.0.0"), ("pkg-b", "2.0.0")]);
    fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    commit_all(dir.path(), "feat(pkg-a): initial");
    tag_head(dir.path(), "v1.0.0");
    fs::write(dir.path().join("tracked.txt"), "two\n").unwrap();
    commit_all(dir.path(), "feat(pkg-a): add release planning");

    common::committy_cmd()
        .current_dir(&dir)
        .arg("bump")
        .arg("--output")
        .arg("json")
        .assert()
        .success();

    let pkg_a = fs::read_to_string(dir.path().join("pkg-a/Cargo.toml")).unwrap();
    let pkg_b = fs::read_to_string(dir.path().join("pkg-b/Cargo.toml")).unwrap();
    assert!(pkg_a.contains("version = \"1.1.0\""));
    assert!(pkg_b.contains("version = \"2.1.0\""));
}
