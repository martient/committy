mod common;

use committy::input::validation::{
    auto_correct_scope, suggest_commit_type, validate_scope, validate_short_message,
};

#[test]
fn test_scope_validation_edge_cases() {
    // Valid scopes
    assert!(validate_scope("").is_ok()); // Empty scope is valid
    assert!(validate_scope("api").is_ok());
    assert!(validate_scope("user-service").is_ok());
    assert!(validate_scope("123").is_ok());
    assert!(validate_scope("api-v2").is_ok());

    // Invalid scopes
    let invalid_cases = vec![
        "user@service",
        "api!!!",
        "test space",
        "special#chars",
        "emoji🚀",
        "path/to/something",
    ];

    for invalid_scope in invalid_cases {
        let result = validate_scope(invalid_scope);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("alphanumeric"));

        // Check auto-correction
        let corrected = auto_correct_scope(invalid_scope);
        assert!(validate_scope(&corrected).is_ok());
    }
}

#[test]
fn test_commit_type_suggestions() {
    // Exact matches
    assert_eq!(suggest_commit_type("feat"), Some("feat"));
    assert_eq!(suggest_commit_type("fix"), Some("fix"));

    // Case variations
    assert_eq!(suggest_commit_type("FEAT"), Some("feat"));
    assert_eq!(suggest_commit_type("Fix"), Some("fix"));
    assert_eq!(suggest_commit_type("DOCS"), Some("docs"));

    // Common typos
    assert_eq!(suggest_commit_type("feature"), Some("feat"));
    assert_eq!(suggest_commit_type("fixx"), Some("fix"));
    assert_eq!(suggest_commit_type("docs!"), Some("docs"));
    assert_eq!(suggest_commit_type("feet"), Some("feat"));
    assert_eq!(suggest_commit_type("ffix"), Some("fix"));

    // With whitespace
    assert_eq!(suggest_commit_type("  feat  "), Some("feat"));
    assert_eq!(suggest_commit_type(" fix "), Some("fix"));

    // No close matches
    assert_eq!(suggest_commit_type("completely-wrong"), None);
    assert_eq!(suggest_commit_type("12345"), None);
    assert_eq!(suggest_commit_type(""), None);
}

#[test]
fn test_message_length_validation() {
    // Valid messages
    assert!(validate_short_message("Simple message").is_ok());
    assert!(validate_short_message("").is_ok());
    assert!(validate_short_message("A").is_ok());
    assert!(validate_short_message("Fix a critical bug").is_ok());

    // Messages at the limit
    let at_limit = "a".repeat(150); // Using MAX_SHORT_DESCRIPTION_LENGTH from config
    assert!(validate_short_message(&at_limit).is_ok());

    // Messages over the limit
    let over_limit = "a".repeat(151);
    assert!(validate_short_message(&over_limit).is_err());

    // Long message with spaces
    let long_message = "This is a very long commit message that should definitely exceed the maximum allowed length for a short description. Adding more text to make it longer than 150 characters. Still adding more text to be absolutely sure it's over the limit.";
    assert!(validate_short_message(long_message).is_err());
}

#[test]
fn test_scope_auto_correction() {
    // Test special characters removal
    assert_eq!(auto_correct_scope("user@service"), "userservice");
    assert_eq!(auto_correct_scope("api!!!"), "api");
    assert_eq!(auto_correct_scope("test space"), "testspace");
    assert_eq!(auto_correct_scope("special#chars"), "specialchars");
    assert_eq!(auto_correct_scope("emoji🚀"), "emoji");

    // Test case preservation
    assert_eq!(auto_correct_scope("UserService"), "UserService");
    assert_eq!(auto_correct_scope("API-service"), "API-service");

    // Test valid input preservation
    assert_eq!(auto_correct_scope("api"), "api");
    assert_eq!(auto_correct_scope("user-service"), "user-service");
    assert_eq!(auto_correct_scope("123"), "123");

    // Test empty and whitespace
    assert_eq!(auto_correct_scope(""), "");
    assert_eq!(auto_correct_scope("  "), "");
    assert_eq!(auto_correct_scope(" - "), "-");
}
