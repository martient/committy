use super::Command;
use crate::error::CliError;
use crate::linter::CommitLinter;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct LintCommand {
    /// Path to the git repository (defaults to current directory)
    #[structopt(long, default_value = ".")]
    repo_path: String,
}

impl Command for LintCommand {
    fn execute(&self) -> Result<(), CliError> {
        let linter =
            CommitLinter::new(&self.repo_path).map_err(|e| CliError::Generic(e.to_string()))?;

        match linter.check_commits_since_last_tag() {
            Ok(issues) => {
                if issues.is_empty() {
                    println!(
                        "✅ All commits since the last tag follow the conventional commit format!"
                    );
                } else {
                    println!("❌ Found {} commit(s) with issues:", issues.len());
                    for issue in issues {
                        println!("\nCommit: {}", issue.commit_id);
                        println!("Message: {}", issue.message);
                        println!("Issue: {}", issue.issue);
                    }
                    std::process::exit(1);
                }
                Ok(())
            }
            Err(e) => Err(CliError::Generic(e.to_string())),
        }
    }
}
