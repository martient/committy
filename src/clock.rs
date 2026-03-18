use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, FixedOffset, Local};

const FIXED_NOW_ENV: &str = "COMMITTY_FIXED_NOW";

pub fn current_time() -> Result<DateTime<FixedOffset>> {
    if let Ok(value) = std::env::var(FIXED_NOW_ENV) {
        return DateTime::parse_from_rfc3339(&value)
            .map_err(|e| anyhow!("Invalid {FIXED_NOW_ENV} value: {e}"));
    }

    Ok(Local::now().fixed_offset())
}

pub fn should_check_update(
    last_check: DateTime<FixedOffset>,
    current_time: DateTime<FixedOffset>,
) -> bool {
    current_time - last_check >= Duration::days(1)
}

pub fn should_remind_metrics(
    last_reminder: DateTime<FixedOffset>,
    current_time: DateTime<FixedOffset>,
) -> bool {
    current_time - last_reminder >= Duration::days(7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_time_uses_override() {
        let expected = "2026-03-15T11:22:33+01:00";
        std::env::set_var(FIXED_NOW_ENV, expected);

        let current = current_time().unwrap();

        std::env::remove_var(FIXED_NOW_ENV);
        assert_eq!(current.to_rfc3339(), expected);
    }

    #[test]
    fn test_should_check_update() {
        let current = DateTime::parse_from_rfc3339("2026-03-15T12:00:00+01:00").unwrap();
        assert!(!should_check_update(current - Duration::hours(23), current));
        assert!(should_check_update(current - Duration::hours(24), current));
    }

    #[test]
    fn test_should_remind_metrics() {
        let current = DateTime::parse_from_rfc3339("2026-03-15T12:00:00+01:00").unwrap();
        assert!(!should_remind_metrics(current - Duration::days(6), current));
        assert!(should_remind_metrics(current - Duration::days(7), current));
    }
}
