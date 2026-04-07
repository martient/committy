use std::path::PathBuf;

use serde::Serialize;
use structopt::StructOpt;

use crate::cli::Command;
use crate::error::CliError;
use crate::release::changelog::{collect_entries, render_changelog};
use crate::release::ReleaseEngine;

#[derive(Debug, StructOpt)]
pub struct ChangelogCommand {
    #[structopt(long)]
    dry_run: bool,

    #[structopt(long = "from-ref")]
    from_ref: Option<String>,

    #[structopt(long = "to-ref")]
    to_ref: Option<String>,

    #[structopt(long = "template")]
    template: Option<String>,

    #[structopt(long = "template-path", parse(from_os_str))]
    template_path: Option<PathBuf>,

    #[structopt(long = "output-file", parse(from_os_str))]
    output_file: Option<PathBuf>,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct ChangelogOutput<'a> {
    command: &'static str,
    ok: bool,
    dry_run: bool,
    changelog: &'a crate::release::ChangelogPlan,
    errors: Option<Vec<String>>,
}

impl Command for ChangelogCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        let engine = ReleaseEngine::load(&self.repo_path, &[])?;
        let mut plan = engine.plan_bump(
            None,
            false,
            true,
            false,
            self.from_ref.as_deref(),
            self.to_ref.as_deref(),
        )?;

        if self.template.is_some() || self.template_path.is_some() {
            let mut changelog_config = engine.changelog_config().clone();
            if let Some(template) = &self.template {
                changelog_config.template = template.clone();
            }
            if let Some(template_path) = &self.template_path {
                changelog_config.template_path = Some(template_path.display().to_string());
            }
            let (entries, previous_ref) = collect_entries(
                &self.repo_path,
                engine.convention(),
                self.from_ref.as_deref(),
                self.to_ref.as_deref(),
                &changelog_config,
            )?;
            plan.changelog.rendered = render_changelog(
                &changelog_config,
                plan.next_version.as_deref().unwrap_or("0.0.0"),
                previous_ref.as_deref(),
                &entries,
            )?;
            plan.changelog.previous_ref = previous_ref;
            plan.changelog.entry_count = entries.len();
        }

        if let Some(output_file) = &self.output_file {
            if !self.dry_run {
                crate::release::changelog::write_changelog(output_file, &plan.changelog.rendered)?;
            }
            plan.changelog.output_file = Some(output_file.display().to_string());
        } else if !self.dry_run {
            if let Some(output_file) = &plan.changelog.output_file {
                crate::release::changelog::write_changelog(
                    &self.repo_path.join(output_file),
                    &plan.changelog.rendered,
                )?;
            }
        }

        if self.output == "json" {
            let payload = ChangelogOutput {
                command: "changelog",
                ok: true,
                dry_run: self.dry_run,
                changelog: &plan.changelog,
                errors: None,
            };
            println!(
                "{}",
                serde_json::to_string(&payload).map_err(|e| CliError::Generic(e.to_string()))?
            );
        } else {
            println!("{}", plan.changelog.rendered);
        }
        Ok(())
    }
}
