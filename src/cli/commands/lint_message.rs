use crate::cli::Command;
use crate::error::CliError;
use crate::linter::check_message_format;
use serde::Serialize;
use std::fs;
use std::io::{self, Read};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "lint-message",
    about = "Lint a single commit message for conventional format"
)]
pub struct LintMessageCommand {
    /// Commit message to lint (use --file or stdin if omitted)
    #[structopt(long)]
    message: Option<String>,

    /// Path to a file containing the commit message (e.g., .git/COMMIT_EDITMSG)
    #[structopt(long)]
    file: Option<String>,

    /// Output format: text or json
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,
}

impl Command for LintMessageCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        // Source the message
        let msg = if let Some(m) = &self.message {
            m.clone()
        } else if let Some(path) = &self.file {
            fs::read_to_string(path).map_err(|e| CliError::Generic(e.to_string()))?
        } else {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| CliError::Generic(e.to_string()))?;
            buf
        };

        let issues = check_message_format(&msg);

        if self.output == "json" {
            #[derive(Serialize)]
            struct LintMessageOutput<'a> {
                ok: bool,
                count: usize,
                issues: &'a [String],
            }
            let payload = LintMessageOutput {
                ok: issues.is_empty(),
                count: issues.len(),
                issues: &issues,
            };
            println!("{}", serde_json::to_string(&payload).unwrap());
        } else if issues.is_empty() {
            println!("✅ Commit message is valid!");
        } else {
            println!("❌ Found {} issue(s):", issues.len());
            for issue in &issues {
                println!("- {issue}");
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(CliError::LintIssues(issues.len()))
        }
    }
}
