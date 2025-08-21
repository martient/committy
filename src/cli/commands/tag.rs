use std::collections::HashMap;

use crate::cli::Command;
use crate::error::CliError;
use crate::git;
use crate::input;
use crate::telemetry;
use log::debug;
use log::info;
use serde_json::Value;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct TagCommand {
    #[structopt(short, long, help = "Provide a tag name")]
    name: Option<String>,

    #[structopt(short = "y", long, help = "Want to create a new version (y/N)")]
    validate: bool,

    #[structopt(
        short = "bfs",
        long = "bump-files",
        help = "Want to auto bump the config to the new version (y/N)"
    )]
    bump_config_files: bool,

    #[structopt(flatten)]
    tag_options: git::TagGeneratorOptions,

    /// Output format: text or json
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,
}

impl Command for TagCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        if git::has_staged_changes()? {
            return Err(CliError::StagedChanges);
        }

        if let Some(name) = &self.name {
            let version_manager =
                git::TagGenerator::new(self.tag_options.clone(), self.bump_config_files);
            version_manager.create_and_push_tag(&version_manager.open_repository()?, name)?;
            if self.output == "json" {
                let payload = serde_json::json!({
                    "ok": true,
                    "new_tag": name,
                });
                println!("{}", serde_json::to_string(&payload).unwrap());
            } else {
                println!("Tag {name} created successfully!");
            }
        } else if non_interactive {
            // In non-interactive mode, auto-calculate and act based on options
            let mut version_manager =
                git::TagGenerator::new(self.tag_options.clone(), self.bump_config_files);
            version_manager.run()?;

            // Print the calculated tag so callers/tests can consume it
            if self.output == "json" {
                let payload = serde_json::json!({
                    "ok": true,
                    "old_tag": version_manager.current_tag,
                    "new_tag": version_manager.new_tag,
                    "pre_release": version_manager.is_pre_release,
                });
                println!("{}", serde_json::to_string(&payload).unwrap());
            } else {
                println!("{}", version_manager.new_tag);
            }
        } else {
            let validate = if !self.validate {
                input::ask_want_create_new_tag()?
            } else {
                true
            };
            if !validate {
                info!("Abort");
                return Ok(());
            }
            let mut version_manager =
                git::TagGenerator::new(self.tag_options.clone(), self.bump_config_files);
            version_manager.run()?;
            if self.output == "json" {
                let payload = serde_json::json!({
                    "ok": true,
                    "old_tag": version_manager.current_tag,
                    "new_tag": version_manager.new_tag,
                    "pre_release": version_manager.is_pre_release,
                });
                println!("{}", serde_json::to_string(&payload).unwrap());
            }
            if let Err(e) =
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(telemetry::posthog::publish_event(
                        "tag_created",
                        HashMap::from([
                            ("old_tag", Value::from(version_manager.current_tag)),
                            ("new_tag", Value::from(version_manager.new_tag)),
                            (
                                "is_pre_release",
                                Value::from(version_manager.is_pre_release),
                            ),
                            ("allow_bump_files", Value::from(self.bump_config_files)),
                        ]),
                    ))
            {
                debug!("Telemetry error: {e:?}");
            }
        }

        Ok(())
    }
}
