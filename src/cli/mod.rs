pub mod commands;

use self::commands::{
    amend, branch, bump, changelog, commit, config, example, group_commit, info, init, lint,
    lint_message, ls, packages, schema, tag, version,
};
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
    #[structopt(about = "Plan or apply a release bump")]
    Bump(bump::BumpCommand),
    #[structopt(about = "Render a changelog from git history")]
    Changelog(changelog::ChangelogCommand),
    #[structopt(about = "Check commits since last tag for conventional format")]
    Lint(lint::LintCommand),
    #[structopt(about = "Lint a single commit message for conventional format")]
    LintMessage(lint_message::LintMessageCommand),
    #[structopt(about = "Print convention examples")]
    Example(example::ExampleCommand),
    #[structopt(about = "Print active convention info")]
    Info(info::InfoCommand),
    #[structopt(about = "List built-in conventions, providers, and templates")]
    Ls(ls::LsCommand),
    #[structopt(about = "Print active convention schema and parser metadata")]
    Schema(schema::SchemaCommand),
    #[structopt(about = "Print the Committy and project versions")]
    Version(version::VersionCommand),
    #[structopt(about = "Create a new branch")]
    Branch(branch::BranchCommand),
    #[structopt(about = "Group changes and optionally commit/apply them (with optional AI)")]
    GroupCommit(group_commit::GroupCommitCommand),
    #[structopt(about = "Initialize multi-package support")]
    Init(init::InitCommand),
    #[structopt(about = "Manage repository configuration")]
    Config(config::ConfigCommand),
    #[structopt(about = "Manage packages in multi-package repositories")]
    Packages(packages::PackagesCommand),
}

impl CliCommand {
    pub fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        match self {
            CliCommand::Commit(cmd) => cmd.execute(non_interactive),
            CliCommand::Amend(cmd) => cmd.execute(non_interactive),
            CliCommand::Tag(cmd) => cmd.execute(non_interactive),
            CliCommand::Bump(cmd) => cmd.execute(non_interactive),
            CliCommand::Changelog(cmd) => cmd.execute(non_interactive),
            CliCommand::Lint(cmd) => cmd.execute(non_interactive),
            CliCommand::LintMessage(cmd) => cmd.execute(non_interactive),
            CliCommand::Example(cmd) => cmd.execute(non_interactive),
            CliCommand::Info(cmd) => cmd.execute(non_interactive),
            CliCommand::Ls(cmd) => cmd.execute(non_interactive),
            CliCommand::Schema(cmd) => cmd.execute(non_interactive),
            CliCommand::Version(cmd) => cmd.execute(non_interactive),
            CliCommand::Branch(cmd) => cmd.execute(non_interactive),
            CliCommand::GroupCommit(cmd) => cmd.execute(non_interactive),
            CliCommand::Init(cmd) => cmd.execute(non_interactive),
            CliCommand::Config(cmd) => cmd.execute(non_interactive),
            CliCommand::Packages(cmd) => cmd.execute(non_interactive),
        }
    }
}
