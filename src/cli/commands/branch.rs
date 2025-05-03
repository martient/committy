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

#[derive(Debug, StructOpt, Default)]
pub struct BranchCommand {
    #[structopt(short, long, help = "Name of the branch to create")]
    name: Option<String>,

    #[structopt(short, long, help = "Force create branch")]
    force: bool,

    #[structopt(short, long, help = "Validate branch name")]
    validate: bool,
}

impl Command for BranchCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        git::validate_git_config()?;

        if let Some(name) = &self.name {
            git::create_branch(name, self.force)?;
            println!("Branch {} created successfully!", name);
        } else {
            if non_interactive {
                return Err(CliError::InputError(
                    "Branch name is required in non-interactive mode".to_string(),
                ));
            }

            let branch_type = input::select_branch_type()?;
            let ticket = input::input_ticket()?;
            let subject = input::input_subject()?;

            let branch_name = if ticket.is_empty() {
                format!("{}-{}", branch_type, subject)
            } else {
                format!("{}-{}-{}", branch_type, ticket, subject)
            };

            let validate = if !self.validate {
                input::ask_want_create_new_branch(&branch_name)?
            } else {
                true
            };
            if !validate {
                info!("Abort");
                return Ok(());
            }
            git::create_branch(&branch_name, self.force)?;
            println!("Branch {} created successfully!", branch_name);
            git::checkout_branch(&branch_name)?;
            println!("Switched to branch {}", branch_name);
            if let Err(e) =
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(telemetry::posthog::publish_event(
                        "branch_created",
                        HashMap::from([
                            ("branch_type", Value::from(branch_type.as_str())),
                            ("as_ticket", Value::from((!ticket.is_empty()).to_string())),
                            ("len_ticket", Value::from(ticket.len())),
                            ("as_subject", Value::from((!subject.is_empty()).to_string())),
                            ("len_subject", Value::from(subject.len())),
                        ]),
                    ))
            {
                debug!("Telemetry error: {:?}", e);
            }
        }

        Ok(())
    }
}
