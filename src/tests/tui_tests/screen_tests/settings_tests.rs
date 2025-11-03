// SettingsScreen Tests - Testing settings configuration screen

use crate::tui::screens::SettingsScreen;

#[test]
fn test_settings_screen_creation() {
    let screen = SettingsScreen::new(1); // Default is 1 minute

    // Should initialize with provided retry interval
    assert_eq!(screen.retry_interval_input, "1");
    assert_eq!(screen.selected_field, 0);
    assert!(screen.status_message.is_some());
    assert!(!screen.is_error);
}

#[test]
fn test_settings_screen_add_char() {
    let mut screen = SettingsScreen::new(10);
    screen.clear_input();

    // Should accept digits
    screen.add_char('1');
    assert_eq!(screen.retry_interval_input, "1");

    screen.add_char('5');
    assert_eq!(screen.retry_interval_input, "15");

    // Should reject non-digits
    screen.add_char('a');
    assert_eq!(screen.retry_interval_input, "15");

    screen.add_char('!');
    assert_eq!(screen.retry_interval_input, "15");
}

#[test]
fn test_settings_screen_add_char_max_length() {
    // No longer needed
    

    let mut screen = SettingsScreen::new(10);
    screen.clear_input();

    // Should accept up to 4 digits
    screen.add_char('1');
    screen.add_char('2');
    screen.add_char('3');
    screen.add_char('4');
    assert_eq!(screen.retry_interval_input, "1234");

    // Should reject 5th digit
    screen.add_char('5');
    assert_eq!(screen.retry_interval_input, "1234");

    // Should still reject 6th digit
    screen.add_char('6');
    assert_eq!(screen.retry_interval_input, "1234");
}

#[test]
fn test_settings_screen_backspace() {
    // No longer needed
    

    let mut screen = SettingsScreen::new(10);
    screen.retry_interval_input = "123".to_string();

    screen.backspace();
    assert_eq!(screen.retry_interval_input, "12");

    screen.backspace();
    assert_eq!(screen.retry_interval_input, "1");

    screen.backspace();
    assert_eq!(screen.retry_interval_input, "");

    // Backspace on empty should not panic
    screen.backspace();
    assert_eq!(screen.retry_interval_input, "");
}

#[test]
fn test_settings_screen_clear_input() {
    // No longer needed
    

    let mut screen = SettingsScreen::new(10);
    screen.retry_interval_input = "123".to_string();

    screen.clear_input();
    assert_eq!(screen.retry_interval_input, "");
}

#[test]
fn test_settings_screen_validate_empty() {
    let mut screen = SettingsScreen::new(10);
    screen.clear_input();

    let result = screen.validate();

    assert!(result.is_none());
    assert!(screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("empty"));
}

#[test]
fn test_settings_screen_validate_zero() {
    let mut screen = SettingsScreen::new(10);
    screen.retry_interval_input = "0".to_string();

    let result = screen.validate();

    assert!(result.is_none());
    assert!(screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("at least 1"));
}

#[test]
fn test_settings_screen_validate_too_large() {
    let mut screen = SettingsScreen::new(10);
    screen.retry_interval_input = "2000".to_string();

    let result = screen.validate();

    assert!(result.is_none());
    assert!(screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("1440"));
}

#[test]
fn test_settings_screen_validate_valid() {
    let mut screen = SettingsScreen::new(10);
    screen.retry_interval_input = "30".to_string();

    let result = screen.validate();

    assert_eq!(result, Some(30));
    assert!(!screen.is_error);
}

#[test]
fn test_settings_screen_set_saved_message() {
    let mut screen = SettingsScreen::new(10);

    screen.set_saved_message(45);

    assert!(!screen.is_error);
    assert!(screen.status_message.as_ref().unwrap().contains("✓"));
    assert!(screen.status_message.as_ref().unwrap().contains("45"));
}
