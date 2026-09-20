//! Public-seam tests for the agent plugin surface.
//!
//! `AGENTS.md` asks for tests over "machine output, exit codes, Git behavior,
//! hook protocol, and plugin manifests". The first four were covered; these
//! close the last. A typo in any of these files currently ships silently,
//! because nothing compiles or executes them.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(relative: &str) -> Value {
    let path = repo_root().join(relative);
    let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"));
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("{relative} is not valid JSON: {e}"))
}

fn skill_dirs() -> Vec<PathBuf> {
    let skills = repo_root().join("plugins/committy/skills");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&skills)
        .expect("plugins/committy/skills must exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty(), "no skills found");
    dirs
}

/// Split `SKILL.md` into its YAML frontmatter and body.
fn frontmatter_of(skill_md: &Path) -> String {
    let body = fs::read_to_string(skill_md)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", skill_md.display()));
    let rest = body
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("{} must open with YAML frontmatter", skill_md.display()));
    let end = rest
        .find("\n---\n")
        .unwrap_or_else(|| panic!("{} frontmatter is not terminated", skill_md.display()));
    rest[..end].to_string()
}

// ---------------------------------------------------------------------------
// Marketplace and plugin manifests
// ---------------------------------------------------------------------------

#[test]
fn claude_marketplace_manifest_is_well_formed() {
    let manifest = read_json(".claude-plugin/marketplace.json");

    assert!(manifest["name"].is_string(), "marketplace needs a name");
    let plugins = manifest["plugins"]
        .as_array()
        .expect("marketplace needs a plugins array");
    assert!(!plugins.is_empty());

    for plugin in plugins {
        let source = plugin["source"]
            .as_str()
            .expect("each plugin entry needs a string source");
        let resolved = repo_root().join(source.trim_start_matches("./"));
        assert!(
            resolved.is_dir(),
            "marketplace points at `{source}`, which does not exist"
        );
        assert!(
            resolved.join(".claude-plugin/plugin.json").is_file(),
            "`{source}` has no .claude-plugin/plugin.json"
        );
    }
}

#[test]
fn codex_marketplace_manifest_is_well_formed() {
    let manifest = read_json(".agents/plugins/marketplace.json");

    let plugins = manifest["plugins"]
        .as_array()
        .expect("marketplace needs a plugins array");
    assert!(!plugins.is_empty());

    for plugin in plugins {
        let path = plugin["source"]["path"]
            .as_str()
            .expect("each plugin entry needs source.path");
        let resolved = repo_root().join(path.trim_start_matches("./"));
        assert!(
            resolved.is_dir(),
            "marketplace points at `{path}`, which does not exist"
        );
        assert!(
            resolved.join(".codex-plugin/plugin.json").is_file(),
            "`{path}` has no .codex-plugin/plugin.json"
        );
    }
}

#[test]
fn plugin_manifests_agree_on_identity() {
    let claude = read_json("plugins/committy/.claude-plugin/plugin.json");
    let codex = read_json("plugins/committy/.codex-plugin/plugin.json");

    for field in ["name", "version", "license"] {
        assert_eq!(
            claude[field], codex[field],
            "plugin manifests disagree on `{field}`; they describe the same plugin"
        );
    }
    assert!(claude["name"].is_string());
    assert!(
        claude["version"].is_string(),
        "plugin needs a version string"
    );
}

#[test]
fn codex_manifest_skills_path_resolves() {
    let codex = read_json("plugins/committy/.codex-plugin/plugin.json");
    let skills = codex["skills"]
        .as_str()
        .expect("codex manifest declares a skills path");
    let resolved = repo_root()
        .join("plugins/committy")
        .join(skills.trim_start_matches("./"));
    assert!(
        resolved.is_dir(),
        "codex manifest points skills at `{skills}`, which does not resolve"
    );
}

// ---------------------------------------------------------------------------
// SKILL.md frontmatter, against the Agent Skills spec
// ---------------------------------------------------------------------------

/// The spec fixes the allowed key set; anything else fails validation in a
/// spec-compliant runtime, so an unrecognised key here is a shipping bug.
const SPEC_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "allowed-tools",
    "metadata",
    "compatibility",
];

#[test]
fn skill_frontmatter_uses_only_spec_keys() {
    for dir in skill_dirs() {
        let skill_md = dir.join("SKILL.md");
        let frontmatter = frontmatter_of(&skill_md);

        let keys: BTreeSet<String> = frontmatter
            .lines()
            .filter(|line| !line.starts_with(char::is_whitespace) && !line.trim().is_empty())
            .filter_map(|line| line.split_once(':').map(|(key, _)| key.trim().to_string()))
            .collect();

        for key in &keys {
            assert!(
                SPEC_KEYS.contains(&key.as_str()),
                "{}: `{key}` is not an Agent Skills frontmatter key (allowed: {SPEC_KEYS:?})",
                skill_md.display()
            );
        }
        for required in ["name", "description"] {
            assert!(
                keys.contains(required),
                "{}: `{required}` is required",
                skill_md.display()
            );
        }
    }
}

#[test]
fn skill_name_matches_its_directory() {
    for dir in skill_dirs() {
        let expected = dir.file_name().unwrap().to_str().unwrap();
        let frontmatter = frontmatter_of(&dir.join("SKILL.md"));
        let name = frontmatter
            .lines()
            .find_map(|line| line.strip_prefix("name:"))
            .map(str::trim)
            .expect("skill needs a name");
        assert_eq!(
            name, expected,
            "skill directory and frontmatter name must match, or discovery breaks"
        );
    }
}

#[test]
fn skill_descriptions_say_when_to_use_the_skill() {
    for dir in skill_dirs() {
        let frontmatter = frontmatter_of(&dir.join("SKILL.md"));
        let description = frontmatter
            .lines()
            .find_map(|line| line.strip_prefix("description:"))
            .map(str::trim)
            .expect("skill needs a description");

        // Description is the only thing a runtime sees before loading the body,
        // so it has to carry the trigger, not just the capability.
        assert!(
            description.len() > 40,
            "{}: description is too thin to route on: {description}",
            dir.display()
        );
        assert!(
            description.contains("Use when"),
            "{}: description must say when to use the skill: {description}",
            dir.display()
        );
    }
}

#[test]
fn skills_do_not_use_runtime_specific_invocation_syntax() {
    for dir in skill_dirs() {
        let skill_md = dir.join("SKILL.md");
        let body = fs::read_to_string(&skill_md).unwrap();

        // `$skill` is Codex syntax; Claude Code uses `/plugin:skill`. The body
        // is shared by both runtimes, so neither form belongs in it.
        assert!(
            !body.contains("$committy-"),
            "{}: uses Codex `$skill` syntax in a body shared with Claude Code",
            skill_md.display()
        );
        assert!(
            !body.contains("/committy:committy-"),
            "{}: uses Claude `/plugin:skill` syntax in a body shared with Codex",
            skill_md.display()
        );
    }
}

#[test]
fn every_skill_is_reachable_from_both_discovery_paths() {
    for dir in skill_dirs() {
        let name = dir.file_name().unwrap();
        assert!(
            repo_root()
                .join(".agents/skills")
                .join(name)
                .join("SKILL.md")
                .is_file(),
            "{name:?} is not reachable via the cross-tool .agents/skills path"
        );
    }
}
