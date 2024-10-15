use crate::error::CliError;
use crate::git;
use crate::input;
use super::Command;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct AmendCommand {}

impl Command for AmendCommand {
    fn execute(&self) -> Result<(), CliError> {
        let commit_type = input::select_commit_type()?;
        let breaking_change = input::confirm_breaking_change()?;
        let scope = input::input_scope()?;
        let short_message = input::input_short_message()?;
        let long_message = input::input_long_message()?;

        let full_message = git::format_commit_message(
            &commit_type,
            breaking_change,
            &scope,
            &short_message,
            &long_message,
        );

        git::commit_changes(&full_message, true)?;

        println!("Previous commit amended successfully!");
        Ok(())
    }
}