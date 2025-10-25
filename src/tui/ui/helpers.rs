//! UI helper functions

use chrono::{DateTime, Utc};

/// Format a duration until expiry timestamp in human-readable form
pub fn format_duration_until(expiry: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = expiry.signed_duration_since(now);

    if duration.num_days() > 0 {
        format!("{} days", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes", duration.num_minutes())
    } else {
        "expired".to_string()
    }
}
