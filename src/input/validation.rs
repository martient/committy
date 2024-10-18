use crate::config::MAX_SHORT_DESCRIPTION_LENGTH;

pub fn validate_short_message(input: &str) -> Result<(), String> {
    if input.len() <= MAX_SHORT_DESCRIPTION_LENGTH {
        Ok(())
    } else {
        Err(format!("The message must be {} characters or less", MAX_SHORT_DESCRIPTION_LENGTH))
    }
}

pub fn validate_scope(input: &str) -> Result<(), String> {
    if input.is_empty() || input.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(())
    } else {
        Err("Scope must contain only alphanumeric characters and hyphens".to_string())
    }
}