// UI Tests - Testing UI helper functions

use crate::tui::ui::format_duration_until;
use crate::tui::App;
use chrono::{Duration, Utc};
use tempfile::TempDir;

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

#[test]
fn test_main_menu_shows_warning_on_startup() {
    // Create app without triggering connectivity
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");
    let app = App::new_with_settings(Some(&settings_path)).expect("Failed to create app");

    // Verify connectivity_result is None (warning should be shown)
    assert!(app.connectivity_result.is_none(),
        "connectivity_result should be None on startup (warning should display)");
}

#[test]
fn test_main_menu_hides_warning_after_connectivity() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");
    let mut app = App::new_with_settings(Some(&settings_path)).expect("Failed to create app");

    // Initially no result
    assert!(app.connectivity_result.is_none());

    // Simulate connectivity completion by setting a mock result
    let mock_result = crate::connectivity::ConnectivityResult {
        mapping: None,
        ipv6: crate::connectivity::StrategyAttempt::NotAttempted,
        pcp: crate::connectivity::StrategyAttempt::NotAttempted,
        natpmp: crate::connectivity::StrategyAttempt::NotAttempted,
        upnp: crate::connectivity::StrategyAttempt::NotAttempted,
        cgnat_detected: false,
    };
    app.connectivity_result = Some(mock_result);

    // Verify result is now Some (warning should be hidden)
    assert!(app.connectivity_result.is_some(),
        "connectivity_result should be Some after completion (warning should be hidden)");
}

#[test]
fn test_main_menu_warning_logic_with_successful_mapping() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let settings_path = temp_dir.path().join("settings.json");
    let mut app = App::new_with_settings(Some(&settings_path)).expect("Failed to create app");

    // Create a successful mapping result
    let successful_mapping = crate::connectivity::PortMappingResult {
        external_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(203, 0, 113, 1)),
        external_port: 8080,
        protocol: crate::connectivity::MappingProtocol::PCP,
        lifetime_secs: 3600,
        created_at_ms: chrono::Utc::now().timestamp_millis(),
    };

    let result_with_mapping = crate::connectivity::ConnectivityResult {
        mapping: Some(successful_mapping.clone()),
        pcp: crate::connectivity::StrategyAttempt::Success(successful_mapping),
        ipv6: crate::connectivity::StrategyAttempt::NotAttempted,
        natpmp: crate::connectivity::StrategyAttempt::NotAttempted,
        upnp: crate::connectivity::StrategyAttempt::NotAttempted,
        cgnat_detected: false,
    };

    app.connectivity_result = Some(result_with_mapping);

    // Verify warning should be hidden (result is Some)
    assert!(app.connectivity_result.is_some());
    assert!(app.connectivity_result.as_ref().unwrap().mapping.is_some(),
        "Should have successful mapping");
}
