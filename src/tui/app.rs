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
    /// Background diagnostics refresh handle
    pub diagnostics_refresh_handle: Option<std::thread::JoinHandle<crate::connectivity::ConnectivityResult>>,
    /// Connectivity result from startup or last refresh
    pub connectivity_result: Option<crate::connectivity::ConnectivityResult>,
    /// Local port for connectivity
    pub local_port: u16,
    /// Path to app state file
    state_path: String,
}

impl App {
    /// Create new application
    ///
    /// # Arguments
    /// * `state_path` - Optional path to app state file. Defaults to "app_state.json" if None.
    ///                  Used primarily for testing to avoid polluting user's state.
    pub fn new_with_settings<P: AsRef<std::path::Path>>(state_path: Option<P>) -> Result<Self, Box<dyn std::error::Error>> {
        // Determine state file path
        let state_path = state_path.as_ref()
            .map(|p| p.as_ref().to_string_lossy().to_string())
            .unwrap_or_else(|| "app_state.json".to_string());

        // Check if this is first run (file doesn't exist)
        let is_first_run = !std::path::Path::new(&state_path).exists();

        // Load or create app state (contacts, chats, messages, settings - everything!)
        let mut app_state = AppState::load(&state_path).unwrap_or_else(|_| {
            AppState::new() // Will have default settings
        });

        // Load or generate user keypair (persistent identity)
        let keypair = if let Some(existing_keypair) = &app_state.user_keypair {
            existing_keypair.clone()
        } else {
            // First run: generate new identity
            let new_keypair = KeyPair::generate()?;
            app_state.user_keypair = Some(new_keypair.clone());
            new_keypair
        };

        // Load or use default network info
        let local_ip = app_state.user_ip.clone().unwrap_or_else(Self::get_local_ip);
        let local_port = app_state.user_port;

        // Simulate checking for pending messages
        // In a real implementation, this would query the MessageQueue
        let pending_count = 0; // TODO: Query actual queue

        // Determine initial screen based on pending messages
        let (current_screen, startup_sync_screen) = if pending_count > 0 {
            // Has pending messages: show startup sync
            (Screen::StartupSync, Some(StartupSyncScreen::new(pending_count)))
        } else {
            // Normal startup: show main menu
            (Screen::MainMenu, None)
        };

        let app = Self {
            current_screen,
            selected_index: 0,
            menu_items: MenuItem::all(),
            keypair,
            local_ip,
            should_quit: false,
            app_state,
            share_contact_screen: None,
            import_contact_screen: None,
            chat_list_screen: None,
            chat_view_screen: None,
            settings_screen: None,
            diagnostics_screen: None,
            startup_sync_screen,
            diagnostics_refresh_handle: None,
            connectivity_result: None,
            local_port,
            state_path,
        };

        // Save initial state on first run
        if is_first_run {
            let _ = app.save_state();
        }

        Ok(app)
    }

    /// Create new application with default state path
    ///
    /// This is a convenience wrapper around `new_with_settings(None)`.
    /// For production use, all data will be stored in "app_state.json" in the project root.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_settings(None::<&str>)
    }

    /// Save application state to disk
    ///
    /// Persists contacts, chats, and messages to app_state.json
    pub fn save_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.app_state.save(&self.state_path)?;
        Ok(())
    }

    /// Trigger startup connectivity diagnostics (non-blocking)
    ///
    /// Should be called after App::new() to detect external IP for contact sharing.
    pub fn trigger_startup_connectivity(&mut self) {
        // Don't start if already running
        if self.diagnostics_refresh_handle.is_some() {
            return;
        }

        let port = self.local_port;

        // Spawn background thread with tokio runtime
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                crate::connectivity::establish_connectivity(port).await
            })
        });

        self.diagnostics_refresh_handle = Some(handle);
    }

    /// Poll for startup connectivity completion and update local IP
    ///
    /// Returns true if connectivity completed this call.
    pub fn poll_startup_connectivity(&mut self) -> bool {
        if let Some(handle) = self.diagnostics_refresh_handle.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(result) => {
                        // Update local_ip from the mapping result
                        if let Some(mapping) = &result.mapping {
                            let detected_ip = format!("{}:{}", mapping.external_ip, mapping.external_port);
                            self.local_ip = detected_ip.clone();

                            // Save detected IP to app_state for persistence
                            self.app_state.user_ip = Some(detected_ip);
                            let _ = self.save_state();
                        }
                        self.connectivity_result = Some(result.clone());

                        // Apply result to diagnostics screen if it's already open
                        self.apply_connectivity_result(result);
                        return true;
                    }
                    Err(_) => {
                        // Failed to get connectivity, keep default local_ip
                        return true;
                    }
                }
            } else {
                // Thread still running, put it back
                self.diagnostics_refresh_handle = Some(handle);
            }
        }
        false
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
        let current_interval = self.app_state.settings.retry_interval_minutes;
        self.settings_screen = Some(SettingsScreen::new(current_interval));
        self.current_screen = Screen::Settings;
    }

    /// Show diagnostics screen
    pub fn show_diagnostics_screen(&mut self) {
        let mut screen = DiagnosticsScreen::new(self.local_port);

        // Populate with current app state data
        self.update_diagnostics_with_app_state(&mut screen);

        // Apply existing connectivity result if available
        if let Some(result) = &self.connectivity_result {
            screen.update_from_connectivity_result(result);
        }

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

    /// Trigger async diagnostics refresh (non-blocking)
    ///
    /// This spawns a background thread with a tokio runtime to perform
    /// connectivity tests. Results are automatically polled in `poll_diagnostics_result()`.
    pub fn trigger_diagnostics_refresh(&mut self) {
        // Don't start a new refresh if one is already running
        if self.diagnostics_refresh_handle.is_some() {
            return;
        }

        if let Some(screen) = &mut self.diagnostics_screen {
            let port = screen.local_port;

            // Spawn background thread with tokio runtime
            let handle = std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async move {
                    crate::connectivity::establish_connectivity(port).await
                })
            });

            self.diagnostics_refresh_handle = Some(handle);
        }
    }

    /// Poll for diagnostics refresh completion (non-blocking)
    ///
    /// Checks if the background refresh thread has completed and applies results.
    /// Returns true if a refresh was completed this call.
    pub fn poll_diagnostics_result(&mut self) -> bool {
        if let Some(handle) = self.diagnostics_refresh_handle.take() {
            // Check if thread is finished (non-blocking)
            if handle.is_finished() {
                // Thread is done, get the result
                match handle.join() {
                    Ok(result) => {
                        // Update local_ip from the mapping result
                        if let Some(mapping) = &result.mapping {
                            let detected_ip = format!("{}:{}", mapping.external_ip, mapping.external_port);
                            self.local_ip = detected_ip.clone();

                            // Save detected IP to app_state for persistence
                            self.app_state.user_ip = Some(detected_ip);
                            let _ = self.save_state();
                        }
                        self.connectivity_result = Some(result.clone());
                        self.apply_connectivity_result(result);
                        return true;
                    }
                    Err(e) => {
                        if let Some(screen) = &mut self.diagnostics_screen {
                            screen.set_status_message(format!("Refresh failed: {:?}", e));
                            screen.is_refreshing = false;
                        }
                        return true;
                    }
                }
            } else {
                // Thread is still running, put the handle back
                self.diagnostics_refresh_handle = Some(handle);
            }
        }
        false
    }

    /// Apply connectivity result to diagnostics screen
    pub fn apply_connectivity_result(&mut self, result: crate::connectivity::ConnectivityResult) {
        if let Some(screen) = &mut self.diagnostics_screen {
            screen.update_from_connectivity_result(&result);
        }
    }

    /// Return to main menu
    pub fn back_to_main_menu(&mut self) {
        self.current_screen = Screen::MainMenu;
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

                // Auto-save after deleting chat
                let _ = self.save_state();

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

    /// Import a contact (chat will be created when first message is sent/received)
    pub fn import_contact(&mut self, contact: crate::storage::Contact) {
        // Check if trying to import own contact (self-import)
        if contact.uid == self.keypair.uid.to_string() {
            if let Some(screen) = &mut self.import_contact_screen {
                screen.status_message = Some("Error: Cannot import your own contact token".to_string());
                screen.is_error = true;
            }
            return;
        }

        // Check if contact already exists
        if !self.app_state.contacts.iter().any(|c| c.uid == contact.uid) {
            // Add contact to list
            self.app_state.contacts.push(contact.clone());

            // Auto-save after importing contact
            let _ = self.save_state();

            // Update import screen status
            if let Some(screen) = &mut self.import_contact_screen {
                screen.status_message = Some(format!("✓ Contact imported and saved!"));
                screen.is_error = false;
            }
        } else {
            // Contact already exists
            if let Some(screen) = &mut self.import_contact_screen {
                screen.status_message = Some("Contact already exists".to_string());
                screen.is_error = true;
            }
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

                // Auto-save after sending message
                let _ = self.save_state();

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
