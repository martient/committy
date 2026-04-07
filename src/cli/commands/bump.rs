use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
use crate::config::repository::BumpType;
use crate::error::CliError;
use crate::release::ReleaseEngine;

#[derive(Debug, StructOpt)]
pub struct BumpCommand {
    #[structopt(
        long,
        help = "Preview the release without mutating git or version files"
    )]
    dry_run: bool,

    #[structopt(long, help = "Override the calculated increment", possible_values = &["major", "minor", "patch"])]
    increment: Option<String>,

    #[structopt(long, help = "Create or continue a prerelease")]
    prerelease: bool,

    #[structopt(long, help = "Publish the release commit and tag(s)")]
    publish: bool,

    #[structopt(long, help = "Confirm publishing the release commit and tag(s)")]
    confirm_publish: bool,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long = "from-ref")]
    from_ref: Option<String>,

    #[structopt(long = "to-ref")]
    to_ref: Option<String>,

    #[structopt(
        long = "git-config",
        help = "Pass through git -c key=value overrides (repeatable)"
    )]
    git_config: Vec<String>,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct BumpOutput<'a> {
    command: &'static str,
    ok: bool,
    dry_run: bool,
    plan: &'a crate::release::BumpPlan,
    errors: Option<Vec<String>>,
}

impl Command for BumpCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &self.git_config)?;
        let plan = engine.plan_bump(
            self.increment.as_deref().map(parse_bump).transpose()?,
            self.prerelease,
            self.dry_run,
            self.publish,
            self.from_ref.as_deref(),
            self.to_ref.as_deref(),
        )?;

        if self.dry_run {
            self.print(&plan)?;
            return Ok(());
        }

        engine.apply_bump(&plan, self.confirm_publish)?;
        self.print(&plan)?;
        Ok(())
    }
}

impl BumpCommand {
    fn print(&self, plan: &crate::release::BumpPlan) -> Result<(), CliError> {
        if self.output == "json" {
            let payload = BumpOutput {
                command: "bump",
                ok: true,
                dry_run: self.dry_run,
                plan,
                errors: None,
            };
            println!(
                "{}",
                serde_json::to_string(&payload).map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            println!(
                "provider={} bump={} next={} tags={}",
                plan.provider,
                plan.bump,
                plan.next_version.as_deref().unwrap_or("n/a"),
                plan.tag_names.join(", ")
            );
        }
        Ok(())
    }
}

fn parse_bump(value: &str) -> Result<BumpType, CliError> {
    match value {
        "major" => Ok(BumpType::Major),
        "minor" => Ok(BumpType::Minor),
        "patch" => Ok(BumpType::Patch),
        other => Err(CliError::InputError(format!(
            "Unsupported increment '{other}'"
        ))),
    }
}
