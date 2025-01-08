use colored::*;
use log::{error, info, warn};

/// Print a success message with a green checkmark
#[allow(dead_code)]
pub fn success(msg: &str) {
    info!("{} {}", "✓".green(), msg);
}

/// Print an info message with a blue info symbol
pub fn info(msg: &str) {
    info!("{} {}", "ℹ".blue(), msg);
}

/// Print a warning message with a yellow warning symbol
#[allow(dead_code)]
pub fn warning(msg: &str) {
    warn!("{} {}", "⚠".yellow(), msg);
}

/// Print an error message with a red X
#[allow(dead_code)]
pub fn error(msg: &str) {
    error!("{} {}", "✗".red(), msg);
}

/// Print a progress message with a blue arrow
#[allow(dead_code)]
pub fn progress(msg: &str) {
    info!("{} {}", "→".blue(), msg);
}

#[allow(dead_code)]
/// Print a completion message with a green checkmark
pub fn done(msg: &str) {
    info!("{} {}", "✓".green(), msg);
}
