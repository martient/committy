use crate::cli::Command;
use crate::error::CliError;
use crate::git;
use crate::input;
use log::info;
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
}

impl Command for TagCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        if git::has_staged_changes()? {
            return Err(CliError::StagedChanges);
        }

        if let Some(name) = &self.name {
            info!("Tag {} created successfully!", name);
        } else {
            if non_interactive {
                return Err(CliError::InputError(
                    "Tag name is required in non-interactive mode".to_string(),
                ));
            }

            let validate = if !self.validate {
                input::ask_want_create_new_tag()?
            } else {
                true
            };
            if !validate {
                info!("Abort");
                return Ok(());
            }
            let version_manager =
                git::TagGenerator::new(self.tag_options.clone(), self.bump_config_files);
            version_manager.run()?;
        }

        Ok(())
    }
}
