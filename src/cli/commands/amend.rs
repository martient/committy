use crate::cli::commands::commit::CommitCommand;
use crate::cli::Command;
use crate::error::CliError;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Default)]
pub struct AmendCommand {
    #[structopt(long = "type", help = "Type of commit (e.g., feat, fix, docs)")]
    commit_type: Option<String>,

    #[structopt(long, help = "Scope of the commit")]
    scope: Option<String>,

    #[structopt(long, help = "Short commit message")]
    message: Option<String>,

    #[structopt(long, help = "Long/detailed commit message")]
    long_message: Option<String>,

    #[structopt(long, help = "Mark this as a breaking change")]
    breaking_change: bool,

    #[structopt(long, help = "Preview the amend without writing to git")]
    dry_run: bool,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

impl Command for AmendCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        let commit = CommitCommand {
            commit_type: self.commit_type.clone(),
            scope: self.scope.clone(),
            message: self.message.clone(),
            long_message: self.long_message.clone(),
            breaking_change: self.breaking_change,
            amend: true,
            dry_run: self.dry_run,
            output: self.output.clone(),
            repo_path: self.repo_path.clone(),
        };

        commit.execute_with_command_name(non_interactive, "amend")
    }
}
