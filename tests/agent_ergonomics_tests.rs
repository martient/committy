//! Agent ergonomics: the seams where AI coding agents actually trip.
//!
//! Three classes of failure are covered here:
//!   1. Global flags must parse on either side of the subcommand.
//!   2. Argument-parse failures must honour `--output json` like every other error.
//!   3. `schema` must advertise the whole command surface, with consent metadata.

mod common;

use serde_json::Value;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_repo() -> tempfile::TempDir {
    common::setup_test_env();

    let dir = tempdir().expect("Failed to create temp directory");

    for args in [
        vec!["init"],
        vec!["config", "user.name", "Test User"],
        vec!["config", "user.email", "test@example.com"],
    ] {
        StdCommand::new("git")
            .args(&args)
            .current_dir(&dir)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?} failed: {e}"));
    }

    fs::write(dir.path().join("tracked.txt"), "initial\n").expect("Failed to write tracked file");
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

fn stdout_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone()).expect("stdout was not valid UTF-8")
}

// ---------------------------------------------------------------------------
// 1. Global flag placement
// ---------------------------------------------------------------------------

/// The documented form. Guards against a regression while fixing the rest.
#[test]
fn test_global_flags_before_subcommand_still_parse() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args([
            "--non-interactive",
            "-q",
            "schema",
            "--repo-path",
            ".",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim()).unwrap();
    assert_eq!(v["command"], Value::String("schema".into()));
}

/// The form agents actually write: global flags after the subcommand.
#[test]
fn test_non_interactive_parses_after_subcommand() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args([
            "schema",
            "--non-interactive",
            "--repo-path",
            ".",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim()).unwrap();
    assert_eq!(v["command"], Value::String("schema".into()));
}

#[test]
fn test_quiet_parses_after_subcommand() {
    let temp_dir = setup_repo();

    for flag in ["-q", "--quiet"] {
        let assert = common::committy_cmd()
            .current_dir(&temp_dir)
            .args(["schema", flag, "--repo-path", ".", "--output", "json"])
            .assert()
            .success();

        let v: Value = serde_json::from_str(stdout_of(&assert).trim())
            .unwrap_or_else(|e| panic!("`schema {flag}` did not emit JSON: {e}"));
        assert_eq!(v["command"], Value::String("schema".into()));
    }
}

/// `--verbose` is deliberately *not* global.
///
/// `-v` is `branch --validate` and `--verbose` is a real flag on
/// `config validate|show`. Promoting the root flag would silently steal both.
/// This test pins that trade-off so a later change has to confront it.
#[test]
fn test_verbose_stays_root_only_because_subcommands_own_it() {
    let temp_dir = setup_repo();

    common::committy_cmd()
        .current_dir(&temp_dir)
        .args([
            "--verbose",
            "schema",
            "--repo-path",
            ".",
            "--output",
            "json",
        ])
        .assert()
        .success();

    // `branch -v` must still mean `--validate`, not root verbosity.
    common::committy_cmd()
        .current_dir(&temp_dir)
        .args([
            "branch",
            "-v",
            "--name",
            "feat-keeps-validate",
            "--output",
            "json",
        ])
        .assert()
        .success();
}

/// Mutating commands take the same treatment, not just read-only ones.
#[test]
fn test_global_flags_after_subcommand_on_branch_and_commit() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args([
            "branch",
            "--non-interactive",
            "--name",
            "feat-agent-ergonomics",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert()
        .success();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim()).unwrap();
    assert_eq!(v["command"], Value::String("branch".into()));
    assert_eq!(v["dry_run"], Value::Bool(true));

    // `commit` reports a command-level error here (nothing staged), but it must
    // be a *parsed* command-level error, not an argv rejection.
    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args([
            "commit",
            "--non-interactive",
            "--type",
            "feat",
            "--message",
            "agent ergonomics",
            "--dry-run",
            "--output",
            "json",
        ])
        .assert();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim())
        .expect("commit must emit a JSON envelope, not an argv rejection");
    assert_eq!(v["command"], Value::String("commit".into()));
}

// ---------------------------------------------------------------------------
// 2. Argument-parse errors must honour --output json
// ---------------------------------------------------------------------------

#[test]
fn test_unknown_flag_with_json_output_emits_error_envelope() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args(["schema", "--output", "json", "--definitely-not-a-flag"])
        .assert()
        .failure();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim())
        .expect("an argv rejection must still produce one JSON document on stdout");

    assert_eq!(v["api_version"], Value::from(1));
    assert_eq!(v["command"], Value::String("schema".into()));
    assert_eq!(v["ok"], Value::Bool(false));
    assert_eq!(
        v["errors"][0]["code"],
        Value::String("invalid_usage".into())
    );
    assert!(
        v["errors"][0]["message"]
            .as_str()
            .expect("message must be a string")
            .contains("--definitely-not-a-flag"),
        "the envelope must name the offending argument so an agent can self-correct"
    );
}

#[test]
fn test_unknown_flag_with_equals_form_json_output_emits_envelope() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args(["schema", "--output=json", "--definitely-not-a-flag"])
        .assert()
        .failure();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim())
        .expect("--output=json must be recognised as well as --output json");
    assert_eq!(v["ok"], Value::Bool(false));
    assert_eq!(
        v["errors"][0]["code"],
        Value::String("invalid_usage".into())
    );
}

/// An unknown subcommand cannot be attributed to a command; the envelope still
/// has to be valid so the agent's parser does not blow up.
#[test]
fn test_unknown_subcommand_with_json_output_emits_envelope() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args(["nonexistent-command", "--output", "json"])
        .assert()
        .failure();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim())
        .expect("an unknown subcommand must still produce one JSON document");
    assert_eq!(v["ok"], Value::Bool(false));
    assert_eq!(v["command"], Value::String("unknown".into()));
}

/// Humans keep the terminal-friendly behaviour: plain text on stderr, no JSON.
#[test]
fn test_unknown_flag_without_json_output_stays_human_readable() {
    let temp_dir = setup_repo();

    let assert = common::committy_cmd()
        .current_dir(&temp_dir)
        .args(["schema", "--definitely-not-a-flag"])
        .assert()
        .failure();

    assert!(
        stdout_of(&assert).trim().is_empty(),
        "text mode must not print a JSON document to stdout"
    );
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("--definitely-not-a-flag"),
        "text mode must still explain the problem on stderr"
    );
}

// ---------------------------------------------------------------------------
// 3. Capability discovery
// ---------------------------------------------------------------------------

fn capabilities_of(temp_dir: &tempfile::TempDir) -> Vec<Value> {
    let assert = common::committy_cmd()
        .current_dir(temp_dir)
        .args(["--non-interactive", "schema", "--output", "json"])
        .assert()
        .success();

    let v: Value = serde_json::from_str(stdout_of(&assert).trim()).unwrap();
    v["capabilities"]
        .as_array()
        .expect("schema must expose a capabilities array")
        .clone()
}

#[test]
fn test_capabilities_cover_the_whole_agent_command_surface() {
    let temp_dir = setup_repo();
    let capabilities = capabilities_of(&temp_dir);

    let names: Vec<&str> = capabilities
        .iter()
        .map(|c| c["name"].as_str().expect("capability needs a name"))
        .collect();

    // Every command the shipped skills instruct an agent to run.
    for expected in [
        "branch.preview",
        "branch.apply",
        "branch.lint",
        "commit.preview",
        "commit.apply",
        "commit.amend",
        "commit.lint",
        "group-commit.plan",
        "group-commit.apply",
        "bump.preview",
        "bump.apply",
        "changelog.preview",
        "tag.preview",
        "tag.apply",
        "tag.publish",
        "config.validate",
        "packages.list",
        "hooks.install",
        "hooks.commit-msg",
        "hooks.pre-push",
        "schema.discover",
    ] {
        assert!(
            names.contains(&expected),
            "capability `{expected}` is missing; skills hard-code it today. Present: {names:?}"
        );
    }
}

#[test]
fn test_capabilities_declare_consent_metadata() {
    let temp_dir = setup_repo();
    let capabilities = capabilities_of(&temp_dir);

    for capability in &capabilities {
        let name = capability["name"].as_str().unwrap();
        assert!(
            capability["mutating"].is_boolean(),
            "capability `{name}` must declare `mutating`"
        );
        assert!(
            capability["requires_confirmation"].is_boolean(),
            "capability `{name}` must declare `requires_confirmation`"
        );
        assert!(
            capability["description"].is_string(),
            "capability `{name}` must keep its description"
        );
    }

    let by_name = |needle: &str| -> Value {
        capabilities
            .iter()
            .find(|c| c["name"] == Value::String(needle.into()))
            .unwrap_or_else(|| panic!("missing capability {needle}"))
            .clone()
    };

    // Previews never touch the repository.
    for read_only in ["branch.preview", "commit.preview", "schema.discover"] {
        assert_eq!(
            by_name(read_only)["mutating"],
            Value::Bool(false),
            "{read_only} must not be marked mutating"
        );
        assert_eq!(
            by_name(read_only)["requires_confirmation"],
            Value::Bool(false)
        );
    }

    // Local writes mutate but do not need the remote-mutation confirmation flag.
    assert_eq!(by_name("commit.apply")["mutating"], Value::Bool(true));
    assert_eq!(
        by_name("commit.apply")["requires_confirmation"],
        Value::Bool(false)
    );

    // Remote mutation is the case an agent must never infer consent for.
    assert_eq!(by_name("tag.publish")["mutating"], Value::Bool(true));
    assert_eq!(
        by_name("tag.publish")["requires_confirmation"],
        Value::Bool(true),
        "publishing must advertise that it needs explicit confirmation"
    );
}

// ---------------------------------------------------------------------------
// 4. The escape hatch must be discoverable where agents read
// ---------------------------------------------------------------------------

#[test]
fn test_agent_docs_mention_the_non_interactive_env_var() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    // `AGENTS.md` is canonical. Per-runtime files may either restate the rule
    // or defer to it; what must never happen is a runtime being told nothing.
    let agent_facing = [
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        ".github/copilot-instructions.md",
        "docs/src/content/docs/reference/agent-workflows.mdx",
    ];

    for relative in agent_facing {
        let body = fs::read_to_string(repo_root.join(relative))
            .unwrap_or_else(|e| panic!("cannot read {relative}: {e}"));
        let defers_to_canonical = body.contains("AGENTS.md");
        assert!(
            body.contains("COMMITTY_NONINTERACTIVE") || defers_to_canonical,
            "{relative} must document COMMITTY_NONINTERACTIVE or point at AGENTS.md; \
             it is the escape hatch that makes flag placement irrelevant for agents"
        );

        // Documenting `-v` as global would be worse than not documenting
        // placement at all: it reads as permission to write `schema -v`.
        assert!(
            !body.contains("`--non-interactive`, `-q`, `-v`"),
            "{relative} lists -v among the global flags, but --verbose is root-only"
        );
    }
}

/// The cross-tool skill discovery path must resolve to the real skills.
///
/// `.agents/skills` is a symlink. If a checkout materialises it as a plain
/// file (Windows without developer mode, or `core.symlinks=false`), discovery
/// silently breaks for every runtime that reads that path.
#[test]
fn test_skills_are_discoverable_at_the_cross_tool_path() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let cross_tool = repo_root.join(".agents/skills");

    assert!(
        cross_tool.is_dir(),
        ".agents/skills must resolve to a directory; if this is a plain file, \
         the symlink did not survive checkout"
    );

    for skill in [
        "committy-branch",
        "committy-commit",
        "committy-enforce",
        "committy-release",
    ] {
        assert!(
            cross_tool.join(skill).join("SKILL.md").is_file(),
            "{skill}/SKILL.md must be reachable via .agents/skills"
        );
    }
}
