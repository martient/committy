use super::validation::{
    auto_correct_scope, validate_scope, validate_short_message,
};
use crate::config::{COMMIT_TYPES, MAX_SHORT_DESCRIPTION_LENGTH};
use crate::error::CliError;
use inquire::validator::Validation;
use inquire::{Confirm, Select, Text};

pub fn select_commit_type() -> Result<String, CliError> {
    let commit_type = Select::new("Select the type of commit:", COMMIT_TYPES.to_vec())
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

pub fn input_scope() -> Result<String, CliError> {
    let scope = Text::new("Enter the scope of the commit (optional):")
        .with_validator(|s: &str| {
            validate_scope(s)
                .map_err(|e| e.into())
                .map(|_| Validation::Valid)
        })
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    if !scope.is_empty() {
        let corrected = auto_correct_scope(&scope);
        if corrected != scope {
            println!("Auto-correcting scope from '{}' to '{}'", scope, corrected);
            Ok(corrected)
        } else {
            Ok(scope)
        }
    } else {
        Ok(scope)
    }
}

pub fn input_short_message() -> Result<String, CliError> {
    Text::new(&format!(
        "Enter a short description (max {} characters):",
        MAX_SHORT_DESCRIPTION_LENGTH
    ))
    .with_validator(|s: &str| {
        validate_short_message(s)
            .map_err(|e| e.into())
            .map(|_| Validation::Valid)
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
        assert_eq!(result, "userservice");

        // Test valid scope
        let scope = "user-service";
        let result = auto_correct_scope(scope);
        assert_eq!(result, "user-service");
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
