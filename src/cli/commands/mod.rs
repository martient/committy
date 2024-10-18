pub mod commit;
pub mod amend;
pub mod tag;

use crate::error::CliError;
use structopt::StructOpt;

pub trait Command {
    fn execute(&self) -> Result<(), CliError>;
}

#[derive(StructOpt)]
pub enum CliCommand {
    #[structopt(about = "Create a new commit")]
    Commit(commit::CommitCommand),
    #[structopt(about = "Amend the previous commit")]
    Amend(amend::AmendCommand),
    #[structopt(about = "Create a new tag")]
    Tag(tag::TagCommand),
}

impl CliCommand {
    pub fn execute(&self) -> Result<(), CliError> {
        match self {
            CliCommand::Commit(cmd) => cmd.execute(),
            CliCommand::Amend(cmd) => cmd.execute(),
            CliCommand::Tag(cmd) => cmd.execute(),
        }
    }

    pub fn default() -> Self {
        CliCommand::Commit(commit::CommitCommand::default())
    }
}