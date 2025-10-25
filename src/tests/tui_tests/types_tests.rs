// Types Tests - Testing MenuItem enum and related types

use crate::tui::MenuItem;

#[test]
fn test_menu_item_labels() {
    assert_eq!(MenuItem::ShareContact.label(), "Share Contact");
    assert_eq!(MenuItem::ImportContact.label(), "Import Contact");
    assert_eq!(MenuItem::Settings.label(), "Settings");
    assert_eq!(MenuItem::Exit.label(), "Exit");
}

#[test]
fn test_menu_item_descriptions() {
    assert_eq!(
        MenuItem::ShareContact.description(),
        "Generate and share your contact token"
    );
    assert_eq!(
        MenuItem::ImportContact.description(),
        "Import a contact from their token"
    );
    assert_eq!(
        MenuItem::Settings.description(),
        "Configure application settings"
    );
    assert_eq!(MenuItem::Exit.description(), "Exit Pure2P");
}

#[test]
fn test_menu_items_updated() {
    // Verify ChatList is first item
    assert_eq!(MenuItem::ChatList.label(), "Chat List");
    assert_eq!(
        MenuItem::ChatList.description(),
        "View and manage your conversations"
    );

    // Verify menu has 6 items now (added Diagnostics)
    let items = MenuItem::all();
    assert_eq!(items.len(), 6);
    assert_eq!(items[0], MenuItem::ChatList);
    assert_eq!(items[1], MenuItem::ShareContact);
    assert_eq!(items[2], MenuItem::ImportContact);
    assert_eq!(items[3], MenuItem::Diagnostics);
    assert_eq!(items[4], MenuItem::Settings);
    assert_eq!(items[5], MenuItem::Exit);
}
