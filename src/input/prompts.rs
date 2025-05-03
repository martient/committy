use super::validation::{
    auto_correct_scope, validate_scope, validate_section, validate_short_message,
};
use crate::config::{BRANCH_TYPES, COMMIT_TYPES, MAX_SHORT_DESCRIPTION_LENGTH};
use crate::error::CliError;
use inquire::{Confirm, Select, Text};
use log::info;

pub fn select_commit_type() -> Result<String, CliError> {
    let commit_type = Select::new("Select the type of commit:", COMMIT_TYPES.to_vec())
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    Ok(commit_type.to_string())
}

pub fn select_branch_type() -> Result<String, CliError> {
    let branch_type = Select::new("Select the type of branch:", BRANCH_TYPES.to_vec())
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    Ok(branch_type.to_string())
}

pub fn confirm_breaking_change() -> Result<bool, CliError> {
    Confirm::new("Is this a breaking change?")
        .with_default(false)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn ask_want_create_new_branch(branch_name: &str) -> Result<bool, CliError> {
    Confirm::new(&format!(
        "Are you sure you want to create a new branch {}?",
        branch_name
    ))
    .with_default(false)
    .prompt()
    .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn input_ticket() -> Result<String, CliError> {
    let ticket = Text::new("Enter the ticket identifier (optional):")
        .with_help_message("Press Enter to skip")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    if ticket.is_empty() {
        Ok(ticket)
    } else {
        validate_section(&ticket).map_err(CliError::InputError)
    }
}

pub fn input_subject() -> Result<String, CliError> {
    let subject = Text::new("Enter the subject")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;
    if subject.is_empty() {
        input_subject()
    } else {
        validate_section(&subject).map_err(CliError::InputError)
    }
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
        validate_section(&scope).map_err(CliError::InputError)
    }
}

pub fn input_short_message() -> Result<String, CliError> {
    loop {
        let validator = |input: &str| {
            let len = input.len();
            let remaining = MAX_SHORT_DESCRIPTION_LENGTH.saturating_sub(len);
            if len < 5 {
                return Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom(format!(
                        "Description must be at least 5 characters ({} more needed)",
                        5 - len
                    )),
                ));
            }
            if len > MAX_SHORT_DESCRIPTION_LENGTH {
                return Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom(format!(
                        "Description must be at most {} characters ({} over)",
                        MAX_SHORT_DESCRIPTION_LENGTH,
                        len - MAX_SHORT_DESCRIPTION_LENGTH
                    )),
                ));
            }
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
            .with_help_message(&format!(
                "Min 5, Max {} characters",
                MAX_SHORT_DESCRIPTION_LENGTH
            ))
            .with_validator(validator)
            .prompt();

        match msg {
            Ok(valid_msg) => {
                if valid_msg.trim().is_empty() {
                    // Empty input, re-prompt
                    println!("Short description cannot be empty.");
                    continue;
                } else {
                    return Ok(valid_msg);
                }
            }
            Err(inquire::error::InquireError::OperationCanceled)
            | Err(inquire::error::InquireError::OperationInterrupted) => {
                // User cancelled the prompt, stop execution and return an error
                return Err(CliError::InputError(
                    "Operation cancelled by user".to_string(),
                ));
            }
            Err(_) => {
                // Any other error, re-prompt
                println!(
                    "Please enter a valid short description (min 5, max {} chars).",
                    MAX_SHORT_DESCRIPTION_LENGTH
                );
                continue;
            }
        }
    }
}

pub fn input_long_message() -> Result<String, CliError> {
    let msg = Text::new("Enter a detailed description (optional):")
        .with_help_message("Press Enter twice to finish")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;
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
