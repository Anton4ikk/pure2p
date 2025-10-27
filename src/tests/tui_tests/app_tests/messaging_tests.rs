//! Message sending tests

use super::helpers::create_test_app;

#[test]
fn test_app_send_message() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Open chat
    app.show_chat_list_screen();
    app.open_selected_chat();

    // Type a message
    if let Some(screen) = &mut app.chat_view_screen {
        screen.input = "Hello Alice!".to_string();
    }

    // Send it
    app.send_message_in_chat();

    // Verify message was added
    let chat = app.app_state.chats.iter()
        .find(|c| c.contact_uid == "alice_uid")
        .unwrap();
    assert_eq!(chat.messages.len(), 1);

    let msg = &chat.messages[0];
    assert_eq!(msg.sender, app.keypair.uid.to_string());
    assert_eq!(msg.recipient, "alice_uid");
    assert_eq!(
        String::from_utf8(msg.content.clone()).unwrap(),
        "Hello Alice!"
    );

    // Input should be cleared
    assert!(app.chat_view_screen.as_ref().unwrap().input.is_empty());
}

#[test]
fn test_app_send_empty_message() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Open chat
    app.show_chat_list_screen();
    app.open_selected_chat();

    // Try to send empty message
    app.send_message_in_chat();

    // Should not have added any message
    let chat = app.app_state.chats.iter()
        .find(|c| c.contact_uid == "alice_uid")
        .unwrap();
    assert_eq!(chat.messages.len(), 0);
}

#[test]
fn test_app_multiple_messages() {
    let (mut app, _temp_dir) = create_test_app();

    // Add chat
    app.app_state.add_chat("alice_uid".to_string());

    // Open chat
    app.show_chat_list_screen();
    app.open_selected_chat();

    // Send multiple messages
    for i in 1..=3 {
        if let Some(screen) = &mut app.chat_view_screen {
            screen.input = format!("Message {}", i);
        }
        app.send_message_in_chat();
    }

    // Verify all messages were added
    let chat = app.app_state.chats.iter()
        .find(|c| c.contact_uid == "alice_uid")
        .unwrap();
    assert_eq!(chat.messages.len(), 3);

    for (i, msg) in chat.messages.iter().enumerate() {
        let expected = format!("Message {}", i + 1);
        assert_eq!(
            String::from_utf8(msg.content.clone()).unwrap(),
            expected
        );
    }
}
