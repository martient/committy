include!(concat!(env!("OUT_DIR"), "/sentry_dsn.rs"));

mod cli;
mod config;
mod error;
mod git;
mod input;
mod linter;
mod logger;
mod release;
mod update;
mod version;

use anyhow::{anyhow, Result};
use env_logger::{Builder, Env};
use sentry::ClientInitGuard;
use structopt::StructOpt;

use crate::cli::commands::commit::CommitCommand;
use crate::cli::{CliCommand, Command};
use crate::error::CliError;
use crate::update::Updater;

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

    #[structopt(
        long = "pre-release",
        help = "Include pre-release versions when checking or updating"
    )]
    pre_release: bool,
}

#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut _guard: ClientInitGuard;

    if SENTRY_DSN != "undefined" {
        _guard = sentry::init((
            SENTRY_DSN,
            sentry::ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                ..Default::default()
            },
        ));
    }

    if let Err(e) = run().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    logger::info("Starting Committy...");

    let opt = Opt::from_args();

    if opt.check_update || opt.update {
        let mut updater = Updater::new(env!("CARGO_PKG_VERSION"))?;
        updater.with_prerelease(opt.pre_release);

        if opt.check_update {
            logger::info("Checking for updates...");
            match updater.check_update().await? {
                Some(release) => {
                    logger::info(&format!(
                        "New version {} is available! Run with --update to upgrade",
                        release.version
                    ));
                }
                None => logger::success("You're on the latest version!"),
            }
            return Ok(());
        }

        if opt.update {
            logger::info("Starting update process...");
            let update_check = updater.check_update().await?;
            match update_check {
                Some(release) => {
                    match updater.update_to_version(&format!("v{}", release.version)) {
                        Ok(_) => logger::success("Update completed successfully!"),
                        Err(e) => {
                            return Err(anyhow!(e));
                        }
                    }
                }
                None => {
                    logger::success("You're already on the latest version!");
                }
            }
            return Ok(());
        }
    }

    // Check for staged changes before starting the interactive CLI
    if opt.cmd.is_none() {
        if let Err(e) = git::has_staged_changes() {
            return Err(anyhow!(e));
        }
        if !git::has_staged_changes().unwrap_or(false) {
            return Err(anyhow!(CliError::NoStagedChanges));
        }
    }

    let result = match opt.cmd {
        Some(cmd) => cmd.execute(),
        None => {
            let cmd = CommitCommand::default();
            cmd.execute()
        }
    };

    match result {
        Ok(_) => {
            logger::success("Operation completed successfully!");
            Ok(())
        }
        Err(e) => Err(anyhow!(e)),
    }
}
