use super::validation::{
    auto_correct_scope, validate_scope, validate_section, validate_short_message,
};
use crate::config::{
    BRANCH_TYPES, COMMIT_TYPES, MAX_SCOPE_NAME_LENGTH, MAX_SHORT_DESCRIPTION_LENGTH,
    MAX_TICKET_NAME_LENGTH,
};
use crate::error::CliError;
use inquire::{Confirm, Select, Text};
use log::info;

fn non_interactive_env() -> bool {
    std::env::var("COMMITTY_NONINTERACTIVE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::var("CI")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
}

pub fn select_commit_type() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot prompt for commit type".to_string(),
        ));
    }
    let commit_type = Select::new("Select the type of commit:", COMMIT_TYPES.to_vec())
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    Ok(commit_type.to_string())
}

pub fn select_branch_type() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot prompt for branch type".to_string(),
        ));
    }
    let branch_type = Select::new("Select the type of branch:", BRANCH_TYPES.to_vec())
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    Ok(branch_type.to_string())
}

pub fn confirm_breaking_change() -> Result<bool, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot confirm breaking change".to_string(),
        ));
    }
    Confirm::new("Is this a breaking change?")
        .with_default(false)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn ask_want_create_new_branch(branch_name: &str) -> Result<bool, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot confirm branch creation".to_string(),
        ));
    }
    Confirm::new(&format!(
        "Are you sure you want to create a new branch {branch_name}?"
    ))
    .with_default(false)
    .prompt()
    .map_err(|e| CliError::InputError(e.to_string()))
}

pub fn input_ticket() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot input ticket".to_string(),
        ));
    }
    let validator = move |input: &str| {
        let len = input.len();
        if len > MAX_TICKET_NAME_LENGTH {
            return Ok(inquire::validator::Validation::Invalid(
                inquire::validator::ErrorMessage::Custom({
                    let over = len - MAX_TICKET_NAME_LENGTH;
                    format!(
                        "Ticket identifier must be at most {MAX_TICKET_NAME_LENGTH} characters ({over} over)"
                    )
                }),
            ));
        }
        Ok(inquire::validator::Validation::Valid)
    };
    let ticket = Text::new("Enter the ticket identifier (optional):")
        .with_help_message(&format!(
            "Press Enter to skip, max {MAX_TICKET_NAME_LENGTH} characters"
        ))
        .with_validator(validator)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    if ticket.is_empty() {
        Ok(ticket)
    } else {
        validate_section(&ticket).map_err(CliError::InputError)
    }
}

pub fn input_subject() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot input subject".to_string(),
        ));
    }
    let subject = Text::new("Enter the subject:")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;
    if subject.is_empty() {
        input_subject()
    } else {
        validate_section(&subject).map_err(CliError::InputError)
    }
}

pub fn validate_scope_input(scope: &str) -> Result<String, CliError> {
    // Compute a suggested correction but do not fail early on validation errors.
    // This ensures interactive users can choose to apply the correction.
    let corrected = auto_correct_scope(scope);
    if corrected != scope {
        info!("Suggested correction: '{scope}' -> '{corrected}'");
        if non_interactive_env() {
            // In non-interactive environments, apply the correction automatically
            info!("Applied correction (non-interactive): '{corrected}'");
            return Ok(corrected);
        }
        if Confirm::new("Do you want to apply this correction?")
            .with_default(true)
            .prompt()
            .map_err(|e| CliError::InputError(e.to_string()))?
        {
            info!("Applied correction: '{corrected}'");
            Ok(corrected)
        } else {
            info!("Keeping original: '{scope}'");
            // If the original value does not pass validation, return a friendly error
            match validate_scope(scope) {
                Ok(_) => Ok(scope.to_string()),
                Err(msg) => Err(CliError::InputError(msg)),
            }
        }
    } else {
        // No correction needed; still validate to ensure it meets constraints
        validate_scope(scope)
            .map(|_| scope.to_string())
            .map_err(CliError::InputError)
    }
}

pub fn input_scope() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot input scope".to_string(),
        ));
    }
    let validator = |input: &str| {
        let len = input.len();
        if len > MAX_SCOPE_NAME_LENGTH {
            return Ok(inquire::validator::Validation::Invalid(
                inquire::validator::ErrorMessage::Custom({
                    let over = len - MAX_SCOPE_NAME_LENGTH;
                    format!(
                        "Scope must be at most {MAX_SCOPE_NAME_LENGTH} characters ({over} over)"
                    )
                }),
            ));
        }
        Ok(inquire::validator::Validation::Valid)
    };
    let scope = Text::new("Enter the scope of the commit (optional):")
        .with_help_message(&format!(
            "Press Enter to skip, max {MAX_SCOPE_NAME_LENGTH} characters"
        ))
        .with_validator(validator)
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;

    if scope.is_empty() {
        Ok(scope)
    } else {
        validate_section(&scope).map_err(CliError::InputError)
    }
}

pub fn input_short_message() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot input short message".to_string(),
        ));
    }
    loop {
        let validator = |input: &str| {
            let len = input.len();
            let remaining = MAX_SHORT_DESCRIPTION_LENGTH.saturating_sub(len);
            if len < 5 {
                return Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom({
                        let needed = 5 - len;
                        format!("Description must be at least 5 characters ({needed} more needed)")
                    }),
                ));
            }
            if len > MAX_SHORT_DESCRIPTION_LENGTH {
                return Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom({
                        let over = len - MAX_SHORT_DESCRIPTION_LENGTH;
                        format!(
                            "Description must be at most {MAX_SHORT_DESCRIPTION_LENGTH} characters ({over} over)"
                        )
                    }),
                ));
            }
            match validate_short_message(input) {
                Ok(_) => Ok(inquire::validator::Validation::Valid),
                Err(msg) => Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom(format!(
                        "{msg} ({remaining} chars remaining)"
                    )),
                )),
            }
        };

        let msg = Text::new("Enter a short description:")
            .with_help_message(&format!(
                "Min 5, Max {MAX_SHORT_DESCRIPTION_LENGTH} characters"
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
                    "Please enter a valid short description (min 5, max {MAX_SHORT_DESCRIPTION_LENGTH} chars)."
                );
                continue;
            }
        }
    }
}

pub fn input_long_message() -> Result<String, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot input long message".to_string(),
        ));
    }
    let msg = Text::new("Enter a detailed description (optional):")
        .with_help_message("Press Enter twice to finish")
        .prompt()
        .map_err(|e| CliError::InputError(e.to_string()))?;
    Ok(msg)
}

pub fn ask_want_create_new_tag() -> Result<bool, CliError> {
    if non_interactive_env() {
        return Err(CliError::InputError(
            "Non-interactive environment: cannot confirm tag creation".to_string(),
        ));
    }
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
        // Force non-interactive environment so prompt returns an error
        std::env::set_var("COMMITTY_NONINTERACTIVE", "1");
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
