mod cli;
mod config;
mod error;
mod git;
mod input;
mod release;
mod version;

use cli::commands::{commit::CommitCommand, CliCommand};
use config::SENTRY_DSN;
use env_logger::{Builder, Env};
use structopt::StructOpt;

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
