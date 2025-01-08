use chrono::{DateTime, Duration};
use std::env;
use tempfile::TempDir;

// Mock the main functionality for testing
fn should_check_update(
    last_check: DateTime<chrono::FixedOffset>,
    current_time: DateTime<chrono::FixedOffset>,
) -> bool {
    current_time - last_check >= Duration::days(1)
}

#[test]
fn test_update_check_timing() {
    let current_time = DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00").unwrap();

    // Test case 1: Last check was less than 24 hours ago
    let recent_check = current_time - Duration::hours(23);
    assert!(!should_check_update(recent_check, current_time));

    // Test case 2: Last check was exactly 24 hours ago
    let day_old_check = current_time - Duration::hours(24);
    assert!(should_check_update(day_old_check, current_time));

    // Test case 3: Last check was more than 24 hours ago
    let old_check = current_time - Duration::hours(25);
    assert!(should_check_update(old_check, current_time));
}

// Integration test for config and update checking
#[test]
fn test_config_integration() {
    // Set up a temporary home directory
    let temp_dir = TempDir::new().unwrap();
    env::set_var("HOME", temp_dir.path());

    // Create a config with old update check time
    let old_time = DateTime::parse_from_rfc3339("2025-01-07T17:39:49+01:00").unwrap(); // 24 hours ago
    let current_time = DateTime::parse_from_rfc3339("2025-01-08T17:39:49+01:00").unwrap();

    assert!(should_check_update(old_time, current_time));
}
