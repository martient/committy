use chrono::{DateTime, Duration};
use committy::clock::{current_time, should_check_update};
use std::env;
use tempfile::TempDir;

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

#[test]
fn test_clock_override_via_env() {
    let expected = "2026-03-15T09:30:00+01:00";
    env::set_var("COMMITTY_FIXED_NOW", expected);

    let now = current_time().unwrap();

    env::remove_var("COMMITTY_FIXED_NOW");
    assert_eq!(now.to_rfc3339(), expected);
}
