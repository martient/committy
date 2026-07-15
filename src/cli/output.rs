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
    command: &'static str,
    ok: bool,
    dry_run: bool,
    errors: [MachineError<'a>; 1],
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
