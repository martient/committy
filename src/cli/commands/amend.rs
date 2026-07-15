use crate::cli::commands::commit::CommitCommand;
use crate::cli::output::MachineContext;
use crate::cli::Command;
use crate::error::CliError;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct AmendCommand {
    #[structopt(long = "type", help = "Type of commit (e.g., feat, fix, docs)")]
    commit_type: Option<String>,

    #[structopt(long, help = "Scope of the commit")]
    scope: Option<String>,

    #[structopt(long, help = "Short commit message")]
    message: Option<String>,

    #[structopt(long, help = "Long/detailed commit message")]
    long_message: Option<String>,

    #[structopt(
        long = "message-file",
        parse(from_os_str),
        help = "Read the full commit message from a file"
    )]
    message_file: Option<PathBuf>,

    #[structopt(long, help = "Mark this as a breaking change")]
    breaking_change: bool,

    #[structopt(long, help = "Preview the amend without writing to git")]
    dry_run: bool,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(
        long = "git-config",
        help = "Pass through git -c key=value overrides (repeatable)"
    )]
    git_config: Vec<String>,

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
            message_file: self.message_file.clone(),
            breaking_change: self.breaking_change,
            amend: true,
            dry_run: self.dry_run,
            output: self.output.clone(),
            git_config: self.git_config.clone(),
            repo_path: self.repo_path.clone(),
        };

        commit.execute_with_command_name(non_interactive, "amend")
    }

    fn machine_context(&self) -> Option<MachineContext> {
        (self.output == "json").then_some(MachineContext {
            command: "amend",
            dry_run: self.dry_run,
        })
    }
}

impl Default for AmendCommand {
    fn default() -> Self {
        Self {
            commit_type: None,
            scope: None,
            message: None,
            long_message: None,
            message_file: None,
            breaking_change: false,
            dry_run: false,
            output: "text".to_string(),
            git_config: vec![],
            repo_path: PathBuf::from("."),
        }
    }
}
