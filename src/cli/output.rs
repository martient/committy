use crate::error::CliError;
use serde::Serialize;

pub const API_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug)]
pub struct MachineContext {
    pub command: &'static str,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
struct MachineError<'a> {
    code: &'static str,
    message: &'a str,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
    api_version: u8,
    command: &'a str,
    ok: bool,
    dry_run: bool,
    errors: [MachineError<'a>; 1],
}

/// Strip ANSI styling so the envelope carries plain text.
///
/// Clap colours its rejections, and those escape sequences would otherwise end
/// up inside a JSON string value.
fn strip_ansi(text: &str) -> String {
    static ANSI: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap());
    ANSI.replace_all(text, "").into_owned()
}

/// Emit the machine envelope for an argument-parse rejection.
///
/// `command` is attributed from raw argv and falls back to `"unknown"`, because
/// clap rejects the invocation before any value is parsed.
pub fn print_usage_error(command: &str, message: &str) {
    let message = strip_ansi(message);
    let payload = ErrorEnvelope {
        api_version: API_VERSION,
        command,
        ok: false,
        dry_run: false,
        errors: [MachineError {
            code: "invalid_usage",
            message: &message,
        }],
    };
    println!("{}", serde_json::to_string(&payload).unwrap());
}

pub fn print_error(context: MachineContext, error: &CliError) {
    let message = error.to_string();
    let payload = ErrorEnvelope {
        api_version: API_VERSION,
        command: context.command,
        ok: false,
        dry_run: context.dry_run,
        errors: [MachineError {
            code: error.code(),
            message: &message,
        }],
    };
    println!("{}", serde_json::to_string(&payload).unwrap());
}
