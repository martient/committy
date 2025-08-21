use crate::cli::Command;
use crate::error::CliError;
use crate::linter::CommitLinter;
use serde::Serialize;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct LintCommand {
    /// Path to the git repository (defaults to current directory)
    #[structopt(long, default_value = ".")]
    repo_path: String,

    /// Output format: text or json
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,
}

impl Command for LintCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let linter =
            CommitLinter::new(&self.repo_path).map_err(|e| CliError::Generic(e.to_string()))?;

        match linter.check_commits_since_last_tag() {
            Ok(issues) => {
                if self.output == "json" {
                    #[derive(Serialize)]
                    struct LintOutput<'a> {
                        ok: bool,
                        count: usize,
                        issues: &'a [crate::linter::CommitIssue],
                    }
                    let payload = LintOutput {
                        ok: issues.is_empty(),
                        count: issues.len(),
                        issues: &issues,
                    };
                    println!("{}", serde_json::to_string(&payload).unwrap());
                } else if issues.is_empty() {
                    println!(
                        "✅ All commits since the last tag follow the conventional commit format!"
                    );
                } else {
                    println!("❌ Found {} commit(s) with issues:", issues.len());
                    for issue in &issues {
                        println!("\nCommit: {}", issue.commit_id);
                        println!("Message: {}", issue.message);
                        println!("Issue: {}", issue.issue);
                    }
                }

                if issues.is_empty() {
                    Ok(())
                } else {
                    Err(CliError::LintIssues(issues.len()))
                }
            }
            Err(e) => Err(CliError::Generic(e.to_string())),
        }
    }
}
