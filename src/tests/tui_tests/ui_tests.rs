// UI Tests - Testing UI helper functions

use crate::tui::ui::format_duration_until;
use chrono::{Duration, Utc};

#[test]
fn test_format_duration_until_days() {
    let expiry = Utc::now() + Duration::days(15);
    let formatted = format_duration_until(expiry);
    // Allow for slight timing variance (14-15 days)
    assert!(
        formatted == "14 days" || formatted == "15 days",
        "Expected 14 or 15 days, got: {}",
        formatted
    );
}

#[test]
fn test_format_duration_until_hours() {
    let expiry = Utc::now() + Duration::hours(12);
    let formatted = format_duration_until(expiry);
    // Allow for slight timing variance (11-12 hours)
    assert!(
        formatted == "11 hours" || formatted == "12 hours",
        "Expected 11 or 12 hours, got: {}",
        formatted
    );
}

#[test]
fn test_format_duration_until_minutes() {
    let expiry = Utc::now() + Duration::minutes(45);
    let formatted = format_duration_until(expiry);
    // Allow for slight timing variance (44-45 minutes)
    assert!(
        formatted == "44 minutes" || formatted == "45 minutes",
        "Expected 44 or 45 minutes, got: {}",
        formatted
    );
}

#[test]
fn test_format_duration_until_expired() {
    let expiry = Utc::now() - Duration::hours(1);
    let formatted = format_duration_until(expiry);
    assert_eq!(formatted, "expired");
}
