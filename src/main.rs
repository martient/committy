include!(concat!(env!("OUT_DIR"), "/sentry_dsn.rs"));

mod cli;
mod config;
mod error;
mod git;
mod input;
mod linter;
mod release;
mod update;
mod version;

use sentry::ClientInitGuard;
use env_logger::{Builder, Env};
use std::error::Error;
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

    #[structopt(long = "check-update", help = "Check for available updates")]
    check_update: bool,

    #[structopt(long = "update", help = "Update to the latest version")]
    update: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut _guard: ClientInitGuard;

    if SENTRY_DSN != "undefined" {
        _guard = sentry::init((
            SENTRY_DSN,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                ..Default::default()
            },
        ));
    }

    let opt = Opt::from_args();

    // Handle update commands
    if opt.check_update || opt.update {
        let updater = update::Updater::new(env!("CARGO_PKG_VERSION"))?;
        if opt.check_update {
            if let Some(release) = updater.check_update().await? {
                println!("New version {} available!", release.version);
                println!("Run 'committy --update' to update to the latest version");
            } else {
                println!("You are running the latest version!");
            }
            return Ok(());
        }

        if opt.update {
            updater.update_to_latest()?;
            return Ok(());
        }
    }

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
