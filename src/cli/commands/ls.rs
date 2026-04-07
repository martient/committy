use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
use crate::error::CliError;
use crate::release::{builtin_provider_ids, builtin_template_ids, ReleaseEngine};

#[derive(Debug, StructOpt)]
pub struct LsCommand {
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct LsOutput {
    command: &'static str,
    ok: bool,
    dry_run: bool,
    conventions: Vec<String>,
    providers: Vec<&'static str>,
    templates: Vec<&'static str>,
    errors: Option<Vec<String>>,
}

impl Command for LsCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &[])?;
        let output = LsOutput {
            command: "ls",
            ok: true,
            dry_run: false,
            conventions: vec![engine.convention().config().name.clone()],
            providers: builtin_provider_ids(),
            templates: builtin_template_ids(),
            errors: None,
        };

        if self.output == "json" {
            println!(
                "{}",
                serde_json::to_string(&output).map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            for convention in output.conventions {
                println!("convention {convention}");
            }
            for provider in output.providers {
                println!("provider {provider}");
            }
            for template in output.templates {
                println!("template {template}");
            }
        }
        Ok(())
    }
}
