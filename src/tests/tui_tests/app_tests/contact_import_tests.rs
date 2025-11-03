//! Contact import business logic tests

use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token};
use chrono::{Duration, Utc};
use super::helpers::create_test_app;

#[test]
fn test_app_import_own_contact_rejected() {
    let (mut app, _temp_dir) = create_test_app();

    // Show import contact screen
    app.show_import_contact_screen();

    // Generate a contact token using the app's own keypair (simulating self-import)
    let expiry = Utc::now() + Duration::days(30);
    let token = generate_contact_token(
        "192.168.1.100:8080",
        &app.keypair.public_key,
        &app.keypair.private_key,
        &app.keypair.x25519_public,
        expiry,
    ).expect("Failed to generate token");

    // Parse the token to create a contact
    let contact = parse_contact_token(&token).expect("Failed to parse token");

    // Verify the contact UID matches the app's UID (self-import scenario)
    assert_eq!(contact.uid, app.keypair.uid.to_string());

    // Attempt to import the contact
    let initial_contact_count = app.app_state.contacts.len();
    app.import_contact(contact);

    // Verify the contact was NOT added
    assert_eq!(
        app.app_state.contacts.len(),
        initial_contact_count,
        "Self-import should not add contact to list"
    );

    // Verify error message was set
    let screen = app.import_contact_screen.as_ref().unwrap();
    assert!(screen.is_error, "Should be in error state");
    assert!(
        screen.status_message.as_ref().unwrap().contains("Cannot import your own"),
        "Should show self-import error message"
    );
}

#[test]
fn test_app_import_valid_contact() {
    let (mut app, _temp_dir) = create_test_app();

    // Show import contact screen
    app.show_import_contact_screen();

    // Generate a contact token using a DIFFERENT keypair (normal import)
    let other_keypair = KeyPair::generate().expect("Failed to generate keypair");
    let expiry = Utc::now() + Duration::days(30);
    let token = generate_contact_token(
        "192.168.1.200:8080",
        &other_keypair.public_key,
        &other_keypair.private_key,
        &other_keypair.x25519_public,
        expiry,
    ).expect("Failed to generate token");

    // Parse the token to create a contact
    let contact = parse_contact_token(&token).expect("Failed to parse token");

    // Verify the contact UID does NOT match the app's UID
    assert_ne!(contact.uid, app.keypair.uid.to_string());

    // Attempt to import the contact
    let initial_contact_count = app.app_state.contacts.len();
    app.import_contact(contact.clone());

    // Verify the contact WAS added
    assert_eq!(
        app.app_state.contacts.len(),
        initial_contact_count + 1,
        "Valid import should add contact to list"
    );

    // Verify success message was set
    let screen = app.import_contact_screen.as_ref().unwrap();
    assert!(!screen.is_error, "Should not be in error state");
    assert!(
        screen.status_message.as_ref().unwrap().contains("imported, ping sent"),
        "Should show success message"
    );

    // Verify the imported contact is in the list
    assert!(
        app.app_state.contacts.iter().any(|c| c.uid == contact.uid),
        "Contact should be in contacts list"
    );

    // Verify a chat was created for the imported contact
    let created_chat = app.app_state.chats.iter().find(|c| c.contact_uid == contact.uid);
    assert!(created_chat.is_some(), "Chat should be created for imported contact");

    // Verify the chat is marked as pending immediately (before ping attempt)
    let chat = created_chat.unwrap();
    assert!(
        !chat.is_active,
        "Newly created chat should not be active (ping hasn't succeeded yet)"
    );
    assert!(
        chat.has_pending_messages,
        "Chat should be marked as having pending messages (ping will be attempted/queued)"
    );
}

#[test]
fn test_app_import_duplicate_contact() {
    let (mut app, _temp_dir) = create_test_app();

    // Show import contact screen
    app.show_import_contact_screen();

    // Generate a contact token
    let other_keypair = KeyPair::generate().expect("Failed to generate keypair");
    let expiry = Utc::now() + Duration::days(30);
    let token = generate_contact_token(
        "192.168.1.200:8080",
        &other_keypair.public_key,
        &other_keypair.private_key,
        &other_keypair.x25519_public,
        expiry,
    ).expect("Failed to generate token");

    let contact = parse_contact_token(&token).expect("Failed to parse token");

    // Import the contact first time
    app.import_contact(contact.clone());
    let contact_count_after_first = app.app_state.contacts.len();

    // Try to import the same contact again
    app.import_contact(contact.clone());

    // Verify the contact was NOT added again
    assert_eq!(
        app.app_state.contacts.len(),
        contact_count_after_first,
        "Duplicate import should not add contact again"
    );

    // Verify error message was set
    let screen = app.import_contact_screen.as_ref().unwrap();
    assert!(screen.is_error, "Should be in error state");
    assert!(
        screen.status_message.as_ref().unwrap().contains("already exists"),
        "Should show duplicate contact error message"
    );
}
