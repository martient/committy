use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
use crate::error::CliError;
use crate::release::ReleaseEngine;

#[derive(Debug, StructOpt)]
pub struct InfoCommand {
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct InfoOutput {
    command: &'static str,
    ok: bool,
    dry_run: bool,
    convention: String,
    info: String,
    provider: String,
    tag_format: String,
    errors: Option<Vec<String>>,
}

impl Command for InfoCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &[])?;
        let convention = engine.convention().config();
        let provider = engine.provider_kind()?.id().to_string();

        if self.output == "json" {
            println!(
                "{}",
                serde_json::to_string(&InfoOutput {
                    command: "info",
                    ok: true,
                    dry_run: false,
                    convention: convention.name.clone(),
                    info: engine.convention().info().to_string(),
                    provider,
                    tag_format: engine.release_config().tag_format.clone(),
                    errors: None,
                })
                .map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            println!("{}", engine.convention().info());
        }
        Ok(())
    }
}
