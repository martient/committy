use inquire::validator::Validation;
use inquire::{Select, Confirm, Text};
use crate::config::{COMMIT_TYPES, MAX_SHORT_DESCRIPTION_LENGTH};
use crate::error::CliError;
use super::validation::{validate_scope, validate_short_message};

pub fn select_commit_type() -> Result<String, CliError> {
    Select::new("Select the type of commit:", COMMIT_TYPES.iter().map(|s| s.to_string()).collect())
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn confirm_breaking_change() -> Result<bool, CliError> {
    Confirm::new("Is this a breaking change?")
        .with_default(false)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn input_scope() -> Result<String, CliError> {
    Text::new("Enter the scope of the commit (optional):")
        .with_validator(|s: &str| { // Change the closure to accept a reference with a generic lifetime 'a
            validate_scope(s).map_err(|e| e.into())
                .map(|_| Validation::Valid) // Return Validation::Valid if the input is valid
        })
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn input_short_message() -> Result<String, CliError> {
    Text::new(&format!("Enter a short description (max {} characters):", MAX_SHORT_DESCRIPTION_LENGTH))
        .with_validator(|s: &str| { // Add type annotation for the closure parameter
            validate_short_message(s).map_err(|e| e.into())
                .map(|_| Validation::Valid) // Return Validation::Valid if the input is valid
        })
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn input_long_message() -> Result<String, CliError> {
    Text::new("Enter a detailed description (optional):")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn ask_want_create_new_tag() -> Result<bool, CliError> {
    Confirm::new("Are you sure you want to create a new tag?")
        .with_default(false)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}