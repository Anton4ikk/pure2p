// Mapping Consent Screen Tests

use crate::tui::screens::MappingConsentScreen;
use crate::storage::MappingConsent;

#[test]
fn test_mapping_consent_screen_initialization() {
    let screen = MappingConsentScreen::new();

    assert_eq!(screen.selected_option, 0, "Should default to 'Always Allow'");
}

#[test]
fn test_mapping_consent_screen_next_option() {
    let mut screen = MappingConsentScreen::new();

    // Start at 0 (Always Allow)
    assert_eq!(screen.selected_option, 0);

    // Move to 1 (Once)
    screen.next_option();
    assert_eq!(screen.selected_option, 1);

    // Move to 2 (Deny)
    screen.next_option();
    assert_eq!(screen.selected_option, 2);

    // Wrap back to 0
    screen.next_option();
    assert_eq!(screen.selected_option, 0);
}

#[test]
fn test_mapping_consent_screen_previous_option() {
    let mut screen = MappingConsentScreen::new();

    // Start at 0, go back wraps to 2
    assert_eq!(screen.selected_option, 0);
    screen.previous_option();
    assert_eq!(screen.selected_option, 2);

    // Move back to 1
    screen.previous_option();
    assert_eq!(screen.selected_option, 1);

    // Move back to 0
    screen.previous_option();
    assert_eq!(screen.selected_option, 0);
}

#[test]
fn test_mapping_consent_screen_get_consent() {
    let mut screen = MappingConsentScreen::new();

    // Option 0 = Always Allow
    screen.selected_option = 0;
    assert_eq!(screen.get_consent(), MappingConsent::AlwaysAllow);

    // Option 1 = Once
    screen.selected_option = 1;
    assert_eq!(screen.get_consent(), MappingConsent::Once);

    // Option 2 = Deny
    screen.selected_option = 2;
    assert_eq!(screen.get_consent(), MappingConsent::Deny);
}

#[test]
fn test_mapping_consent_screen_labels() {
    let screen = MappingConsentScreen::new();

    assert_eq!(screen.get_option_label(0), "Always allow");
    assert_eq!(screen.get_option_label(1), "Once");
    assert_eq!(screen.get_option_label(2), "Deny");
}

#[test]
fn test_mapping_consent_screen_descriptions() {
    let screen = MappingConsentScreen::new();

    assert_eq!(
        screen.get_option_description(0),
        "Automatically configure network on every startup"
    );
    assert_eq!(
        screen.get_option_description(1),
        "Allow only this time (ask again next time)"
    );
    assert_eq!(
        screen.get_option_description(2),
        "Never configure network automatically"
    );
}
