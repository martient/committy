pub mod commands;

use self::commands::Command;
use crate::error::CliError;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "committy", about = "A tool for managing git commits")]
pub enum Cli {
    /// Create a new commit following conventional commit format
    Commit(commands::commit::CommitCommand),
    /// Check if commits since the last tag follow conventional commit format
    Lint(commands::lint::LintCommand),
    /// Tag the current commit with a version
    Tag(commands::tag::TagCommand),
}

impl Cli {
    pub fn execute(&self) -> Result<(), CliError> {
        match self {
            Cli::Commit(cmd) => cmd.execute(),
            Cli::Lint(cmd) => cmd.execute(),
            Cli::Tag(cmd) => cmd.execute(),
        }
    }
}
