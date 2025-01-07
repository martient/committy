pub mod commands;

use self::commands::{amend, commit, lint, tag};
use crate::error::CliError;
use structopt::StructOpt;

pub trait Command {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError>;
}

#[derive(StructOpt)]
pub enum CliCommand {
    #[structopt(about = "Create a new commit")]
    Commit(commit::CommitCommand),
    #[structopt(about = "Amend the previous commit")]
    Amend(amend::AmendCommand),
    #[structopt(about = "Create a new tag")]
    Tag(tag::TagCommand),
    #[structopt(about = "Check commits since last tag for conventional format")]
    Lint(lint::LintCommand),
}

impl CliCommand {
    pub fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        match self {
            CliCommand::Commit(cmd) => cmd.execute(non_interactive),
            CliCommand::Amend(cmd) => cmd.execute(non_interactive),
            CliCommand::Tag(cmd) => cmd.execute(non_interactive),
            CliCommand::Lint(cmd) => cmd.execute(non_interactive),
        }
    }
}
