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
}

fn capabilities() -> Vec<CapabilityDefinition> {
    vec![
        CapabilityDefinition {
            name: "branch.preview",
            description: "Resolve and validate a branch without changing git",
        },
        CapabilityDefinition {
            name: "branch.apply",
            description: "Create a validated branch",
        },
        CapabilityDefinition {
            name: "branch.lint",
            description: "Validate structured or policy-enforced branch names",
        },
        CapabilityDefinition {
            name: "commit.preview",
            description: "Resolve and lint a commit without changing git",
        },
        CapabilityDefinition {
            name: "commit.apply",
            description: "Create a validated conventional commit",
        },
        CapabilityDefinition {
            name: "commit.amend",
            description: "Preview or apply a validated amend",
        },
        CapabilityDefinition {
            name: "commit.lint",
            description: "Lint one message or repository history",
        },
        CapabilityDefinition {
            name: "hooks.install",
            description: "Preview or install native Git hooks and CI enforcement",
        },
        CapabilityDefinition {
            name: "hooks.commit-msg",
            description: "Enforce commit messages through a native git hook",
        },
        CapabilityDefinition {
            name: "hooks.pre-push",
            description: "Enforce outgoing commit history through a native git hook",
        },
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
