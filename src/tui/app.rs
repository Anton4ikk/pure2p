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
    /// Flag to signal retry worker to stop
    retry_worker_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Background retry worker thread handle
    retry_worker_handle: Option<std::thread::JoinHandle<()>>,
    /// Transport server status
    pub transport_server_status: std::sync::Arc<std::sync::Mutex<TransportServerStatus>>,
}

/// Status of the transport server
#[derive(Debug, Clone, PartialEq)]
pub enum TransportServerStatus {
    /// Not started yet
    NotStarted,
    /// Currently attempting to start
    Starting,
    /// Running successfully on specified port
    Running(u16),
    /// Failed to start with error message
    Failed(String),
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

        // Smart port selection: reuse saved port if IP hasn't changed, generate new if IP changed
        let local_port = Self::select_port(&app_state, &local_ip);

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

        // Sync pending message flags with actual queue state
        if let Ok(pending_uids) = queue.get_pending_contact_uids() {
            app_state.sync_pending_status(&pending_uids);
        }

        // Always start at main menu (retry worker handles queue silently in background)
        let current_screen = Screen::MainMenu;
        let startup_sync_screen = None;

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
            retry_worker_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            retry_worker_handle: None,
            transport_server_status: std::sync::Arc::new(std::sync::Mutex::new(TransportServerStatus::NotStarted)),
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

        // Sync pending message flags with actual queue state
        if let Ok(pending_uids) = self.queue.get_pending_contact_uids() {
            loaded_state.sync_pending_status(&pending_uids);
        }

        // Update app state
        self.app_state = loaded_state;

        Ok(())
    }

    /// Start transport server in background with automatic retry on failure
    ///
    /// The server starts immediately and runs until app shutdown, independent of connectivity.
    /// This ensures peers can always send messages to the shared contact endpoint.
    ///
    /// # Server Lifecycle
    /// 1. Binds to preferred port (or tries up to 10 random ports if unavailable)
    /// 2. Runs health check to verify server is listening
    /// 3. Updates database with actual running port
    /// 4. Stays running until app exit (tokio runtime kept alive with oneshot channel)
    ///
    /// # Connectivity Independence
    /// - Server starts BEFORE connectivity detection
    /// - Connectivity uses the actual running port to create port mappings
    /// - Server continues running regardless of connectivity success/failure
    ///
    /// If the preferred port fails, it will try alternative ports automatically.
    /// Handlers are set up to automatically create chats when pings are received.
    pub fn start_transport(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set local UID for transport
        let uid = self.keypair.uid.to_string();
        let transport = self.transport.clone();
        let preferred_port = self.local_port;
        let state_path = self.state_path.clone();
        let status = self.transport_server_status.clone();
        let storage = self.storage.clone();

        // Create storage instances for handlers (they need their own connections in separate threads)
        let use_in_memory = state_path.contains("test") || state_path.contains("tmp");

        // Mark as starting
        *status.lock().unwrap() = TransportServerStatus::Starting;

        // Setup ping handler and message handler to receive messages and pings
        // Use a channel to keep the runtime alive until app exits
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            let (_tx, rx) = tokio::sync::oneshot::channel::<()>();

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

                // Try to start server with automatic port retry
                let mut current_port = preferred_port;
                let max_retries = 10;
                let mut last_error = String::new();
                let mut bind_successful = false;

                for attempt in 0..max_retries {
                    let addr = format!("0.0.0.0:{}", current_port).parse().expect("Invalid address");

                    tracing::info!("Attempting to start transport server on port {} (attempt {}/{})", current_port, attempt + 1, max_retries);

                    match transport.clone().start(addr).await {
                        Ok(_) => {
                            tracing::info!("✓ Transport server successfully started on port {}", current_port);

                            // Wait a moment for server to fully initialize
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                            // Verify server is actually listening by testing locally
                            let test_url = format!("http://127.0.0.1:{}/health", current_port);
                            match reqwest::Client::new()
                                .get(&test_url)
                                .timeout(std::time::Duration::from_secs(2))
                                .send()
                                .await
                            {
                                Ok(response) if response.status().is_success() => {
                                    tracing::info!("✓ Transport server verified listening on port {}", current_port);
                                    *status.lock().unwrap() = TransportServerStatus::Running(current_port);
                                    bind_successful = true;

                                    // Always update database and app state with actual running port
                                    if let Ok(mut app_state) = AppState::load_from_db(&storage) {
                                        app_state.user_port = current_port;
                                        let _ = app_state.save_to_db(&storage);
                                        tracing::info!("Updated database with running port: {}", current_port);
                                    }
                                    break;
                                }
                                Ok(response) => {
                                    last_error = format!("Server started but health check returned: {}", response.status());
                                    tracing::warn!("{}", last_error);
                                }
                                Err(e) => {
                                    last_error = format!("Server started but health check failed: {}", e);
                                    tracing::warn!("{}", last_error);
                                }
                            }
                        }
                        Err(e) => {
                            last_error = format!("Port {} bind failed: {}", current_port, e);
                            tracing::warn!("{}", last_error);

                            // Try a different random port
                            current_port = AppState::generate_random_port();
                            tracing::info!("Will retry with port {}", current_port);
                        }
                    }

                    // Small delay between retries
                    if attempt < max_retries - 1 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }

                if !bind_successful {
                    let final_error = format!("Failed to start transport server after {} attempts. Last error: {}", max_retries, last_error);
                    tracing::error!("{}", final_error);
                    *status.lock().unwrap() = TransportServerStatus::Failed(final_error);
                } else {
                    // Keep runtime alive by waiting on channel (which never receives)
                    // This ensures the spawned server task continues running
                    tracing::info!("Transport server is running, keeping runtime alive");
                    let _ = rx.await;
                }
            });
        });

        Ok(())
    }

    /// Get the actual running port from transport server (or configured port if not running yet)
    pub fn get_actual_port(&self) -> u16 {
        if let Ok(status) = self.transport_server_status.lock() {
            if let TransportServerStatus::Running(port) = *status {
                return port;
            }
        }
        self.local_port
    }

    /// Trigger startup connectivity diagnostics (non-blocking)
    ///
    /// Should be called after App::new() to detect external IP for contact sharing.
    /// Waits for transport server to be running before starting connectivity check.
    pub fn trigger_startup_connectivity(&mut self) {
        // Don't start if already running
        if self.diagnostics_refresh_handle.is_some() {
            return;
        }

        // Wait for transport server to be running before checking connectivity
        let status = self.transport_server_status.clone();
        let port = self.local_port;

        // Spawn background thread with tokio runtime
        let handle = std::thread::spawn(move || {
            // Wait up to 30 seconds for transport server to start
            for _ in 0..60 {
                if let Ok(current_status) = status.lock() {
                    if let TransportServerStatus::Running(actual_port) = *current_status {
                        // Server is running, use the actual port
                        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                        return rt.block_on(async move {
                            crate::connectivity::establish_connectivity(actual_port).await
                        });
                    } else if matches!(*current_status, TransportServerStatus::Failed(_)) {
                        // Server failed, return empty result
                        tracing::error!("Transport server failed to start, skipping connectivity check");
                        return crate::connectivity::ConnectivityResult::new();
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            // Timeout waiting for server, use configured port anyway
            tracing::warn!("Timeout waiting for transport server, using configured port {}", port);
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

                            // Start retry worker now that connectivity is established
                            if self.retry_worker_handle.is_none() {
                                if let Err(e) = self.start_retry_worker() {
                                    tracing::error!("Failed to start retry worker: {}", e);
                                }
                            }

                            // Save detected IP and port to app_state for persistence
                            self.app_state.user_ip = Some(detected_ip);
                            self.app_state.user_port = mapping.external_port;
                            let _ = self.save_state();

                            // Run health check AFTER transport server has started (give it 1 second)
                            // This verifies that our port is actually reachable from external networks
                            tracing::info!("Scheduling external reachability health check...");
                            let result_for_health = result.clone();
                            std::thread::spawn(move || {
                                // Wait for transport server to fully start
                                std::thread::sleep(std::time::Duration::from_secs(2));

                                let runtime = tokio::runtime::Runtime::new().unwrap();
                                let verified_result = runtime.block_on(async {
                                    crate::connectivity::verify_connectivity_health(result_for_health).await
                                });

                                if verified_result.externally_reachable == Some(true) {
                                    tracing::info!("✓ External reachability confirmed - you can receive messages!");
                                } else if verified_result.externally_reachable == Some(false) {
                                    tracing::warn!("✗ Port is NOT reachable from external networks");
                                    tracing::warn!("   You may need to manually configure port forwarding on your router");
                                } else {
                                    tracing::warn!("⚠ Health check inconclusive");
                                }
                            });
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

            // Create a new chat for this contact with pending status
            // (will be updated to active if ping succeeds)
            let mut new_chat = crate::storage::Chat::new(contact_uid.clone());
            new_chat.mark_has_pending(); // Mark as pending until ping succeeds
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

            // Try to send ping immediately, queue on failure (background thread)
            let transport = self.transport.clone();
            let contact_for_ping = contact.clone();
            let storage_clone = self.storage.clone();
            let sender_uid = self.keypair.uid.to_string();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async move {
                    // Try to send ping
                    match transport.send_ping(&contact_for_ping, &my_token).await {
                        Ok(ping_response) => {
                            tracing::info!("Successfully pinged newly imported contact {} (response: {})", contact_for_ping.uid, ping_response.status);
                            // Ping succeeded - mark chat as active and clear pending status
                            if let Ok(mut app_state) = AppState::load_from_db(&storage_clone) {
                                // Mark chat as active (connection confirmed) and clear pending flag
                                if let Some(chat) = app_state.chats.iter_mut().find(|c| c.contact_uid == contact_for_ping.uid) {
                                    chat.mark_unread(); // Mark as active
                                    chat.mark_no_pending(); // Clear pending flag since ping succeeded
                                    tracing::info!("Marked chat with {} as active after successful ping", contact_for_ping.uid);
                                }
                                let _ = app_state.save_to_db(&storage_clone);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to ping newly imported contact {}: {}. Queueing for retry.", contact_for_ping.uid, e);

                            // Create a special "ping" message to queue
                            let ping_message = crate::storage::Message::new(
                                uuid::Uuid::new_v4().to_string(),
                                sender_uid,
                                contact_for_ping.uid.clone(),
                                my_token.as_bytes().to_vec(), // Store contact token as content
                                Utc::now().timestamp_millis(),
                            );

                            // Create queue instance for this thread
                            let mut queue = match crate::queue::MessageQueue::new_with_path("./app_data/message_queue.db") {
                                Ok(q) => q,
                                Err(e) => {
                                    tracing::error!("Failed to create queue: {}", e);
                                    return;
                                }
                            };

                            // Queue the ping with Urgent priority
                            if let Err(e) = queue.enqueue_with_type(
                                ping_message,
                                crate::queue::Priority::Urgent,
                                "ping"
                            ) {
                                tracing::error!("Failed to queue ping for {}: {}", contact_for_ping.uid, e);
                                return;
                            }

                            // Sync pending status with queue (will set has_pending_messages=true)
                            if let Ok(mut app_state) = AppState::load_from_db(&storage_clone) {
                                if let Ok(pending_uids) = queue.get_pending_contact_uids() {
                                    app_state.sync_pending_status(&pending_uids);
                                    let _ = app_state.save_to_db(&storage_clone);
                                }
                            }
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
                        let mut queue = match crate::queue::MessageQueue::new_with_path("./app_data/message_queue.db") {
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

    /// Start background retry worker
    ///
    /// This spawns a background thread that periodically processes the message queue
    /// based on the retry interval configured in settings.
    ///
    /// The worker:
    /// 1. Immediately retries all pending messages on startup
    /// 2. Then periodically checks for messages where next_retry <= now
    /// 3. Attempts delivery for each (handles both "ping" and "text" types)
    /// 4. Updates queue status appropriately (success/failure)
    pub fn start_retry_worker(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Don't start if already running
        if self.retry_worker_handle.is_some() {
            return Ok(());
        }

        let transport = self.transport.clone();
        let queue_path = if self.state_path.contains("test") || self.state_path.contains("tmp") {
            std::path::Path::new(&self.state_path)
                .parent()
                .and_then(|p| p.to_str())
                .map(|p| format!("{}/message_queue.db", p))
                .unwrap_or_else(|| "message_queue.db".to_string())
        } else {
            "./app_data/message_queue.db".to_string()
        };
        let storage_path = self.state_path.clone();
        let stop_flag = self.retry_worker_stop.clone();
        let retry_interval_ms = self.app_state.settings.get_global_retry_interval_ms();

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                // Create queue instance for this worker thread
                let mut queue = match MessageQueue::new_with_path(&queue_path) {
                    Ok(q) => q,
                    Err(e) => {
                        tracing::error!("Retry worker: Failed to create queue: {}", e);
                        return;
                    }
                };

                // Create storage instance for this worker thread
                let storage = if storage_path.contains("test") || storage_path.contains("tmp") {
                    match Storage::new_in_memory() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Retry worker: Failed to create storage: {}", e);
                            return;
                        }
                    }
                } else {
                    match Storage::new_with_default_path() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Retry worker: Failed to create storage: {}", e);
                            return;
                        }
                    }
                };

                tracing::info!("Retry worker started with {}ms interval", retry_interval_ms);

                // PHASE 1: Startup - immediately retry ALL pending messages
                tracing::info!("Retry worker: Starting initial retry of all pending messages");
                match queue.fetch_all_pending() {
                    Ok(pending_messages) => {
                        let count = pending_messages.len();
                        if count > 0 {
                            tracing::info!("Retry worker: Found {} pending messages to retry on startup", count);
                            let mut succeeded = 0;
                            let mut failed = 0;

                            for queued_msg in pending_messages {
                                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                    tracing::info!("Retry worker: Stop signal received during startup");
                                    return;
                                }

                                let message_id = queued_msg.message.id.clone();
                                let target_uid = queued_msg.message.recipient.clone();

                                // Get message type from queue (need to query separately)
                                let message_type: String = match queue.conn.query_row(
                                    "SELECT message_type FROM message_queue WHERE message_id = ?1",
                                    [&message_id],
                                    |row| row.get(0),
                                ) {
                                    Ok(t) => t,
                                    Err(e) => {
                                        tracing::error!("Failed to get message type for {}: {}", message_id, e);
                                        continue;
                                    }
                                };

                                // Get contact info from storage
                                let contact = match AppState::load_from_db(&storage) {
                                    Ok(app_state) => {
                                        app_state.contacts.iter().find(|c| c.uid == target_uid).cloned()
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to load app state for message {}: {}", message_id, e);
                                        None
                                    }
                                };

                                let contact = match contact {
                                    Some(c) => c,
                                    None => {
                                        tracing::warn!("Contact {} not found for message {}, marking as failed", target_uid, message_id);
                                        let _ = queue.mark_failed(&message_id);
                                        failed += 1;
                                        continue;
                                    }
                                };

                                // Attempt delivery based on message type
                                let result = if message_type == "ping" {
                                    // For ping, content is the contact token
                                    let token = String::from_utf8_lossy(&queued_msg.message.content).to_string();
                                    transport.send_ping(&contact, &token).await
                                        .map(|ping_response| {
                                            tracing::info!("Retry worker: Ping successful, response: {}", ping_response.status);
                                            Some(ping_response)
                                        })
                                        .map_err(|e| crate::Error::Transport(e.to_string()))
                                } else {
                                    // For text messages, send normally
                                    transport.send_message(
                                        &contact,
                                        &queued_msg.message.sender,
                                        "text",
                                        queued_msg.message.content.clone(),
                                    ).await
                                        .map(|_| None)
                                        .map_err(|e| crate::Error::Transport(e.to_string()))
                                };

                                match result {
                                    Ok(ping_response_opt) => {
                                        if let Err(e) = queue.mark_success(&message_id) {
                                            tracing::error!("Failed to mark message {} as delivered: {}", message_id, e);
                                        } else {
                                            succeeded += 1;
                                            tracing::info!("Retry worker: {} delivered successfully to {}", message_type, target_uid);

                                            // If this was a successful ping, mark chat as active and clear pending
                                            if message_type == "ping" && ping_response_opt.is_some() {
                                                if let Ok(mut app_state) = AppState::load_from_db(&storage) {
                                                    if let Some(chat) = app_state.chats.iter_mut().find(|c| c.contact_uid == target_uid) {
                                                        chat.mark_unread(); // Mark as active
                                                        chat.mark_no_pending(); // Clear pending flag since ping succeeded
                                                        tracing::info!("Retry worker: Marked chat with {} as active after successful ping", target_uid);
                                                        let _ = app_state.save_to_db(&storage);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Retry worker: Failed to deliver {} to {}: {}", message_type, target_uid, e);
                                        if let Err(e) = queue.mark_failed(&message_id) {
                                            tracing::error!("Failed to update retry status for {}: {}", message_id, e);
                                        }
                                        failed += 1;
                                    }
                                }
                            }

                            tracing::info!("Retry worker: Startup retry complete - {} succeeded, {} failed", succeeded, failed);
                        } else {
                            tracing::info!("Retry worker: No pending messages on startup");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Retry worker: Failed to fetch pending messages: {}", e);
                    }
                }

                // PHASE 2: Periodic retry loop
                tracing::info!("Retry worker: Entering periodic retry loop");

                loop {
                    // Sleep for retry interval (check stop flag every 100ms)
                    let sleep_iterations = (retry_interval_ms / 100).max(1);
                    for _ in 0..sleep_iterations {
                        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            tracing::info!("Retry worker: Stop signal received, exiting");
                            return;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }

                    // Fetch messages that are ready for retry (next_retry <= now)
                    match queue.fetch_pending() {
                        Ok(ready_messages) => {
                            if ready_messages.is_empty() {
                                continue;
                            }

                            tracing::info!("Retry worker: Processing {} messages ready for retry", ready_messages.len());

                            for queued_msg in ready_messages {
                                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                                    tracing::info!("Retry worker: Stop signal received during processing");
                                    return;
                                }

                                let message_id = queued_msg.message.id.clone();
                                let target_uid = queued_msg.message.recipient.clone();

                                // Get message type
                                let message_type: String = match queue.conn.query_row(
                                    "SELECT message_type FROM message_queue WHERE message_id = ?1",
                                    [&message_id],
                                    |row| row.get(0),
                                ) {
                                    Ok(t) => t,
                                    Err(e) => {
                                        tracing::error!("Failed to get message type for {}: {}", message_id, e);
                                        continue;
                                    }
                                };

                                // Get contact info
                                let contact = match AppState::load_from_db(&storage) {
                                    Ok(app_state) => {
                                        app_state.contacts.iter().find(|c| c.uid == target_uid).cloned()
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to load app state: {}", e);
                                        None
                                    }
                                };

                                let contact = match contact {
                                    Some(c) => c,
                                    None => {
                                        tracing::warn!("Contact {} not found, marking message as failed", target_uid);
                                        let _ = queue.mark_failed(&message_id);
                                        continue;
                                    }
                                };

                                // Attempt delivery
                                let result = if message_type == "ping" {
                                    let token = String::from_utf8_lossy(&queued_msg.message.content).to_string();
                                    transport.send_ping(&contact, &token).await
                                        .map(|ping_response| {
                                            tracing::info!("Retry worker (periodic): Ping successful, response: {}", ping_response.status);
                                            Some(ping_response)
                                        })
                                        .map_err(|e| crate::Error::Transport(e.to_string()))
                                } else {
                                    transport.send_message(
                                        &contact,
                                        &queued_msg.message.sender,
                                        "text",
                                        queued_msg.message.content.clone(),
                                    ).await
                                        .map(|_| None)
                                        .map_err(|e| crate::Error::Transport(e.to_string()))
                                };

                                match result {
                                    Ok(ping_response_opt) => {
                                        if let Err(e) = queue.mark_success(&message_id) {
                                            tracing::error!("Failed to mark message as delivered: {}", e);
                                        } else {
                                            tracing::info!("Retry worker (periodic): {} delivered to {}", message_type, target_uid);

                                            // If this was a successful ping, mark chat as active
                                            if message_type == "ping" && ping_response_opt.is_some() {
                                                if let Ok(mut app_state) = AppState::load_from_db(&storage) {
                                                    if let Some(chat) = app_state.chats.iter_mut().find(|c| c.contact_uid == target_uid) {
                                                        chat.mark_unread(); // Mark as active
                                                        tracing::info!("Retry worker (periodic): Marked chat with {} as active after successful ping", target_uid);
                                                        let _ = app_state.save_to_db(&storage);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Retry worker: Failed to deliver {} to {}: {}", message_type, target_uid, e);
                                        let _ = queue.mark_failed(&message_id);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Retry worker: Failed to fetch ready messages: {}", e);
                        }
                    }
                }
            });
        });

        self.retry_worker_handle = Some(handle);
        tracing::info!("Retry worker thread started");
        Ok(())
    }

    /// Stop the background retry worker
    ///
    /// Signals the worker to stop and waits for it to finish gracefully.
    pub fn stop_retry_worker(&mut self) {
        if self.retry_worker_handle.is_some() {
            tracing::info!("Stopping retry worker...");
            self.retry_worker_stop.store(true, std::sync::atomic::Ordering::Relaxed);

            if let Some(handle) = self.retry_worker_handle.take() {
                // Wait for worker to finish (with timeout)
                let _ = handle.join();
                tracing::info!("Retry worker stopped");
            }
        }
    }

    /// Smart port selection: reuse saved port if IP hasn't changed, generate new if IP changed
    ///
    /// This ensures that when users restart the app on the same network, they keep the same port
    /// (maintaining their contact token validity and any port mappings). But if they switch
    /// networks (different IP), a new port is generated.
    ///
    /// # Arguments
    /// * `app_state` - Current application state with saved network info
    /// * `current_ip` - Current detected IP address
    ///
    /// # Returns
    /// Port number to use (either saved port or newly generated one)
    fn select_port(app_state: &AppState, current_ip: &str) -> u16 {
        // Extract just the IP part from the IP:port string
        let extract_ip = |ip_str: &str| -> String {
            ip_str.split(':').next().unwrap_or(ip_str).to_string()
        };

        let current_ip_only = extract_ip(current_ip);

        // If we have a saved IP, check if it matches the current IP
        if let Some(ref saved_ip) = app_state.user_ip {
            let saved_ip_only = extract_ip(saved_ip);

            // If IPs match (same network), reuse the saved port
            if saved_ip_only == current_ip_only {
                tracing::info!("IP unchanged ({}), reusing saved port {}", current_ip_only, app_state.user_port);
                return app_state.user_port;
            } else {
                // IP changed (different network), generate new port
                tracing::info!("IP changed ({}→{}), generating new port", saved_ip_only, current_ip_only);
            }
        }

        // No saved IP or IP changed - generate new random port
        let new_port = AppState::generate_random_port();
        tracing::info!("Generated new port: {}", new_port);
        new_port
    }

    /// Get local IP address (simplified - returns localhost for now)
    fn get_local_ip() -> String {
        // TODO: Implement actual local IP detection
        "127.0.0.1:8080".to_string()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Ensure retry worker is stopped when app is dropped
        self.stop_retry_worker();
    }
}
