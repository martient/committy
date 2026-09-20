use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::output::{MachineContext, API_VERSION};
use crate::cli::Command;
use crate::config::convention::ConventionType;
use crate::error::CliError;
use crate::release::ReleaseEngine;

#[derive(Debug, StructOpt)]
pub struct SchemaCommand {
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct SchemaOutput {
    api_version: u8,
    command: &'static str,
    ok: bool,
    dry_run: bool,
    convention: String,
    schema: String,
    parser: String,
    types: Vec<String>,
    type_definitions: Vec<ConventionType>,
    capabilities: Vec<CapabilityDefinition>,
    errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CapabilityDefinition {
    name: &'static str,
    description: &'static str,
    /// Whether applying this capability changes the repository or a remote.
    mutating: bool,
    /// Whether the CLI refuses to proceed without an explicit confirmation
    /// flag. An agent must never infer this consent from the task alone.
    requires_confirmation: bool,
}

/// Read-only capability: previews and validation.
const fn read_only(name: &'static str, description: &'static str) -> CapabilityDefinition {
    CapabilityDefinition {
        name,
        description,
        mutating: false,
        requires_confirmation: false,
    }
}

/// Local mutation: changes this checkout, needs no confirmation flag.
const fn local_write(name: &'static str, description: &'static str) -> CapabilityDefinition {
    CapabilityDefinition {
        name,
        description,
        mutating: true,
        requires_confirmation: false,
    }
}

/// Remote mutation: gated behind an explicit confirmation flag.
const fn remote_write(name: &'static str, description: &'static str) -> CapabilityDefinition {
    CapabilityDefinition {
        name,
        description,
        mutating: true,
        requires_confirmation: true,
    }
}

fn capabilities() -> Vec<CapabilityDefinition> {
    vec![
        read_only(
            "schema.discover",
            "Report active convention, types, and this capability list",
        ),
        read_only(
            "branch.preview",
            "Resolve and validate a branch without changing git",
        ),
        local_write("branch.apply", "Create a validated branch"),
        read_only(
            "branch.lint",
            "Validate structured or policy-enforced branch names",
        ),
        read_only(
            "commit.preview",
            "Resolve and lint a commit without changing git",
        ),
        local_write("commit.apply", "Create a validated conventional commit"),
        local_write("commit.amend", "Preview or apply a validated amend"),
        read_only("commit.lint", "Lint one message or repository history"),
        read_only(
            "group-commit.plan",
            "Group a dirty worktree into coherent commits without changing git",
        ),
        local_write(
            "group-commit.apply",
            "Create the planned grouped commits; pushing additionally requires confirmation",
        ),
        read_only(
            "bump.preview",
            "Compute the next version from commit history without writing files",
        ),
        local_write(
            "bump.apply",
            "Write the computed version to the configured version files",
        ),
        read_only(
            "changelog.preview",
            "Render a changelog from git history without writing files",
        ),
        read_only(
            "tag.preview",
            "Compute the next tag and release plan without changing git",
        ),
        local_write(
            "tag.apply",
            "Create the computed tag in the local repository",
        ),
        remote_write(
            "tag.publish",
            "Push tags and publish the release; requires an explicit confirmation flag",
        ),
        read_only(
            "config.validate",
            "Validate repository configuration and report findings",
        ),
        read_only(
            "packages.list",
            "List configured packages and their resolved versions",
        ),
        local_write(
            "hooks.install",
            "Preview or install native Git hooks and CI enforcement",
        ),
        read_only(
            "hooks.commit-msg",
            "Enforce commit messages through a native git hook",
        ),
        read_only(
            "hooks.pre-push",
            "Enforce outgoing commit history through a native git hook",
        ),
    ]
}

impl Command for SchemaCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &[])?;
        let convention = engine.convention().config();
        let types = engine.convention().allowed_types();
        let type_definitions = engine
            .convention()
            .visible_types()
            .into_iter()
            .cloned()
            .collect();

        if self.output == "json" {
            println!(
                "{}",
                serde_json::to_string(&SchemaOutput {
                    api_version: API_VERSION,
                    command: "schema",
                    ok: true,
                    dry_run: false,
                    convention: convention.name.clone(),
                    schema: engine.convention().schema().to_string(),
                    parser: engine.convention().parser_pattern().to_string(),
                    types,
                    type_definitions,
                    capabilities: capabilities(),
                    errors: None,
                })
                .map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            println!("{}", engine.convention().schema());
        }
        Ok(())
    }

    fn machine_context(&self) -> Option<MachineContext> {
        (self.output == "json").then_some(MachineContext {
            command: "schema",
            dry_run: false,
        })
    }
}
