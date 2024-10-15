mod cli;
mod git;
mod input;
mod error;
mod config;

use structopt::StructOpt;
use cli::commands::{commit::CommitCommand, CliCommand};
use error::CliError;
use config::SENTRY_DSN;

#[derive(StructOpt)]
#[structopt(name = "Committy", about = "🚀 Generate clear, concise, and structured commit messages effortlessly")]
struct Opt {
    #[structopt(subcommand)]
    cmd: Option<CliCommand>,
}

fn main() -> Result<(), CliError> {
    let _guard = sentry::init((
        SENTRY_DSN,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let opt = Opt::from_args();

    let _ = match opt.cmd {
        Some(cmd) => cmd.execute(),
        None => {
            CliCommand::Commit(CommitCommand::default()).execute()?;
            return Ok(());
        }
    };

    Ok(())
}