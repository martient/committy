use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
use crate::error::CliError;
use crate::release::ReleaseEngine;

#[derive(Debug, StructOpt)]
pub struct ExampleCommand {
    #[structopt(long, default_value = "1")]
    count: usize,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct ExampleOutput {
    command: &'static str,
    ok: bool,
    dry_run: bool,
    convention: String,
    examples: Vec<String>,
    errors: Option<Vec<String>>,
}

impl Command for ExampleCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &[])?;
        let config = engine.convention().config();
        let examples = engine
            .convention()
            .examples()
            .iter()
            .cloned()
            .cycle()
            .take(self.count.max(1))
            .collect::<Vec<_>>();

        if self.output == "json" {
            println!(
                "{}",
                serde_json::to_string(&ExampleOutput {
                    command: "example",
                    ok: true,
                    dry_run: false,
                    convention: config.name.clone(),
                    examples,
                    errors: None,
                })
                .map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            for example in examples {
                println!("{example}");
            }
        }
        Ok(())
    }
}
