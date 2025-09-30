use crate::cli::Command;
use crate::error::CliError;
use crate::tui::{self, App};
use log::info;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct TuiCommand {
    #[structopt(long = "ai", help = "Enable AI assistance for commit messages")]
    ai: bool,

    #[structopt(long = "ai-provider", default_value = "openrouter", possible_values = &["openrouter", "ollama"])]
    ai_provider: String,

    #[structopt(long = "ai-model", help = "AI model to use")]
    ai_model: Option<String>,
}

impl Command for TuiCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        info!("🚀 Starting Committy TUI");

        // Initialize terminal
        let mut terminal = tui::init()?;

        // Create app
        let mut app = App::new()?;

        // Set AI options if enabled
        if self.ai {
            app.state.ai_enabled = true;
        }

        // Run the TUI
        let result = app.run(&mut terminal);

        // Restore terminal
        tui::restore()?;

        result
    }
}