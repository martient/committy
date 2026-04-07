use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
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
    command: &'static str,
    ok: bool,
    dry_run: bool,
    convention: String,
    schema: String,
    parser: String,
    types: Vec<String>,
    errors: Option<Vec<String>>,
}

impl Command for SchemaCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &[])?;
        let convention = engine.convention().config();
        let types = engine.convention().allowed_types();

        if self.output == "json" {
            println!(
                "{}",
                serde_json::to_string(&SchemaOutput {
                    command: "schema",
                    ok: true,
                    dry_run: false,
                    convention: convention.name.clone(),
                    schema: engine.convention().schema().to_string(),
                    parser: engine.convention().parser_pattern().to_string(),
                    types,
                    errors: None,
                })
                .map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            println!("{}", engine.convention().schema());
        }
        Ok(())
    }
}
