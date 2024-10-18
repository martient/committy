use crate::error::CliError;
use crate::git;
use crate::input;
use super::Command;
use log::info;
use structopt::StructOpt;

#[derive(StructOpt, Default)]
pub struct CommitCommand {
    #[structopt(short, long, help = "Provide a short commit message")]
    short_message: Option<String>,
}

impl Command for CommitCommand {
    fn execute(&self) -> Result<(), CliError> {
        if !git::has_staged_changes()? {
            return Err(CliError::NoStagedChanges);
        }

        let commit_type = input::select_commit_type()?;
        let breaking_change = input::confirm_breaking_change()?;
        let scope = input::input_scope()?;
        let short_message = match &self.short_message {
            Some(msg) if !msg.is_empty() => msg.clone(),
            _ => input::input_short_message()?,
        };
        let long_message = input::input_long_message()?;

        let full_message = git::format_commit_message(
            &commit_type,
            breaking_change,
            &scope,
            &short_message,
            &long_message,
        );

        git::commit_changes(&full_message, false)?;

        info!("Changes committed successfully! 🎉");
        Ok(())
    }
}