pub mod app;
pub mod event;
pub mod state;
pub mod ui;

pub use app::App;
pub use event::{Event, EventHandler};
pub use state::AppState;

use crate::error::CliError;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Initialize the terminal
pub fn init() -> Result<Tui, CliError> {
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| CliError::Generic(format!("Failed to enter alternate screen: {}", e)))?;
    enable_raw_mode()
        .map_err(|e| CliError::Generic(format!("Failed to enable raw mode: {}", e)))?;

    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)
        .map_err(|e| CliError::Generic(format!("Failed to create terminal: {}", e)))?;

    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore() -> Result<(), CliError> {
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .map_err(|e| CliError::Generic(format!("Failed to leave alternate screen: {}", e)))?;
    disable_raw_mode()
        .map_err(|e| CliError::Generic(format!("Failed to disable raw mode: {}", e)))?;
    Ok(())
}