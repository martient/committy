use super::validation::{
    auto_correct_scope, suggest_commit_type, validate_scope, validate_short_message,
};
use crate::config::{COMMIT_TYPES, MAX_SHORT_DESCRIPTION_LENGTH};
use crate::error::CliError;
use inquire::validator::Validation;
use inquire::{Confirm, Text};

pub fn select_commit_type() -> Result<String, CliError> {
    let commit_type = Text::new("Enter the type of commit:")
        .with_validator(|input: &str| {
            if COMMIT_TYPES.contains(&input) {
                Ok(Validation::Valid)
            } else if let Some(suggestion) = suggest_commit_type(input) {
                Ok(Validation::Invalid(
                    format!("Invalid commit type. Did you mean '{}'?", suggestion).into(),
                ))
            } else {
                Ok(Validation::Invalid(
                    format!(
                        "Invalid commit type. Valid types are: {}",
                        COMMIT_TYPES.join(", ")
                    )
                    .into(),
                ))
            }
        })
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    // Auto-correct if there's a close match
    if let Some(suggestion) = suggest_commit_type(&commit_type) {
        if suggestion != commit_type.as_str() {
            println!(
                "Auto-correcting commit type from '{}' to '{}'",
                commit_type, suggestion
            );
            Ok(suggestion.to_string())
        } else {
            Ok(commit_type)
        }
    } else {
        Ok(commit_type)
    }
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
        // Test auto-correction
        let commit_type = "  Feature  ";
        assert!(suggest_commit_type(commit_type).is_some());
        assert_eq!(suggest_commit_type(commit_type).unwrap(), "feat");

        // Test invalid input
        let commit_type = "something_completely_different";
        assert!(suggest_commit_type(commit_type).is_none());
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
