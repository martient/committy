use crate::config::{COMMIT_TYPES, MAX_SHORT_DESCRIPTION_LENGTH};
use strsim;

pub fn validate_short_message(input: &str) -> Result<(), String> {
    if input.len() <= MAX_SHORT_DESCRIPTION_LENGTH {
        Ok(())
    } else {
        Err(format!(
            "The message must be {} characters or less",
            MAX_SHORT_DESCRIPTION_LENGTH
        ))
    }
}

pub fn suggest_commit_type(input: &str) -> Option<&'static str> {
    let input = input.trim().to_lowercase();
    COMMIT_TYPES
        .iter()
        .min_by_key(|&&valid_type| strsim::levenshtein(&input, valid_type))
        .filter(|&&valid_type| {
            let distance = strsim::levenshtein(&input, valid_type);
            let max_allowed = (valid_type.len() as f32 * 0.6).ceil() as usize;
            distance <= max_allowed
        })
        .copied()
}

pub fn auto_correct_scope(input: &str) -> String {
    input
        .chars()
        .filter(|&c| c.is_ascii_alphanumeric() || c == '-')
        .collect()
}

pub fn validate_scope(input: &str) -> Result<(), String> {
    if input.is_empty() || input.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(())
    } else {
        let corrected = auto_correct_scope(input);
        Err(format!(
            "Scope must contain only alphanumeric characters and hyphens.\nSuggested correction: {}",
            corrected
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_commit_type() {
        // Exact matches
        assert_eq!(suggest_commit_type("feat"), Some("feat"));
        assert_eq!(suggest_commit_type("fix"), Some("fix"));

        // Close matches with different cases and whitespace
        assert_eq!(suggest_commit_type("  Feature  "), Some("feat"));
        assert_eq!(suggest_commit_type("FIX"), Some("fix"));
        assert_eq!(suggest_commit_type("fixx"), Some("fix"));

        // No close matches
        assert_eq!(suggest_commit_type("something_completely_different"), None);
    }

    #[test]
    fn test_auto_correct_scope() {
        // Already correct
        assert_eq!(auto_correct_scope("user-auth"), "user-auth");
        assert_eq!(auto_correct_scope("api"), "api");

        // Needs correction
        assert_eq!(auto_correct_scope("user@auth"), "userauth");
        assert_eq!(auto_correct_scope("api!service"), "apiservice");
        assert_eq!(auto_correct_scope("front_end"), "frontend");

        // Empty or special cases
        assert_eq!(auto_correct_scope(""), "");
        assert_eq!(auto_correct_scope("!@#$"), "");
    }

    #[test]
    fn test_validate_scope() {
        // Valid scopes
        assert!(validate_scope("").is_ok());
        assert!(validate_scope("auth").is_ok());
        assert!(validate_scope("user-service").is_ok());

        // Invalid scopes
        let invalid_result = validate_scope("user@service");
        assert!(invalid_result.is_err());
        assert!(invalid_result.unwrap_err().contains("userservice"));

        let invalid_result = validate_scope("api!!!");
        assert!(invalid_result.is_err());
        assert!(invalid_result.unwrap_err().contains("api"));
    }

    #[test]
    fn test_validate_short_message() {
        // Valid messages
        assert!(validate_short_message("Add user authentication").is_ok());
        assert!(validate_short_message("").is_ok());

        // Message too long
        let long_message = "a".repeat(MAX_SHORT_DESCRIPTION_LENGTH + 1);
        assert!(validate_short_message(&long_message).is_err());
    }
}
