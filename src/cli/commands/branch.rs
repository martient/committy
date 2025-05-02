use crate::cli::Command;
use crate::error::CliError;
use structopt::StructOpt;
use crate::git;
use log::info;
use crate::input;

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

        if git::has_staged_changes()? {
            return Err(CliError::StagedChanges);
        }

        if let Some(name) = &self.name {
            git::create_branch(name, self.force)?;
            println!("Branch {} created successfully!", name);
        } else {
            if non_interactive {
                return Err(CliError::InputError(
                    "Branch name is required in non-interactive mode".to_string(),
                ));
            }

            let type_ = input::select_commit_type()?;
            let ticket = input::input_ticket()?;
            let subject = input::input_subject()?;

            let branch_name = if ticket.is_empty() {
                format!("{}-{}", type_, subject)
            } else {
                format!("{}-{}-{}", type_, ticket, subject)
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
        }

        Ok(())
    }
}