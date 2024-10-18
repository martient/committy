mod cli;
mod git;
mod input;
mod error;
mod config;
mod release;

use structopt::StructOpt;
use cli::commands::{commit::CommitCommand, CliCommand};
use error::CliError;
use config::SENTRY_DSN;
use env_logger::{Env, Builder};
use log::LevelFilter;

#[derive(StructOpt)]
#[structopt(name = "Committy", about = "🚀 Generate clear, concise, and structured commit messages effortlessly")]
struct Opt {
    #[structopt(subcommand)]
    cmd: Option<CliCommand>,
}

fn main() -> Result<(), CliError> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

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