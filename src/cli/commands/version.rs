use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
use crate::error::CliError;
use crate::release::ReleaseEngine;

#[derive(Debug, StructOpt)]
pub struct VersionCommand {
    #[structopt(long, help = "Show only the Committy binary version")]
    tool: bool,

    #[structopt(long, help = "Show only the project version")]
    project: bool,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct VersionOutput {
    command: &'static str,
    ok: bool,
    dry_run: bool,
    tool_version: Option<String>,
    project: Option<crate::release::ProjectVersion>,
    errors: Option<Vec<String>>,
}

impl Command for VersionCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let show_tool = self.tool || !self.project;
        let show_project = self.project || !self.tool;
        let project = if show_project {
            Some(ReleaseEngine::load(&self.repo_path, &[])?.project_version()?)
        } else {
            None
        };
        let tool_version = show_tool.then(|| env!("CARGO_PKG_VERSION").to_string());

        if self.output == "json" {
            println!(
                "{}",
                serde_json::to_string(&VersionOutput {
                    command: "version",
                    ok: true,
                    dry_run: false,
                    tool_version,
                    project,
                    errors: None,
                })
                .map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            if let Some(tool_version) = &tool_version {
                println!("committy {tool_version}");
            }
            if let Some(project) = &project {
                println!(
                    "{} {}",
                    project.provider,
                    project.version.as_deref().unwrap_or("unknown")
                );
                if let Some(package_versions) = &project.package_versions {
                    for (name, version) in package_versions {
                        println!("{name} {version}");
                    }
                }
            }
        }
        Ok(())
    }
}
