include!(concat!(env!("OUT_DIR"), "/sentry_dsn.rs"));

mod ai;
mod cli;
mod clock;
mod config;
mod convention;
mod dependency;
mod error;
mod git;
mod input;
mod linter;
mod logger;
mod packages;
mod release;
mod scope;
mod telemetry;
mod update;
mod version;
mod versioning;
mod workflow;

use anyhow::Result;
use env_logger::{Builder, Env};
use log::LevelFilter;
use sentry::ClientInitGuard;
use structopt::StructOpt;

use crate::cli::commands::commit::CommitCommand;
use crate::cli::{CliCommand, Command};
use crate::clock::{current_time, should_check_update, should_remind_metrics};
use crate::config::Config;
use crate::error::CliError;
use crate::update::Updater;
use chrono::DateTime;

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

    #[structopt(
        long = "non-interactive",
        global = true,
        help = "Run in non-interactive mode"
    )]
    non_interactive: bool,

    #[structopt(long = "metrics-toggle", help = "Toggle metrics collection on/off")]
    metrics_toggle: bool,

    // Not global: `-v` is already taken by `branch --validate` and `--verbose`
    // by `config validate|show`. Promoting it would silently steal those
    // shorts. Agents use `-q` and `--non-interactive`, which are global below.
    #[structopt(
        short = "v",
        long = "verbose",
        parse(from_occurrences),
        help = "Increase verbosity (-v, -vv); must precede the subcommand"
    )]
    verbose: u8,

    #[structopt(
        short = "q",
        long = "quiet",
        global = true,
        help = "Reduce verbosity (errors only)"
    )]
    quiet: bool,
}

/// Subcommand names as they appear on the command line.
///
/// Used only to attribute an argv rejection to a command, since clap gives us
/// no parsed value in that case.
const SUBCOMMAND_NAMES: &[&str] = &[
    "commit",
    "amend",
    "tag",
    "bump",
    "changelog",
    "lint",
    "lint-message",
    "example",
    "info",
    "ls",
    "schema",
    "version",
    "branch",
    "group-commit",
    "hooks",
    "init",
    "config",
    "packages",
];

/// Whether the caller asked for machine output, determined from raw argv.
///
/// An argv rejection happens before any flag is parsed, so this is the only way
/// to know whether the caller is an agent expecting one JSON document.
fn requested_json_output(args: &[String]) -> bool {
    args.iter().enumerate().any(|(index, arg)| {
        arg == "--output=json"
            || (arg == "--output" && args.get(index + 1).map(String::as_str) == Some("json"))
    })
}

/// Best-effort command attribution for an argv rejection.
fn command_from_args(args: &[String]) -> &str {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(String::as_str)
        .filter(|candidate| SUBCOMMAND_NAMES.contains(candidate))
        .unwrap_or("unknown")
}

fn main() {
    // Parse argv before touching configuration so a malformed invocation is
    // rejected without side effects.
    let args: Vec<String> = std::env::args().collect();
    let opt = match Opt::from_iter_safe(&args) {
        Ok(opt) => opt,
        Err(error) => exit_on_parse_error(error, &args),
    };

    // Load configuration
    let mut config = Config::load().unwrap_or_else(|_| {
        let default_config = Config::default();
        if let Err(e) = default_config.save() {
            eprintln!("Failed to save default configuration: {e}");
        }
        default_config
    });

    if let Err(e) = run(&mut config, opt) {
        eprintln!("{e}");
        let exit_code = e
            .downcast_ref::<CliError>()
            .map(CliError::exit_code)
            .unwrap_or(1);
        std::process::exit(exit_code);
    }
}

/// Report a clap rejection and terminate.
///
/// `--help` and `--version` arrive here as errors too; they are successful
/// output and keep their plain-text form. A genuine rejection honours
/// `--output json` so that an agent's parser sees the same envelope it sees for
/// every other failure, instead of an empty stdout and prose on stderr.
fn exit_on_parse_error(error: structopt::clap::Error, args: &[String]) -> ! {
    use structopt::clap::ErrorKind;

    if matches!(
        error.kind,
        ErrorKind::HelpDisplayed | ErrorKind::VersionDisplayed
    ) {
        println!("{}", error.message);
        std::process::exit(0);
    }

    if requested_json_output(args) {
        cli::output::print_usage_error(command_from_args(args), &error.message);
    } else {
        eprintln!("{}", error.message);
    }
    std::process::exit(1);
}

fn run(config: &mut Config, opt: Opt) -> Result<()> {
    // Initialize logger based on verbosity flags
    let mut builder = Builder::from_env(Env::default().default_filter_or("info"));
    let level = if opt.quiet {
        LevelFilter::Error
    } else {
        match opt.verbose {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    };
    builder.filter_level(level).init();

    // Unified non-interactive mode for CI/tests and CLI flag
    let env_non_interactive = std::env::var("COMMITTY_NONINTERACTIVE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::var("CI")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
    let non_interactive = opt.non_interactive || env_non_interactive;

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

    let current_time: DateTime<_> = current_time()?;
    let mut config_updated = false;

    // Show metrics reminder if enabled and it's been a week
    if config.metrics_enabled && should_remind_metrics(config.last_metrics_reminder, current_time) {
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
            .with_non_interactive(non_interactive);

        match updater.check_update() {
            Ok(Some(release)) => {
                logger::info(&format!("New version {} is available!", release.version));
                if opt.update {
                    // Auto-apply the update when --update is passed
                    updater.update_to_version(&release.version)?;
                    config.last_update_check = current_time;
                    config_updated = true;
                }
            }
            Ok(None) => {
                // No update available (covers both --check-update and --update)
                logger::info("You're running the latest version!");
                config.last_update_check = current_time;
                config_updated = true;
            }
            Err(e) => return Err(e),
        }
    }

    // Check for updates when running any command
    if !non_interactive
        && !opt.check_update
        && !opt.update
        && should_check_update(config.last_update_check, current_time)
    {
        let mut updater = Updater::new(env!("CARGO_PKG_VERSION"))?;
        updater.with_prerelease(true);
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

    // If no subcommand is provided and we're only handling update-related flags,
    // exit early to avoid falling through to the default commit flow.
    if opt.cmd.is_none() && (opt.check_update || opt.update) {
        return Ok(());
    }

    // Check for staged changes before starting the interactive CLI
    // Skip this preflight when running only top-level maintenance flags
    // like --check-update or --update with no subcommand.
    if opt.cmd.is_none() && !opt.check_update && !opt.update {
        if let Err(e) = git::has_staged_changes() {
            return Err(e.into());
        }
        if !git::has_staged_changes().unwrap_or(false) {
            return Err(CliError::NoStagedChanges.into());
        }
    }

    let result = match &opt.cmd {
        Some(cmd) => {
            let result = cmd.execute(non_interactive);
            if let Err(error) = &result {
                if !matches!(error, CliError::LintIssues(_)) {
                    if let Some(context) = cmd.machine_context() {
                        cli::output::print_error(context, error);
                    }
                }
            }
            result
        }
        None => {
            let cmd = CommitCommand::default_interactive();
            cmd.execute(non_interactive)
        }
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
