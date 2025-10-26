//! Main TUI application state and logic

use crate::crypto::KeyPair;
use crate::storage::{AppState, Message};
use crate::tui::types::{Screen, MenuItem};
use crate::tui::screens::*;
use chrono::Utc;

/// Application state
pub struct App {
    /// Current screen
    pub current_screen: Screen,
    /// Currently selected menu item
    pub selected_index: usize,
    /// Menu items
    pub menu_items: Vec<MenuItem>,
    /// User's keypair
    pub keypair: KeyPair,
    /// Local IP address
    pub local_ip: String,
    /// Should quit
    pub should_quit: bool,
    /// Application state (chats, contacts, settings)
    pub app_state: AppState,
    /// Mapping consent screen (when active)
    pub mapping_consent_screen: Option<MappingConsentScreen>,
    /// Share contact screen (when active)
    pub share_contact_screen: Option<ShareContactScreen>,
    /// Import contact screen (when active)
    pub import_contact_screen: Option<ImportContactScreen>,
    /// Chat list screen (when active)
    pub chat_list_screen: Option<ChatListScreen>,
    /// Chat view screen (when active)
    pub chat_view_screen: Option<ChatViewScreen>,
    /// Settings screen (when active)
    pub settings_screen: Option<SettingsScreen>,
    /// Diagnostics screen (when active)
    pub diagnostics_screen: Option<DiagnosticsScreen>,
    /// Startup sync screen (when active)
    pub startup_sync_screen: Option<StartupSyncScreen>,
}

impl App {
    /// Create new application
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let keypair = KeyPair::generate()?;
        let local_ip = Self::get_local_ip();
        let app_state = AppState::new();

        // Load settings to check mapping consent
        let settings = crate::storage::Settings::load("settings.json").unwrap_or_default();
        let should_ask_consent = settings.should_ask_consent();

        // Simulate checking for pending messages
        // In a real implementation, this would query the MessageQueue
        let pending_count = 0; // TODO: Query actual queue

        // Determine initial screen based on consent and pending messages
        let (current_screen, startup_sync_screen, mapping_consent_screen) = if should_ask_consent {
            // First run: show mapping consent dialog
            (Screen::MappingConsent, None, Some(MappingConsentScreen::new()))
        } else if pending_count > 0 {
            // Has pending messages: show startup sync
            (Screen::StartupSync, Some(StartupSyncScreen::new(pending_count)), None)
        } else {
            // Normal startup: show main menu
            (Screen::MainMenu, None, None)
        };

        Ok(Self {
            current_screen,
            selected_index: 0,
            menu_items: MenuItem::all(),
            keypair,
            local_ip,
            should_quit: false,
            app_state,
            mapping_consent_screen,
            share_contact_screen: None,
            import_contact_screen: None,
            chat_list_screen: None,
            chat_view_screen: None,
            settings_screen: None,
            diagnostics_screen: None,
            startup_sync_screen,
        })
    }

    /// Show mapping consent screen
    pub fn show_mapping_consent_screen(&mut self) {
        self.mapping_consent_screen = Some(MappingConsentScreen::new());
        self.current_screen = Screen::MappingConsent;
    }

    /// Get currently selected menu item
    pub fn selected_item(&self) -> MenuItem {
        self.menu_items[self.selected_index]
    }

    /// Move to next menu item
    pub fn next(&mut self) {
        self.selected_index = (self.selected_index + 1) % self.menu_items.len();
    }

    /// Move to previous menu item
    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.menu_items.len() - 1;
        }
    }

    /// Select current menu item
    pub fn select(&mut self) {
        match self.selected_item() {
            MenuItem::ChatList => {
                self.show_chat_list_screen();
            }
            MenuItem::ShareContact => {
                self.show_share_contact_screen();
            }
            MenuItem::ImportContact => {
                self.show_import_contact_screen();
            }
            MenuItem::Diagnostics => {
                self.show_diagnostics_screen();
            }
            MenuItem::Settings => {
                self.show_settings_screen();
            }
            MenuItem::Exit => {
                self.should_quit = true;
            }
        }
    }

    /// Show chat list screen
    pub fn show_chat_list_screen(&mut self) {
        self.chat_list_screen = Some(ChatListScreen::new());
        self.current_screen = Screen::ChatList;
    }

    /// Show share contact screen
    pub fn show_share_contact_screen(&mut self) {
        self.share_contact_screen = Some(ShareContactScreen::new(&self.keypair, &self.local_ip));
        self.current_screen = Screen::ShareContact;
    }

    /// Show import contact screen
    pub fn show_import_contact_screen(&mut self) {
        self.import_contact_screen = Some(ImportContactScreen::new());
        self.current_screen = Screen::ImportContact;
    }

    /// Show settings screen
    pub fn show_settings_screen(&mut self) {
        self.settings_screen = Some(SettingsScreen::new("settings.json".to_string()));
        self.current_screen = Screen::Settings;
    }

    /// Show diagnostics screen
    pub fn show_diagnostics_screen(&mut self) {
        let default_port = 8080; // TODO: Get from actual listening port
        let mut screen = DiagnosticsScreen::new(default_port);

        // Populate with current app state data
        self.update_diagnostics_with_app_state(&mut screen);

        self.diagnostics_screen = Some(screen);
        self.current_screen = Screen::Diagnostics;
    }

    /// Update diagnostics screen with current app state
    pub fn update_diagnostics_with_app_state(&self, screen: &mut DiagnosticsScreen) {
        // Set IPv4 address from local_ip
        if !self.local_ip.is_empty() {
            // Parse to check if it's IPv4 or IPv6
            if let Ok(ip) = self.local_ip.parse::<std::net::IpAddr>() {
                if ip.is_ipv4() {
                    screen.set_ipv4_address(Some(self.local_ip.clone()));
                } else if ip.is_ipv6() {
                    screen.set_ipv6_address(Some(self.local_ip.clone()));
                }
            } else {
                // If it's not a valid IP, assume IPv4 for display
                screen.set_ipv4_address(Some(self.local_ip.clone()));
            }
        }

        // Set queue size from app state
        screen.set_queue_size(self.app_state.message_queue.len());
    }

    /// Refresh diagnostics screen with latest data
    pub fn refresh_diagnostics(&mut self) {
        // Extract necessary data first to avoid borrow conflicts
        let local_ip = self.local_ip.clone();
        let queue_size = self.app_state.message_queue.len();

        if let Some(screen) = &mut self.diagnostics_screen {
            // Set IPv4 address from local_ip
            if !local_ip.is_empty() {
                // Parse to check if it's IPv4 or IPv6
                if let Ok(ip) = local_ip.parse::<std::net::IpAddr>() {
                    if ip.is_ipv4() {
                        screen.set_ipv4_address(Some(local_ip));
                    } else if ip.is_ipv6() {
                        screen.set_ipv6_address(Some(local_ip));
                    }
                } else {
                    // If it's not a valid IP, assume IPv4 for display
                    screen.set_ipv4_address(Some(local_ip));
                }
            }

            // Set queue size
            screen.set_queue_size(queue_size);
        }
    }

    /// Return to main menu
    pub fn back_to_main_menu(&mut self) {
        self.current_screen = Screen::MainMenu;
        self.mapping_consent_screen = None;
        self.share_contact_screen = None;
        self.import_contact_screen = None;
        self.chat_list_screen = None;
        self.chat_view_screen = None;
        self.settings_screen = None;
        self.diagnostics_screen = None;
    }

    /// Return to chat list
    pub fn back_to_chat_list(&mut self) {
        self.current_screen = Screen::ChatList;
        self.chat_view_screen = None;
    }

    /// Complete startup sync
    pub fn complete_startup_sync(&mut self) {
        self.current_screen = Screen::MainMenu;
        self.startup_sync_screen = None;
    }

    /// Confirm mapping consent selection and save to settings
    pub fn confirm_mapping_consent(&mut self) {
        if let Some(screen) = &self.mapping_consent_screen {
            let consent = screen.get_consent();

            // Load settings, update consent, and save
            if let Ok(mut settings) = crate::storage::Settings::load("settings.json") {
                let _ = settings.update_mapping_consent(consent, "settings.json");
            }
        }

        // Clear screen and return to main menu
        self.mapping_consent_screen = None;
        self.current_screen = Screen::MainMenu;
    }

    /// Update startup sync progress
    pub fn update_startup_sync(&mut self) {
        // Simulate processing messages
        // In a real implementation, this would actually send messages via the queue
        if let Some(sync_screen) = &mut self.startup_sync_screen {
            if !sync_screen.is_complete && sync_screen.current < sync_screen.total_messages {
                // Simulate success/failure (80% success rate for demo)
                let success = (sync_screen.current % 5) != 1;
                sync_screen.process_message(success);
            }
        }
    }

    /// Open selected chat
    pub fn open_selected_chat(&mut self) {
        if let Some(chat_list) = &self.chat_list_screen {
            if self.app_state.chats.is_empty() {
                return;
            }

            let contact_uid = self.app_state.chats[chat_list.selected_index].contact_uid.clone();
            self.chat_view_screen = Some(ChatViewScreen::new(contact_uid));
            self.current_screen = Screen::ChatView;
        }
    }

    /// Show delete confirmation popup
    pub fn show_delete_confirmation(&mut self) {
        if let Some(chat_list) = &mut self.chat_list_screen {
            if self.app_state.chats.is_empty() {
                return;
            }
            chat_list.show_delete_popup(chat_list.selected_index);
        }
    }

    /// Confirm deletion of chat
    pub fn confirm_delete_chat(&mut self) {
        if let Some(chat_list) = &self.chat_list_screen {
            if let Some(delete_index) = chat_list.pending_delete_index {
                if delete_index >= self.app_state.chats.len() {
                    return;
                }

                let chat = &self.app_state.chats[delete_index];
                let chat_uid = chat.contact_uid.clone();
                let is_active = chat.is_active;

                // Delete the chat
                self.app_state.chats.retain(|c| c.contact_uid != chat_uid);

                // Update status based on whether it was active or inactive
                if let Some(screen) = &mut self.chat_list_screen {
                    let status_msg = if is_active {
                        format!("Sent delete request and removed chat with {}", &chat_uid[..16.min(chat_uid.len())])
                    } else {
                        format!("Deleted inactive chat with {}", &chat_uid[..16.min(chat_uid.len())])
                    };
                    screen.set_status(status_msg);
                    screen.hide_delete_popup();

                    // Adjust selection if needed
                    if screen.selected_index >= self.app_state.chats.len() && !self.app_state.chats.is_empty() {
                        screen.selected_index = self.app_state.chats.len() - 1;
                    }
                }

                // TODO: Actually send delete request via transport if chat was active
                // For now, we just delete locally
            }
        }
    }

    /// Cancel chat deletion
    pub fn cancel_delete_chat(&mut self) {
        if let Some(screen) = &mut self.chat_list_screen {
            screen.hide_delete_popup();
        }
    }

    /// Send message in current chat
    pub fn send_message_in_chat(&mut self) {
        if let Some(chat_view) = &mut self.chat_view_screen {
            if chat_view.input.trim().is_empty() {
                return;
            }

            let message_content = chat_view.input.clone();
            let contact_uid = chat_view.contact_uid.clone();

            // Find the chat and add the message
            if let Some(chat) = self.app_state.chats.iter_mut().find(|c| c.contact_uid == contact_uid) {
                // Create a new message
                let message = Message::new(
                    uuid::Uuid::new_v4().to_string(),
                    self.keypair.uid.to_string(),
                    contact_uid.clone(),
                    message_content.as_bytes().to_vec(),
                    Utc::now().timestamp_millis(),
                );

                chat.append_message(message);
                chat_view.clear_input();
                chat_view.set_status("Message queued for sending".to_string());

                // TODO: Actually send via transport layer
                // For now, messages are just stored locally
            }
        }
    }

    /// Get local IP address (simplified - returns localhost for now)
    fn get_local_ip() -> String {
        // TODO: Implement actual local IP detection
        "127.0.0.1:8080".to_string()
    }
}
