//! Screen navigation and menu selection tests

use crate::tui::{Screen, MenuItem};
use super::helpers::create_test_app;

#[test]
fn test_app_navigation() {
    let (mut app, _temp_dir) = create_test_app();

    // Test next navigation
    assert_eq!(app.selected_index, 0);
    app.next();
    assert_eq!(app.selected_index, 1);
    app.next();
    assert_eq!(app.selected_index, 2);
    app.next();
    assert_eq!(app.selected_index, 3);
    app.next();
    assert_eq!(app.selected_index, 4);
    app.next();
    assert_eq!(app.selected_index, 5);

    // Test wrap around
    app.next();
    assert_eq!(app.selected_index, 0, "Should wrap to beginning");

    // Test previous navigation
    app.previous();
    assert_eq!(app.selected_index, 5, "Should wrap to end");
    app.previous();
    assert_eq!(app.selected_index, 4);
}

#[test]
fn test_app_show_share_contact_screen() {
    let (mut app, _temp_dir) = create_test_app();

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.share_contact_screen.is_none());

    // Show share contact screen
    app.show_share_contact_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::ShareContact);
    assert!(
        app.share_contact_screen.is_some(),
        "Share contact screen should be initialized"
    );

    // Verify token was generated
    let screen = app.share_contact_screen.as_ref().unwrap();
    assert!(!screen.token.is_empty());
    assert!(screen.expiry > chrono::Utc::now());
}

#[test]
fn test_app_back_to_main_menu() {
    let (mut app, _temp_dir) = create_test_app();

    // Show share contact screen
    app.show_share_contact_screen();
    assert_eq!(app.current_screen, Screen::ShareContact);
    assert!(app.share_contact_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(
        app.share_contact_screen.is_none(),
        "Share contact screen should be cleared"
    );
}

#[test]
fn test_app_select_share_contact() {
    let (mut app, _temp_dir) = create_test_app();

    // Select ShareContact item (index 1)
    app.selected_index = 1;
    assert_eq!(app.selected_item(), MenuItem::ShareContact);

    // Trigger selection
    app.select();

    // Should navigate to share contact screen
    assert_eq!(app.current_screen, Screen::ShareContact);
    assert!(app.share_contact_screen.is_some());
}

#[test]
fn test_app_select_exit() {
    let (mut app, _temp_dir) = create_test_app();

    // Navigate to Exit item (index 5)
    app.selected_index = 5;
    assert_eq!(app.selected_item(), MenuItem::Exit);
    assert!(!app.should_quit);

    // Trigger selection
    app.select();

    // Should set quit flag
    assert!(app.should_quit);
}

#[test]
fn test_app_show_import_contact_screen() {
    let (mut app, _temp_dir) = create_test_app();

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.import_contact_screen.is_none());

    // Show import contact screen
    app.show_import_contact_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::ImportContact);
    assert!(
        app.import_contact_screen.is_some(),
        "Import contact screen should be initialized"
    );
}

#[test]
fn test_app_back_from_import_contact() {
    let (mut app, _temp_dir) = create_test_app();

    // Show import contact screen
    app.show_import_contact_screen();
    assert_eq!(app.current_screen, Screen::ImportContact);
    assert!(app.import_contact_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(
        app.import_contact_screen.is_none(),
        "Import contact screen should be cleared"
    );
}

#[test]
fn test_app_select_import_contact() {
    let (mut app, _temp_dir) = create_test_app();

    // Navigate to ImportContact item (index 2)
    app.selected_index = 2;
    assert_eq!(app.selected_item(), MenuItem::ImportContact);

    // Trigger selection
    app.select();

    // Should navigate to import contact screen
    assert_eq!(app.current_screen, Screen::ImportContact);
    assert!(app.import_contact_screen.is_some());
}

#[test]
fn test_app_show_chat_list_screen() {
    let (mut app, _temp_dir) = create_test_app();

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.chat_list_screen.is_none());

    // Show chat list screen
    app.show_chat_list_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(
        app.chat_list_screen.is_some(),
        "Chat list screen should be initialized"
    );
}

#[test]
fn test_app_back_from_chat_list() {
    let (mut app, _temp_dir) = create_test_app();

    // Show chat list screen
    app.show_chat_list_screen();
    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(app.chat_list_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(
        app.chat_list_screen.is_none(),
        "Chat list screen should be cleared"
    );
}

#[test]
fn test_app_select_chat_list() {
    let (mut app, _temp_dir) = create_test_app();

    // Navigate to ChatList item (index 0)
    app.selected_index = 0;
    assert_eq!(app.selected_item(), MenuItem::ChatList);

    // Trigger selection
    app.select();

    // Should navigate to chat list screen
    assert_eq!(app.current_screen, Screen::ChatList);
    assert!(app.chat_list_screen.is_some());
}

#[test]
fn test_app_show_settings_screen() {
    let (mut app, _temp_dir) = create_test_app();

    // Initially on main menu
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.settings_screen.is_none());

    // Show settings screen
    app.show_settings_screen();

    // Verify screen changed
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());
}

#[test]
fn test_app_back_from_settings() {
    let (mut app, _temp_dir) = create_test_app();

    // Show settings screen
    app.show_settings_screen();
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());

    // Go back to main menu
    app.back_to_main_menu();

    // Verify state
    assert_eq!(app.current_screen, Screen::MainMenu);
    assert!(app.settings_screen.is_none());
}

#[test]
fn test_app_select_settings() {
    let (mut app, _temp_dir) = create_test_app();

    // Navigate to Settings item (index 4)
    app.selected_index = 4;
    assert_eq!(app.selected_item(), MenuItem::Settings);

    // Trigger selection
    app.select();

    // Should navigate to settings screen
    assert_eq!(app.current_screen, Screen::Settings);
    assert!(app.settings_screen.is_some());
}
