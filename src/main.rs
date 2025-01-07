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

use env_logger::{Builder, Env};
use sentry::ClientInitGuard;
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

    #[structopt(
        long = "pre-release",
        help = "Include pre-release versions when checking or updating"
    )]
    pre_release: bool,
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
        let mut updater = update::Updater::new(env!("CARGO_PKG_VERSION"))?;
        updater.with_prerelease(opt.pre_release);
        if opt.check_update {
            if let Some(release) = updater.check_update().await? {
                let pre_release_suffix = if update::Updater::is_prerelease(&release.version) {
                    " (pre-release)"
                } else {
                    ""
                };
                println!(
                    "New version {}{} available!",
                    release.version, pre_release_suffix
                );
                println!(
                    "Run 'committy --update{}' to update to this version",
                    if opt.pre_release {
                        " --pre-release"
                    } else {
                        ""
                    }
                );
            } else {
                println!(
                    "You are running the latest{}version!",
                    if opt.pre_release {
                        " (including pre-release) "
                    } else {
                        " "
                    }
                );
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
