// ChatViewScreen Tests - Testing individual chat conversation view

use crate::tui::screens::ChatViewScreen;

#[test]
fn test_chat_view_screen_creation() {
    let screen = ChatViewScreen::new("alice_uid".to_string());

    assert_eq!(screen.contact_uid, "alice_uid");
    assert!(screen.input.is_empty(), "Input should be empty initially");
    assert_eq!(screen.scroll_offset, 0, "Should start at top");
    assert!(
        screen.status_message.is_none(),
        "Should have no status message initially"
    );
}

#[test]
fn test_chat_view_screen_input() {
    let mut screen = ChatViewScreen::new("alice_uid".to_string());

    screen.add_char('H');
    screen.add_char('i');
    assert_eq!(screen.input, "Hi");

    screen.backspace();
    assert_eq!(screen.input, "H");

    screen.clear_input();
    assert!(screen.input.is_empty());
}

#[test]
fn test_chat_view_screen_scroll() {
    let mut screen = ChatViewScreen::new("alice_uid".to_string());

    // Scroll down
    screen.scroll_down(10);
    assert_eq!(screen.scroll_offset, 1);
    screen.scroll_down(10);
    assert_eq!(screen.scroll_offset, 2);

    // Scroll up
    screen.scroll_up();
    assert_eq!(screen.scroll_offset, 1);
    screen.scroll_up();
    assert_eq!(screen.scroll_offset, 0);

    // Can't scroll past 0
    screen.scroll_up();
    assert_eq!(screen.scroll_offset, 0);

    // Can't scroll past max
    screen.scroll_offset = 10;
    screen.scroll_down(10);
    assert_eq!(screen.scroll_offset, 10, "Should stay at max offset");
}
