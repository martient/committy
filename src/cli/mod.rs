pub mod commands;

use self::commands::{amend, branch, commit, group_commit, lint, lint_message, tag, tui};
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
    #[structopt(about = "Lint a single commit message for conventional format")]
    LintMessage(lint_message::LintMessageCommand),
    #[structopt(about = "Create a new branch")]
    Branch(branch::BranchCommand),
    #[structopt(about = "Group changes and optionally commit/apply them (with optional AI)")]
    GroupCommit(group_commit::GroupCommitCommand),
    #[structopt(about = "Interactive TUI for staging and committing changes")]
    Tui(tui::TuiCommand),
}

impl CliCommand {
    pub fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        match self {
            CliCommand::Commit(cmd) => cmd.execute(non_interactive),
            CliCommand::Amend(cmd) => cmd.execute(non_interactive),
            CliCommand::Tag(cmd) => cmd.execute(non_interactive),
            CliCommand::Lint(cmd) => cmd.execute(non_interactive),
            CliCommand::LintMessage(cmd) => cmd.execute(non_interactive),
            CliCommand::Branch(cmd) => cmd.execute(non_interactive),
            CliCommand::GroupCommit(cmd) => cmd.execute(non_interactive),
            CliCommand::Tui(cmd) => cmd.execute(non_interactive),
        }
    }
}
