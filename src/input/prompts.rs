use super::validation::{auto_correct_scope, validate_scope, validate_short_message};
use crate::config::{COMMIT_TYPES, MAX_SHORT_DESCRIPTION_LENGTH};
use crate::error::CliError;
use crate::logger;
use inquire::{Confirm, Select, Text};
use log::info;

pub fn select_commit_type() -> Result<String, CliError> {
    let commit_type = Select::new("Select the type of commit:", COMMIT_TYPES.to_vec())
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    Ok(commit_type.to_string())
}

pub fn confirm_breaking_change() -> Result<bool, CliError> {
    Confirm::new("Is this a breaking change?")
        .with_default(false)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn validate_scope_input(scope: &str) -> Result<String, CliError> {
    // First validate the scope
    validate_scope(scope).map_err(CliError::InputError)?;

    let corrected = auto_correct_scope(scope);
    if corrected != scope {
        info!("Suggested correction: '{}' -> '{}'", scope, corrected);
        if Confirm::new("Do you want to apply this correction?")
            .with_default(true)
            .prompt()
            .map_err(|e| CliError::InputError(e.to_string()))?
        {
            info!("Applied correction: '{}'", corrected);
            Ok(corrected)
        } else {
            info!("Keeping original: '{}'", scope);
            Ok(scope.to_string())
        }
    } else {
        Ok(scope.to_string())
    }
}

pub fn input_scope() -> Result<String, CliError> {
    let scope = Text::new("Enter the scope of the commit (optional):")
        .with_help_message("Press Enter to skip")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    if scope.is_empty() {
        Ok(scope)
    } else {
        validate_scope_input(&scope)
    }
}

pub fn input_short_message() -> Result<String, CliError> {
    let validator = move |input: &str| {
        let remaining = MAX_SHORT_DESCRIPTION_LENGTH - input.len();
        match validate_short_message(input) {
            Ok(_) => Ok(inquire::validator::Validation::Valid),
            Err(msg) => Ok(inquire::validator::Validation::Invalid(
                inquire::validator::ErrorMessage::Custom(format!(
                    "{} ({} chars remaining)",
                    msg, remaining
                )),
            )),
        }
    };

    let msg = Text::new("Enter a short description:")
        .with_help_message(&format!("Max {} characters", MAX_SHORT_DESCRIPTION_LENGTH))
        .with_validator(validator)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    logger::success("Short description validated");
    Ok(msg)
}

pub fn input_long_message() -> Result<String, CliError> {
    let msg = Text::new("Enter a detailed description (optional):")
        .with_help_message("Press Enter twice to finish")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    if !msg.is_empty() {
        logger::info("Added detailed description");
    }
    Ok(msg)
}

pub fn ask_want_create_new_tag() -> Result<bool, CliError> {
    Confirm::new("Are you sure you want to create a new tag?")
        .with_default(false)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_commit_type() {
        // Since we can't easily test interactive selection in unit tests,
        // we'll just verify that the function exists and returns an error
        // when run in a non-interactive environment
        let result = select_commit_type();
        assert!(matches!(result, Err(CliError::InputError(_))));
    }

    #[test]
    fn test_input_scope() {
        // Test auto-correction of invalid characters
        let scope = "user@service";
        let result = auto_correct_scope(scope);
        assert_eq!(result, "user-service");

        // Test valid scope
        let scope = "user-service";
        let result = auto_correct_scope(scope);
        assert_eq!(result, "user-service");

        // Test empty scope
        let scope = "";
        let result = auto_correct_scope(scope);
        assert_eq!(result, "");

        // Test whitespace scope
        let scope = "user service";
        let result = auto_correct_scope(scope);
        assert_eq!(result, "user-service");

        // Test whitespace scope with trimming
        let scope = "  user service  ";
        let result = auto_correct_scope(scope);
        assert_eq!(result, "user-service");

        // Note: We can't directly test the interactive confirmation here
        // as it requires user input. The integration tests will handle this
        // using the --non-interactive flag.
    }

    #[test]
    fn test_input_short_message() {
        // Test valid message
        let msg = "Add user authentication";
        assert!(validate_short_message(msg).is_ok());

        // Test message too long
        let long_msg = "a".repeat(MAX_SHORT_DESCRIPTION_LENGTH + 1);
        assert!(validate_short_message(&long_msg).is_err());
    }
}
