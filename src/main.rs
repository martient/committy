mod cli;
mod config;
mod error;
mod git;
mod input;
mod linter;
mod release;
mod version;

use config::SENTRY_DSN;
use env_logger::{Builder, Env};
use structopt::StructOpt;

use crate::cli::commands::commit::CommitCommand;
use crate::cli::CliCommand;

#[derive(StructOpt)]
#[structopt(
    name = env!("CARGO_PKG_NAME"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION")
)]
struct Opt {
    #[structopt(subcommand)]
    cmd: Option<CliCommand>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let _guard = sentry::init((
        SENTRY_DSN,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let opt = Opt::from_args();

    // Check for staged changes before starting the interactive CLI
    if opt.cmd.is_none() {
        if let Err(e) = git::has_staged_changes() {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        if !git::has_staged_changes().unwrap_or(false) {
            eprintln!("Error: No staged changes found\nFor help, run 'committy --help'");
            std::process::exit(1);
        }
    }

    let result = match opt.cmd {
        Some(cmd) => cmd.execute(),
        None => CliCommand::Commit(CommitCommand::default()).execute(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
