//! Main TUI application state and logic

use crate::crypto::KeyPair;
use crate::storage::{AppState, Message, storage_db::Storage};
use crate::tui::types::{Screen, MenuItem};
use crate::tui::screens::*;
use crate::transport::Transport;
use crate::queue::MessageQueue;
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
    /// Path to app state file (legacy, kept for compatibility)
    state_path: String,
    /// Transport layer for sending/receiving messages
    pub transport: Transport,
    /// Message queue for retry logic
    pub queue: MessageQueue,
    /// SQLite storage backend
    storage: Storage,
}

impl App {
    /// Create new application
    ///
    /// # Arguments
    /// * `state_path` - Optional path for testing. Production uses SQLite in ./app_data/
    ///                  Used primarily for testing to avoid polluting user's state.
    pub fn new_with_settings<P: AsRef<std::path::Path>>(state_path: Option<P>) -> Result<Self, Box<dyn std::error::Error>> {
        // Determine state file path (legacy, used for migration)
        let state_path = state_path.as_ref()
            .map(|p| p.as_ref().to_string_lossy().to_string())
            .unwrap_or_else(|| "app_state.json".to_string());

        // Initialize SQLite storage
        let storage = if state_path.contains("test") || state_path.contains("tmp") {
            // For tests, use in-memory database
            Storage::new_in_memory()?
        } else {
            // Production: use ./app_data/pure2p.db
            Storage::new_with_default_path()?
        };

        // Check for legacy app_state.json and migrate if exists
        let migrated = AppState::migrate_from_json(&state_path, &storage)?;
        if migrated {
            tracing::info!("Migrated app state from {} to SQLite database", state_path);
        }

        // Load app state from SQLite (or create new if empty)
        let mut app_state = AppState::load_from_db(&storage)?;

        // Check if this is first run (no user keypair)
        let is_first_run = app_state.user_keypair.is_none();

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

        // Create transport layer
        let transport = Transport::new();

        // Create message queue in app_data directory
        let queue_path = if state_path.contains("test") || state_path.contains("tmp") {
            // For tests, use the test directory
            std::path::Path::new(&state_path)
                .parent()
                .and_then(|p| p.to_str())
                .map(|p| format!("{}/message_queue.db", p))
                .unwrap_or_else(|| "message_queue.db".to_string())
        } else {
            // Production: use ./app_data/message_queue.db
            "./app_data/message_queue.db".to_string()
        };
        let queue = MessageQueue::new_with_path(&queue_path)?;

        // Check for pending messages in queue
        let pending_count = queue.count_pending()?;

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
            transport,
            queue,
            storage,
        };

        // Save initial state on first run
        if is_first_run {
            let _ = app.save_state();
        }

        Ok(app)
    }

    /// Create new application with default storage
    ///
    /// This is a convenience wrapper around `new_with_settings(None)`.
    /// For production use, all data will be stored in SQLite at ./app_data/pure2p.db
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_settings(None::<&str>)
    }

    /// Save application state to SQLite database
    ///
    /// Persists user identity, contacts, chats, messages, and settings to pure2p.db
    pub fn save_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.app_state.save_to_db(&self.storage)?;
        Ok(())
    }

    /// Reload application state from SQLite database
    ///
    /// This is called periodically to pick up changes made by transport handlers
    /// (e.g., incoming messages from other peers)
    ///
    /// Note: Only reloads in production (file-based storage). Tests use in-memory
    /// databases which don't share state between connections, so reloading would
    /// create a fresh empty database.
    pub fn reload_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Skip reload for test environments (in-memory databases don't share state)
        if self.state_path.contains("test") || self.state_path.contains("tmp") {
            return Ok(());
        }

        // Load fresh state from database
        let mut loaded_state = AppState::load_from_db(&self.storage)?;

        // Preserve user identity and network info if not in DB yet
        if loaded_state.user_keypair.is_none() {
            loaded_state.user_keypair = self.app_state.user_keypair.clone();
        }
        if loaded_state.user_ip.is_none() {
            loaded_state.user_ip = self.app_state.user_ip.clone();
        }

        // Update app state
        self.app_state = loaded_state;

        Ok(())
    }

    /// Start transport server in background
    ///
    /// This starts the HTTP server to receive messages and pings.
    /// Handlers are set up to automatically create chats when pings are received.
    pub fn start_transport(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set local UID for transport
        let uid = self.keypair.uid.to_string();
        let transport = self.transport.clone();
        let local_port = self.local_port;
        let state_path = self.state_path.clone();

        // Create storage instances for handlers (they need their own connections in separate threads)
        let use_in_memory = state_path.contains("test") || state_path.contains("tmp");

        // Setup ping handler and message handler to receive messages and pings
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                // Set local UID
                transport.set_local_uid(uid).await;

                // Setup ping handler
                let use_in_memory_ping = use_in_memory;
                transport.set_ping_handler(move |contact_token: String| {
                    // Create storage connection for this handler
                    let storage_result = if use_in_memory_ping {
                        Storage::new_in_memory()
                    } else {
                        Storage::new_with_default_path()
                    };

                    if let Ok(storage) = storage_result {
                        if let Ok(mut app_state) = AppState::load_from_db(&storage) {
                            // Parse and verify the contact token
                            match crate::storage::Contact::parse_token(&contact_token) {
                                Ok(sender_contact) => {
                                    tracing::info!("Received ping from {} at {}", sender_contact.uid, sender_contact.ip);

                                    // Check if contact already exists
                                    let contact_exists = app_state.contacts.iter().any(|c| c.uid == sender_contact.uid);

                                    if !contact_exists {
                                        // Auto-import the sender as a new contact
                                        app_state.contacts.push(sender_contact.clone());
                                        tracing::info!("Auto-imported contact {} from ping", sender_contact.uid);
                                    }

                                    // Create or get existing chat (active status, not pending)
                                    let chat = app_state.get_or_create_chat(&sender_contact.uid);
                                    chat.mark_unread(); // Mark as active (new ping received)
                                    let _ = app_state.save_to_db(&storage);
                                    tracing::info!("Created/updated chat for ping from {}", sender_contact.uid);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse contact token from ping: {}", e);
                                }
                            }
                        }
                    }
                }).await;

                // Setup message handler
                let use_in_memory_msg = use_in_memory;
                transport.set_new_message_handler(move |msg_req: crate::transport::MessageRequest| {
                    // Create storage connection for this handler
                    let storage_result = if use_in_memory_msg {
                        Storage::new_in_memory()
                    } else {
                        Storage::new_with_default_path()
                    };

                    if let Ok(storage) = storage_result {
                        if let Ok(mut app_state) = AppState::load_from_db(&storage) {
                            // Get to_uid before borrowing app_state mutably
                            let to_uid = app_state.user_keypair.as_ref().map(|kp| kp.uid.to_string()).unwrap_or_default();

                            let chat = app_state.get_or_create_chat(&msg_req.from_uid);

                            // Create message from the request
                            let message = Message::new(
                                uuid::Uuid::new_v4().to_string(),
                                msg_req.from_uid.clone(),
                                to_uid,
                                msg_req.payload,
                                Utc::now().timestamp_millis(),
                            );

                            chat.append_message(message);
                            chat.mark_unread(); // Mark as unread (new message received)

                            let _ = app_state.save_to_db(&storage);
                            tracing::info!("Received message from {} (type: {})", msg_req.from_uid, msg_req.message_type);
                        }
                    }
                }).await;

                // Start server
                let addr = format!("0.0.0.0:{}", local_port).parse().expect("Invalid address");
                if let Err(e) = transport.clone().start(addr).await {
                    tracing::error!("Transport server failed: {}", e);
                }
            });
        });

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
                        // Update local_ip and port from the mapping result
                        if let Some(mapping) = &result.mapping {
                            let detected_ip = format!("{}:{}", mapping.external_ip, mapping.external_port);
                            self.local_ip = detected_ip.clone();

                            // Save detected IP and port to app_state for persistence
                            self.app_state.user_ip = Some(detected_ip);
                            self.app_state.user_port = mapping.external_port;
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
        // Reload state to pick up any new messages from transport handlers
        let _ = self.reload_state();

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

        // Set queue size from SQLite queue (not app_state.message_queue)
        let queue_size = self.queue.count_pending().unwrap_or(0);
        screen.set_queue_size(queue_size);
    }

    /// Refresh diagnostics screen with latest data
    pub fn refresh_diagnostics(&mut self) {
        // Extract necessary data first to avoid borrow conflicts
        let local_ip = self.local_ip.clone();
        let queue_size = self.queue.count_pending().unwrap_or(0);

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
                        // Update local_ip and port from the mapping result
                        if let Some(mapping) = &result.mapping {
                            let detected_ip = format!("{}:{}", mapping.external_ip, mapping.external_port);
                            self.local_ip = detected_ip.clone();

                            // Save detected IP and port to app_state for persistence
                            self.app_state.user_ip = Some(detected_ip);
                            self.app_state.user_port = mapping.external_port;
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
        // Reload state to pick up any changes while in other screens
        let _ = self.reload_state();

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
        // Reload state to show any new messages
        let _ = self.reload_state();

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
        // Get selected index first to avoid borrow checker issues
        let selected_index = if let Some(chat_list) = &self.chat_list_screen {
            if self.app_state.chats.is_empty() {
                return;
            }
            chat_list.selected_index
        } else {
            return;
        };

        // Reload state before entering chat to show latest messages
        let _ = self.reload_state();

        let contact_uid = self.app_state.chats[selected_index].contact_uid.clone();
        self.chat_view_screen = Some(ChatViewScreen::new(contact_uid));
        self.current_screen = Screen::ChatView;
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

    /// Import a contact and create a new chat
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
            let contact_uid = contact.uid.clone();

            // Add contact to list
            self.app_state.contacts.push(contact.clone());

            // Create a new chat for this contact and mark as pending
            // (waiting for ping response)
            let mut new_chat = crate::storage::Chat::new(contact_uid);
            new_chat.mark_has_pending(); // Show ⌛ Pending status until ping response
            self.app_state.chats.push(new_chat);

            // Auto-save after importing contact and creating chat
            let _ = self.save_state();

            // Generate my contact token to send in ping (so receiver can auto-import me)
            let my_contact = crate::storage::Contact::new(
                self.keypair.uid.to_string(),
                self.local_ip.clone(),
                self.keypair.public_key.clone(),
                self.keypair.x25519_public.clone(),
                Utc::now() + chrono::Duration::days(1), // 24 hour expiry
            );
            let my_token = match my_contact.sign_token(&self.keypair) {
                Ok(token) => token,
                Err(e) => {
                    tracing::error!("Failed to generate contact token for ping: {}", e);
                    if let Some(screen) = &mut self.import_contact_screen {
                        screen.status_message = Some(format!("Error generating token: {}", e));
                        screen.is_error = true;
                    }
                    return;
                }
            };

            // Send ping to the contact in background thread
            let transport = self.transport.clone();
            let contact_for_ping = contact.clone();
            let storage_clone = self.storage.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async move {
                    match transport.send_ping(&contact_for_ping, &my_token).await {
                        Ok(_ping_response) => {
                            tracing::info!("Successfully pinged newly imported contact {}", contact_for_ping.uid);

                            // Update chat status: remove pending flag since ping succeeded
                            if let Ok(mut app_state) = AppState::load_from_db(&storage_clone) {
                                if let Some(chat) = app_state.chats.iter_mut().find(|c| c.contact_uid == contact_for_ping.uid) {
                                    // Clear pending flag - contact is reachable
                                    chat.has_pending_messages = false;
                                    let _ = app_state.save_to_db(&storage_clone);
                                    tracing::info!("Updated chat status for {} to active (ping successful)", contact_for_ping.uid);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to ping newly imported contact {}: {}", contact_for_ping.uid, e);
                            // Chat remains in pending status - will need manual retry or wait for contact to come online
                        }
                    }
                });
            });

            // Update import screen status
            if let Some(screen) = &mut self.import_contact_screen {
                screen.status_message = Some(format!("✓ Contact imported, ping sent!"));
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
        // Extract necessary data from chat_view_screen first
        let (message_content, contact_uid) = if let Some(chat_view) = &self.chat_view_screen {
            if chat_view.input.trim().is_empty() {
                return;
            }
            (chat_view.input.clone(), chat_view.contact_uid.clone())
        } else {
            return;
        };

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

            chat.append_message(message.clone());

            // Clear input after adding message
            if let Some(chat_view) = &mut self.chat_view_screen {
                chat_view.clear_input();
            }

            // Auto-save after sending message
            let _ = self.save_state();

            // Send message via messaging API (auto-queues on failure)
            let contact_found = self.app_state.contacts.iter().find(|c| c.uid == contact_uid).cloned();

            if let Some(contact) = contact_found {
                let transport = self.transport.clone();
                let message_clone = message.clone();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                    rt.block_on(async move {
                        // Create new MessageQueue instance (persistent SQLite allows multiple connections)
                        let mut queue = match crate::queue::MessageQueue::new_with_path("message_queue.db") {
                            Ok(q) => q,
                            Err(e) => {
                                tracing::error!("Failed to create queue: {}", e);
                                return;
                            }
                        };

                        match crate::messaging::send_message(
                            &transport,
                            &mut queue,
                            &contact,
                            &message_clone,
                            crate::queue::Priority::Normal,
                        ).await {
                            Ok(delivered) => {
                                if delivered {
                                    tracing::info!("Message sent successfully to {}", contact.uid);
                                } else {
                                    tracing::info!("Message queued for retry to {}", contact.uid);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to send/queue message to {}: {}", contact.uid, e);
                            }
                        }
                    });
                });

                if let Some(chat_view) = &mut self.chat_view_screen {
                    chat_view.set_status("Message sent".to_string());
                }
            } else {
                if let Some(chat_view) = &mut self.chat_view_screen {
                    chat_view.set_status("Error: Contact not found".to_string());
                }
            }
        }
    }

    /// Get local IP address (simplified - returns localhost for now)
    fn get_local_ip() -> String {
        // TODO: Implement actual local IP detection
        "127.0.0.1:8080".to_string()
    }
}
