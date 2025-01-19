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

use anyhow::Result;
use env_logger::{Builder, Env};
use sentry::ClientInitGuard;
use structopt::StructOpt;

use crate::cli::commands::commit::CommitCommand;
use crate::cli::{CliCommand, Command};
use crate::config::Config;
use crate::error::CliError;
use crate::update::Updater;
use chrono::{DateTime, Duration};

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

    #[structopt(long = "non-interactive", help = "Run in non-interactive mode")]
    non_interactive: bool,

    #[structopt(long = "metrics-toggle", help = "Toggle metrics collection on/off")]
    metrics_toggle: bool,
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    // Load configuration
    let mut config = Config::load().unwrap_or_else(|_| {
        let default_config = Config::default();
        if let Err(e) = default_config.save() {
            eprintln!("Failed to save default configuration: {}", e);
        }
        default_config
    });

    if let Err(e) = run(&mut config) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run(config: &mut Config) -> Result<()> {
    let opt = Opt::from_args();

    if opt.metrics_toggle {
        config.metrics_enabled = !config.metrics_enabled;
        logger::info(&format!(
            "Metrics collection has been {} ",
            if config.metrics_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
        config.save()?;
        return Ok(());
    }

    let current_time = DateTime::parse_from_rfc3339("2025-01-08T17:49:53+01:00").unwrap();
    let one_week = Duration::days(7);
    let one_day = Duration::days(1);
    let mut config_updated = false;

    // Show metrics reminder if enabled and it's been a week
    if config.metrics_enabled && current_time - config.last_metrics_reminder >= one_week {
        logger::info(
            " Metrics collection is enabled to help improve Committy. You can opt-out anytime with --metrics-toggle",
        );
        config.last_metrics_reminder = current_time;
        config_updated = true;
    }

    // Initialize sentry if metrics are enabled
    let _guard: Option<ClientInitGuard> = if config.metrics_enabled && SENTRY_DSN != "undefined" {
        Some(sentry::init((
            SENTRY_DSN,
            sentry::ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                ..Default::default()
            },
        )))
    } else {
        None
    };

    if opt.check_update || opt.update {
        let mut updater = Updater::new(env!("CARGO_PKG_VERSION"))?;
        updater
            .with_prerelease(opt.pre_release)
            .with_non_interactive(opt.non_interactive);

        if opt.update && (updater.check_and_prompt_update()).is_ok() {
            config.last_update_check = current_time;
            config_updated = true;
        } else {
            logger::info("You're running the latest version!");
            config.last_update_check = current_time;
            config_updated = true;
        }
    }

    // Check for updates when running any command
    if !opt.non_interactive
        && !opt.check_update
        && !opt.update
        && current_time - config.last_update_check >= one_day
    {
        let mut updater = Updater::new(env!("CARGO_PKG_VERSION"))?;
        updater.with_prerelease(Updater::is_prerelease(&updater.current_version.to_string()));
        if (updater.check_and_prompt_update()).is_ok() {
            // if let Some(_) = updater.check_and_prompt_update().await? {
            config.last_update_check = current_time;
            config_updated = true;
        }
    }

    // Save config only if it was updated
    if config_updated {
        config.save()?;
    }

    // Check for staged changes before starting the interactive CLI
    if opt.cmd.is_none() {
        if let Err(e) = git::has_staged_changes() {
            return Err(e.into());
        }
        if !git::has_staged_changes().unwrap_or(false) {
            return Err(CliError::NoStagedChanges.into());
        }
    }

    let result = match opt.cmd {
        Some(cmd) => cmd.execute(opt.non_interactive),
        None => {
            let cmd = CommitCommand::default();
            cmd.execute(opt.non_interactive)
        }
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
