//! Pure2P TUI (Terminal User Interface)
//!
//! A terminal-based user interface for Pure2P messaging.

use arboard::Clipboard;
use chrono::{DateTime, Duration, Utc};
use uuid;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pure2p::crypto::KeyPair;
use pure2p::storage::{generate_contact_token, parse_contact_token, AppState, Chat, Contact, Message};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::fs;
use std::io;

/// Application screens
#[derive(Debug, Clone, PartialEq, Eq)]
enum Screen {
    StartupSync,
    MainMenu,
    ShareContact,
    ImportContact,
    ChatList,
    ChatView,
    Settings,
}

/// Main menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuItem {
    ChatList,
    ShareContact,
    ImportContact,
    Settings,
    Exit,
}

impl MenuItem {
    fn all() -> Vec<Self> {
        vec![
            Self::ChatList,
            Self::ShareContact,
            Self::ImportContact,
            Self::Settings,
            Self::Exit,
        ]
    }

    fn label(&self) -> &str {
        match self {
            Self::ChatList => "Chat List",
            Self::ShareContact => "Share Contact",
            Self::ImportContact => "Import Contact",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::ChatList => "View and manage your conversations",
            Self::ShareContact => "Generate and share your contact token",
            Self::ImportContact => "Import a contact from their token",
            Self::Settings => "Configure application settings",
            Self::Exit => "Exit Pure2P",
        }
    }
}

/// Share Contact screen state
struct ShareContactScreen {
    /// Generated contact token
    token: String,
    /// Token expiry timestamp
    expiry: DateTime<Utc>,
    /// Status message (for copy/save feedback)
    status_message: Option<String>,
}

/// Import Contact screen state
struct ImportContactScreen {
    /// Input buffer for token
    input: String,
    /// Parsed contact (if valid)
    parsed_contact: Option<Contact>,
    /// Status message
    status_message: Option<String>,
    /// Whether the status is an error
    is_error: bool,
}

/// Chat List screen state
struct ChatListScreen {
    /// Selected chat index
    selected_index: usize,
    /// Status message
    status_message: Option<String>,
    /// Confirmation popup state
    show_delete_confirmation: bool,
    /// Index of chat pending deletion
    pending_delete_index: Option<usize>,
}

impl ChatListScreen {
    fn new() -> Self {
        Self {
            selected_index: 0,
            status_message: None,
            show_delete_confirmation: false,
            pending_delete_index: None,
        }
    }

    fn next(&mut self, chat_count: usize) {
        if chat_count > 0 {
            self.selected_index = (self.selected_index + 1) % chat_count;
        }
    }

    fn previous(&mut self, chat_count: usize) {
        if chat_count > 0 {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = chat_count - 1;
            }
        }
    }

    fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    fn clear_status(&mut self) {
        self.status_message = None;
    }

    fn show_delete_popup(&mut self, chat_index: usize) {
        self.show_delete_confirmation = true;
        self.pending_delete_index = Some(chat_index);
    }

    fn hide_delete_popup(&mut self) {
        self.show_delete_confirmation = false;
        self.pending_delete_index = None;
    }
}

/// Chat View screen state
struct ChatViewScreen {
    /// UID of the contact we're chatting with
    contact_uid: String,
    /// Input buffer for message composition
    input: String,
    /// Scroll offset for message history
    scroll_offset: usize,
    /// Status message
    status_message: Option<String>,
}

impl ChatViewScreen {
    fn new(contact_uid: String) -> Self {
        Self {
            contact_uid,
            input: String::new(),
            scroll_offset: 0,
            status_message: None,
        }
    }

    fn add_char(&mut self, c: char) {
        self.input.push(c);
    }

    fn backspace(&mut self) {
        self.input.pop();
    }

    fn clear_input(&mut self) {
        self.input.clear();
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self, max_offset: usize) {
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }

    fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }
}

/// Settings screen state
struct SettingsScreen {
    /// Input buffer for retry interval
    retry_interval_input: String,
    /// Currently selected field (0 = retry interval, can extend for more fields)
    selected_field: usize,
    /// Status/confirmation message
    status_message: Option<String>,
    /// Whether status is an error
    is_error: bool,
    /// Settings path for saving
    settings_path: String,
}

impl SettingsScreen {
    fn new(settings_path: String) -> Self {
        // Load current settings to populate defaults
        let settings = pure2p::storage::Settings::load(&settings_path).unwrap_or_default();

        Self {
            retry_interval_input: settings.retry_interval_minutes.to_string(),
            selected_field: 0,
            status_message: Some("Edit retry interval and press Enter to save".to_string()),
            is_error: false,
            settings_path,
        }
    }

    fn add_char(&mut self, c: char) {
        // Only allow digits for retry interval
        if c.is_ascii_digit() {
            self.retry_interval_input.push(c);
        }
    }

    fn backspace(&mut self) {
        self.retry_interval_input.pop();
    }

    fn clear_input(&mut self) {
        self.retry_interval_input.clear();
    }

    fn validate_and_save(&mut self) -> bool {
        // Validate input
        if self.retry_interval_input.is_empty() {
            self.status_message = Some("Error: Retry interval cannot be empty".to_string());
            self.is_error = true;
            return false;
        }

        match self.retry_interval_input.parse::<u32>() {
            Ok(minutes) if minutes > 0 && minutes <= 1440 => {
                // Valid range: 1 minute to 24 hours (1440 minutes)
                match self.save_settings(minutes) {
                    Ok(_) => {
                        self.status_message = Some(format!("✓ Saved! Retry interval set to {} minutes", minutes));
                        self.is_error = false;
                        true
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Error saving: {}", e));
                        self.is_error = true;
                        false
                    }
                }
            }
            Ok(minutes) if minutes == 0 => {
                self.status_message = Some("Error: Retry interval must be at least 1 minute".to_string());
                self.is_error = true;
                false
            }
            Ok(_) => {
                self.status_message = Some("Error: Retry interval cannot exceed 1440 minutes (24 hours)".to_string());
                self.is_error = true;
                false
            }
            Err(_) => {
                self.status_message = Some("Error: Invalid number".to_string());
                self.is_error = true;
                false
            }
        }
    }

    fn save_settings(&mut self, retry_interval_minutes: u32) -> Result<(), Box<dyn std::error::Error>> {
        // Load existing settings
        let mut settings = pure2p::storage::Settings::load(&self.settings_path)?;

        // Update retry interval
        settings.retry_interval_minutes = retry_interval_minutes;
        settings.global_retry_interval_ms = (retry_interval_minutes as u64) * 60 * 1000;

        // Save settings
        settings.save(&self.settings_path)?;

        Ok(())
    }
}

/// Startup Sync screen state
struct StartupSyncScreen {
    /// Total pending messages to sync
    total_messages: usize,
    /// Number of successfully delivered messages
    succeeded: usize,
    /// Number of failed deliveries
    failed: usize,
    /// Current message being processed (for progress bar)
    current: usize,
    /// Whether sync is complete
    is_complete: bool,
    /// Timestamp when sync started
    start_time: std::time::Instant,
}

impl StartupSyncScreen {
    fn new(total_messages: usize) -> Self {
        Self {
            total_messages,
            succeeded: 0,
            failed: 0,
            current: 0,
            is_complete: total_messages == 0, // Complete immediately if no messages
            start_time: std::time::Instant::now(),
        }
    }

    fn process_message(&mut self, success: bool) {
        if success {
            self.succeeded += 1;
        } else {
            self.failed += 1;
        }
        self.current += 1;

        if self.current >= self.total_messages {
            self.is_complete = true;
        }
    }

    fn get_progress_percentage(&self) -> u16 {
        if self.total_messages == 0 {
            100
        } else {
            ((self.current as f64 / self.total_messages as f64) * 100.0) as u16
        }
    }

    fn get_elapsed_time(&self) -> String {
        let elapsed = self.start_time.elapsed();
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

impl ImportContactScreen {
    fn new() -> Self {
        Self {
            input: String::new(),
            parsed_contact: None,
            status_message: Some("Paste contact token and press Enter to import".to_string()),
            is_error: false,
        }
    }

    fn add_char(&mut self, c: char) {
        self.input.push(c);
    }

    fn backspace(&mut self) {
        self.input.pop();
    }

    fn clear(&mut self) {
        self.input.clear();
        self.parsed_contact = None;
        self.status_message = Some("Input cleared. Paste contact token and press Enter".to_string());
        self.is_error = false;
    }

    fn paste_from_clipboard(&mut self) {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_text() {
                    Ok(text) => {
                        self.input = text.trim().to_string();
                        self.status_message = Some("Pasted from clipboard. Press Enter to import".to_string());
                        self.is_error = false;
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to paste: {}", e));
                        self.is_error = true;
                    }
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Clipboard error: {}", e));
                self.is_error = true;
            }
        }
    }

    fn parse_token(&mut self) {
        if self.input.is_empty() {
            self.status_message = Some("Error: Token is empty".to_string());
            self.is_error = true;
            return;
        }

        match parse_contact_token(&self.input) {
            Ok(contact) => {
                self.parsed_contact = Some(contact.clone());
                self.status_message = Some(format!(
                    "✓ Valid token! Contact: {} ({})",
                    &contact.uid[..16],
                    contact.ip
                ));
                self.is_error = false;
            }
            Err(e) => {
                self.parsed_contact = None;
                self.status_message = Some(format!("Error parsing token: {}", e));
                self.is_error = true;
            }
        }
    }

    fn get_contact(&self) -> Option<&Contact> {
        self.parsed_contact.as_ref()
    }
}

impl ShareContactScreen {
    fn new(keypair: &KeyPair, local_ip: &str) -> Self {
        // Default: 30 days expiry
        let expiry = Utc::now() + Duration::days(30);
        let token = generate_contact_token(local_ip, &keypair.public_key, expiry);

        Self {
            token,
            expiry,
            status_message: None,
        }
    }

    fn copy_to_clipboard(&mut self) {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.set_text(&self.token) {
                    Ok(_) => self.status_message = Some("Copied to clipboard!".to_string()),
                    Err(e) => self.status_message = Some(format!("Copy failed: {}", e)),
                }
            }
            Err(e) => self.status_message = Some(format!("Clipboard error: {}", e)),
        }
    }

    fn save_to_file(&mut self) {
        let filename = format!("contact_token_{}.txt", Utc::now().format("%Y%m%d_%H%M%S"));
        match fs::write(&filename, &self.token) {
            Ok(_) => self.status_message = Some(format!("Saved to {}", filename)),
            Err(e) => self.status_message = Some(format!("Save failed: {}", e)),
        }
    }
}

/// Application state
struct App {
    /// Current screen
    current_screen: Screen,
    /// Currently selected menu item
    selected_index: usize,
    /// Menu items
    menu_items: Vec<MenuItem>,
    /// User's keypair
    keypair: KeyPair,
    /// Local IP address
    local_ip: String,
    /// Should quit
    should_quit: bool,
    /// Application state (chats, contacts, settings)
    app_state: AppState,
    /// Share contact screen (when active)
    share_contact_screen: Option<ShareContactScreen>,
    /// Import contact screen (when active)
    import_contact_screen: Option<ImportContactScreen>,
    /// Chat list screen (when active)
    chat_list_screen: Option<ChatListScreen>,
    /// Chat view screen (when active)
    chat_view_screen: Option<ChatViewScreen>,
    /// Settings screen (when active)
    settings_screen: Option<SettingsScreen>,
    /// Startup sync screen (when active)
    startup_sync_screen: Option<StartupSyncScreen>,
}

impl App {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let keypair = KeyPair::generate()?;
        let local_ip = get_local_ip();
        let app_state = AppState::new();

        // Simulate checking for pending messages
        // In a real implementation, this would query the MessageQueue
        let pending_count = 0; // TODO: Query actual queue

        let (current_screen, startup_sync_screen) = if pending_count > 0 {
            (Screen::StartupSync, Some(StartupSyncScreen::new(pending_count)))
        } else {
            (Screen::MainMenu, None)
        };

        Ok(Self {
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
            startup_sync_screen,
        })
    }

    fn selected_item(&self) -> MenuItem {
        self.menu_items[self.selected_index]
    }

    fn next(&mut self) {
        self.selected_index = (self.selected_index + 1) % self.menu_items.len();
    }

    fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.menu_items.len() - 1;
        }
    }

    fn select(&mut self) {
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
            MenuItem::Settings => {
                self.show_settings_screen();
            }
            MenuItem::Exit => {
                self.should_quit = true;
            }
        }
    }

    fn show_chat_list_screen(&mut self) {
        self.chat_list_screen = Some(ChatListScreen::new());
        self.current_screen = Screen::ChatList;
    }

    fn show_share_contact_screen(&mut self) {
        self.share_contact_screen = Some(ShareContactScreen::new(&self.keypair, &self.local_ip));
        self.current_screen = Screen::ShareContact;
    }

    fn show_import_contact_screen(&mut self) {
        self.import_contact_screen = Some(ImportContactScreen::new());
        self.current_screen = Screen::ImportContact;
    }

    fn show_settings_screen(&mut self) {
        self.settings_screen = Some(SettingsScreen::new("settings.json".to_string()));
        self.current_screen = Screen::Settings;
    }

    fn back_to_main_menu(&mut self) {
        self.current_screen = Screen::MainMenu;
        self.share_contact_screen = None;
        self.import_contact_screen = None;
        self.chat_list_screen = None;
        self.chat_view_screen = None;
        self.settings_screen = None;
    }

    fn back_to_chat_list(&mut self) {
        self.current_screen = Screen::ChatList;
        self.chat_view_screen = None;
    }

    fn complete_startup_sync(&mut self) {
        self.current_screen = Screen::MainMenu;
        self.startup_sync_screen = None;
    }

    fn update_startup_sync(&mut self) {
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

    fn open_selected_chat(&mut self) {
        if let Some(chat_list) = &self.chat_list_screen {
            if self.app_state.chats.is_empty() {
                return;
            }

            let contact_uid = self.app_state.chats[chat_list.selected_index].contact_uid.clone();
            self.chat_view_screen = Some(ChatViewScreen::new(contact_uid));
            self.current_screen = Screen::ChatView;
        }
    }

    fn show_delete_confirmation(&mut self) {
        if let Some(chat_list) = &mut self.chat_list_screen {
            if self.app_state.chats.is_empty() {
                return;
            }
            chat_list.show_delete_popup(chat_list.selected_index);
        }
    }

    fn confirm_delete_chat(&mut self) {
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

    fn cancel_delete_chat(&mut self) {
        if let Some(screen) = &mut self.chat_list_screen {
            screen.hide_delete_popup();
        }
    }

    fn send_message_in_chat(&mut self) {
        if let Some(chat_view) = &mut self.chat_view_screen {
            if chat_view.input.trim().is_empty() {
                return;
            }

            let message_content = chat_view.input.clone();
            let contact_uid = chat_view.contact_uid.clone();

            // Find the chat and add the message
            if let Some(chat) = self.app_state.chats.iter_mut().find(|c| c.contact_uid == contact_uid) {
                // Create a new message
                let message = Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    sender: self.keypair.uid.to_string(),
                    recipient: contact_uid.clone(),
                    content: message_content.as_bytes().to_vec(),
                    timestamp: Utc::now().timestamp_millis(),
                    delivered: false, // Will be marked true when actually sent
                };

                chat.append_message(message);
                chat_view.clear_input();
                chat_view.set_status("Message queued for sending".to_string());

                // TODO: Actually send via transport layer
                // For now, messages are just stored locally
            }
        }
    }
}

/// Get local IP address (simplified - returns localhost for now)
fn get_local_ip() -> String {
    // TODO: Implement actual local IP detection
    "127.0.0.1:8080".to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new()?;

    // Run main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Handle startup sync screen updates
        if app.current_screen == Screen::StartupSync {
            app.update_startup_sync();

            // Check if sync is complete
            if let Some(sync) = &app.startup_sync_screen {
                if sync.is_complete {
                    // Wait a moment to show final stats
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.current_screen {
                    Screen::StartupSync => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                // Allow user to skip or dismiss when complete
                                if let Some(sync) = &app.startup_sync_screen {
                                    if sync.is_complete {
                                        app.complete_startup_sync();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::MainMenu => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous();
                            }
                            KeyCode::Enter => {
                                app.select();
                            }
                            _ => {}
                        }
                    }
                    Screen::ShareContact => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char('c') => {
                                if let Some(screen) = &mut app.share_contact_screen {
                                    screen.copy_to_clipboard();
                                }
                            }
                            KeyCode::Char('s') => {
                                if let Some(screen) = &mut app.share_contact_screen {
                                    screen.save_to_file();
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::ImportContact => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.add_char(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.backspace();
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.parse_token();
                                }
                            }
                            KeyCode::Char('v') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.paste_from_clipboard();
                                }
                            }
                            KeyCode::Delete => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.clear();
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::ChatList => {
                        // Check if delete confirmation popup is shown
                        let show_popup = app.chat_list_screen.as_ref()
                            .map(|s| s.show_delete_confirmation)
                            .unwrap_or(false);

                        if show_popup {
                            // Handle confirmation popup keys
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    app.confirm_delete_chat();
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    app.cancel_delete_chat();
                                }
                                _ => {}
                            }
                        } else {
                            // Handle normal chat list navigation
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                    app.back_to_main_menu();
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if let Some(screen) = &mut app.chat_list_screen {
                                        screen.next(app.app_state.chats.len());
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if let Some(screen) = &mut app.chat_list_screen {
                                        screen.previous(app.app_state.chats.len());
                                    }
                                }
                                KeyCode::Char('d') | KeyCode::Delete => {
                                    app.show_delete_confirmation();
                                }
                                KeyCode::Enter => {
                                    app.open_selected_chat();
                                }
                                _ => {}
                            }
                        }
                    }
                    Screen::ChatView => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_chat_list();
                            }
                            KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    screen.add_char(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    screen.backspace();
                                }
                            }
                            KeyCode::Enter => {
                                app.send_message_in_chat();
                            }
                            KeyCode::PageUp => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    screen.scroll_up();
                                }
                            }
                            KeyCode::PageDown => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    let chat = app.app_state.chats.iter()
                                        .find(|c| c.contact_uid == screen.contact_uid);
                                    let max_offset = chat.map(|c| c.messages.len().saturating_sub(10)).unwrap_or(0);
                                    screen.scroll_down(max_offset);
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::Settings => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.add_char(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.backspace();
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.validate_and_save();
                                }
                            }
                            KeyCode::Delete => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.clear_input();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::StartupSync => render_startup_sync(f, app),
        Screen::MainMenu => render_main_menu(f, app),
        Screen::ShareContact => render_share_contact(f, app),
        Screen::ImportContact => render_import_contact(f, app),
        Screen::ChatList => render_chat_list(f, app),
        Screen::ChatView => render_chat_view(f, app),
        Screen::Settings => render_settings(f, app),
    }
}

fn render_startup_sync(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.startup_sync_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(4)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Progress bar
                Constraint::Length(5),  // Stats
                Constraint::Length(3),  // Status/Help
            ])
            .split(size);

        // Title
        let title = Paragraph::new("Syncing Pending Messages")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Progress bar
        let progress_percentage = screen.get_progress_percentage();
        let progress_label = if screen.is_complete {
            format!("Complete - {} of {} messages processed", screen.current, screen.total_messages)
        } else {
            format!("Processing {} of {} messages ({}%)", screen.current, screen.total_messages, progress_percentage)
        };

        use ratatui::widgets::Gauge;
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(
                Style::default()
                    .fg(if screen.is_complete { Color::Green } else { Color::Cyan })
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(progress_percentage)
            .label(progress_label);
        f.render_widget(gauge, chunks[1]);

        // Stats
        let stats_text = vec![
            Line::from(vec![
                Span::styled("✓ Succeeded: ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{}", screen.succeeded),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("✗ Failed: ", Style::default().fg(Color::Red)),
                Span::styled(
                    format!("{}", screen.failed),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("⏱ Time: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    screen.get_elapsed_time(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let stats_widget = Paragraph::new(stats_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Statistics"));
        f.render_widget(stats_widget, chunks[2]);

        // Status/Help
        let help_text = if screen.is_complete {
            "Sync complete! Press Enter or Space to continue to main menu"
        } else {
            "Syncing messages... Please wait"
        };
        let help_color = if screen.is_complete { Color::Green } else { Color::Yellow };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(help_color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[3]);
    }
}

fn render_main_menu(f: &mut Frame, app: &App) {
    let size = f.size();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // IP display
            Constraint::Min(10),   // Menu
            Constraint::Length(3), // Help text
        ])
        .split(size);

    // Title
    let title = Paragraph::new("Pure2P - True P2P Messenger")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // IP display
    let uid_short = &app.keypair.uid.to_string()[..16];
    let ip_text = format!("Your UID: {}... | IP: {}", uid_short, app.local_ip);
    let ip_widget = Paragraph::new(ip_text)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Identity"));
    f.render_widget(ip_widget, chunks[1]);

    // Menu items
    let menu_items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let content = if i == app.selected_index {
                Line::from(vec![
                    Span::styled("→ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        item.label(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(item.label(), Style::default().fg(Color::White)),
                ])
            };
            ListItem::new(content)
        })
        .collect();

    let menu = List::new(menu_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Main Menu")
            .style(Style::default()),
    );
    f.render_widget(menu, chunks[2]);

    // Help text
    let selected = app.selected_item();
    let help_text = format!(
        "{} | Navigation: ↑↓ or j/k | Select: Enter | Quit: q/Esc",
        selected.description()
    );
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[3]);
}

fn render_share_contact(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.share_contact_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Expiry info
                Constraint::Min(5),     // Token display
                Constraint::Length(3),  // Status message
                Constraint::Length(3),  // Help text
            ])
            .split(size);

        // Title
        let title = Paragraph::new("Share Contact Token")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Expiry info
        let expiry_text = format!(
            "Expires: {} ({})",
            screen.expiry.format("%Y-%m-%d %H:%M:%S UTC"),
            format_duration_until(screen.expiry)
        );
        let expiry_widget = Paragraph::new(expiry_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Expiry"));
        f.render_widget(expiry_widget, chunks[1]);

        // Token display (wrapped and scrollable if needed)
        let token_text = Text::from(screen.token.clone());
        let token_widget = Paragraph::new(token_text)
            .style(Style::default().fg(Color::Green))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Contact Token"),
            );
        f.render_widget(token_widget, chunks[2]);

        // Status message
        let status_text = screen
            .status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let status_color = if status_text.contains("failed") || status_text.contains("error") {
            Color::Red
        } else {
            Color::Green
        };
        let status_widget = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[3]);

        // Help text
        let help_text = "c: Copy to Clipboard | s: Save to File | b/Esc: Back to Menu | q: Quit";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[4]);
    }
}

fn format_duration_until(expiry: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = expiry.signed_duration_since(now);

    if duration.num_days() > 0 {
        format!("{} days", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes", duration.num_minutes())
    } else {
        "expired".to_string()
    }
}

fn render_import_contact(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.import_contact_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(5),     // Input field
                Constraint::Length(8),  // Contact info (if parsed)
                Constraint::Length(3),  // Status message
                Constraint::Length(3),  // Help text
            ])
            .split(size);

        // Title
        let title = Paragraph::new("Import Contact")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Input field
        let input_text = Text::from(screen.input.as_str());
        let input_widget = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Contact Token"),
            );
        f.render_widget(input_widget, chunks[1]);

        // Contact info (if parsed)
        if let Some(contact) = screen.get_contact() {
            let info_lines = vec![
                Line::from(vec![
                    Span::styled("UID: ", Style::default().fg(Color::Yellow)),
                    Span::styled(&contact.uid, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("IP: ", Style::default().fg(Color::Yellow)),
                    Span::styled(&contact.ip, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Expires: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        contact.expiry.format("%Y-%m-%d %H:%M UTC").to_string(),
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        if contact.is_active { "Active" } else { "Inactive" },
                        Style::default().fg(if contact.is_active { Color::Green } else { Color::Gray }),
                    ),
                ]),
            ];

            let info_widget = Paragraph::new(info_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Contact Information"),
                );
            f.render_widget(info_widget, chunks[2]);
        } else {
            let placeholder = Paragraph::new("No contact parsed yet")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Contact Information"),
                );
            f.render_widget(placeholder, chunks[2]);
        }

        // Status message
        let status_text = screen
            .status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let status_color = if screen.is_error {
            Color::Red
        } else {
            Color::Green
        };
        let status_widget = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[3]);

        // Help text
        let help_text = "Enter: Parse | Ctrl+V: Paste | Delete: Clear | b/Esc: Back | q: Quit";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[4]);
    }
}

fn render_chat_list(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.chat_list_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(5),     // Chat list
                Constraint::Length(3),  // Status message
                Constraint::Length(3),  // Help text
            ])
            .split(size);

        // Title
        let title = Paragraph::new(format!("Chat List ({} chats)", app.app_state.chats.len()))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Chat list
        if app.app_state.chats.is_empty() {
            let empty_msg = Paragraph::new("No chats yet. Import a contact to start chatting!")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Chats"));
            f.render_widget(empty_msg, chunks[1]);
        } else {
            let chat_items: Vec<ListItem> = app
                .app_state
                .chats
                .iter()
                .enumerate()
                .map(|(i, chat)| {
                    let uid_short = &chat.contact_uid[..16.min(chat.contact_uid.len())];
                    let msg_count = chat.messages.len();

                    // Check if contact is expired
                    let contact_expired = app.app_state.contacts
                        .iter()
                        .find(|c| c.uid == chat.contact_uid)
                        .map(|c| c.is_expired())
                        .unwrap_or(false);

                    // Determine style and indicator with priority system:
                    // Priority 1: Expired contact (highest urgency)
                    // Priority 2: Pending messages (action needed)
                    // Priority 3: New/unread messages (active chat)
                    // Priority 4: Inactive/read (lowest)
                    let (style, indicator) = if contact_expired {
                        // Expired contact - highest priority, red warning
                        (Style::default().fg(Color::Red).add_modifier(Modifier::BOLD), "⚠ ")
                    } else if chat.has_pending_messages {
                        // Pending messages - highlighted in yellow with hourglass
                        (Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD), "⌛ ")
                    } else if chat.is_active {
                        // Active chat with new/unread messages - green dot
                        (Style::default().fg(Color::Green).add_modifier(Modifier::BOLD), "● ")
                    } else {
                        // Inactive chat - dimmed gray circle
                        (Style::default().fg(Color::DarkGray), "○ ")
                    };

                    let content = if i == screen.selected_index {
                        Line::from(vec![
                            Span::styled("→ ", Style::default().fg(Color::Cyan)),
                            Span::styled(indicator, style),
                            Span::styled(
                                format!("{} ({} msgs)", uid_short, msg_count),
                                style,
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(indicator, style),
                            Span::styled(
                                format!("{} ({} msgs)", uid_short, msg_count),
                                style,
                            ),
                        ])
                    };
                    ListItem::new(content)
                })
                .collect();

            let chat_list = List::new(chat_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chats (● New Messages | ⌛ Pending | ⚠ Expired | ○ Read)")
                    .style(Style::default()),
            );
            f.render_widget(chat_list, chunks[1]);
        }

        // Status message
        let status_text = screen
            .status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let status_widget = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[2]);

        // Help text
        let help_text = "↑↓/j/k: Navigate | Enter: Open | d/Del: Delete | b/Esc: Back | q: Quit";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[3]);

        // Render confirmation popup if shown
        if screen.show_delete_confirmation {
            if let Some(delete_index) = screen.pending_delete_index {
                if delete_index < app.app_state.chats.len() {
                    let chat = &app.app_state.chats[delete_index];
                    render_delete_confirmation_popup(f, size, chat);
                }
            }
        }
    }
}

fn render_delete_confirmation_popup(f: &mut Frame, area: ratatui::layout::Rect, chat: &Chat) {
    // Create a centered popup area
    let popup_width = 60;
    let popup_height = 10;

    let popup_area = ratatui::layout::Rect {
        x: area.width.saturating_sub(popup_width) / 2,
        y: area.height.saturating_sub(popup_height) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    // Create the popup layout
    let popup_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(4),  // Message
            Constraint::Length(2),  // Buttons
        ])
        .split(popup_area);

    // Clear the popup area with a background block
    let background = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .style(Style::default().bg(Color::Black));
    f.render_widget(background, popup_area);

    // Title
    let title = Paragraph::new("Confirm Delete")
        .style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, popup_chunks[0]);

    // Message
    let uid_short = &chat.contact_uid[..16.min(chat.contact_uid.len())];
    let chat_type = if chat.is_active { "active" } else { "inactive" };
    let action_text = if chat.is_active {
        "This will send a delete request to the contact\nand remove the chat locally."
    } else {
        "This will delete the chat locally only."
    };

    let message_text = vec![
        Line::from(vec![
            Span::raw("Delete "),
            Span::styled(chat_type, Style::default().fg(if chat.is_active { Color::Green } else { Color::Gray })),
            Span::raw(" chat with "),
            Span::styled(uid_short, Style::default().fg(Color::Cyan)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(Span::styled(action_text, Style::default().fg(Color::Yellow))),
    ];

    let message = Paragraph::new(message_text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(message, popup_chunks[1]);

    // Buttons
    let buttons = Paragraph::new(Line::from(vec![
        Span::styled("[Y]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("es  "),
        Span::styled("[N]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw("o"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(buttons, popup_chunks[2]);
}

fn render_chat_view(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.chat_view_screen {
        // Find the chat
        let chat = app.app_state.chats.iter()
            .find(|c| c.contact_uid == screen.contact_uid);

        if let Some(chat) = chat {
            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Min(5),     // Message history
                    Constraint::Length(3),  // Input box
                    Constraint::Length(3),  // Status/Help
                ])
                .split(size);

            // Title - show contact UID
            let uid_short = &chat.contact_uid[..16.min(chat.contact_uid.len())];
            let title = Paragraph::new(format!("Chat with {}", uid_short))
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Message history
            if chat.messages.is_empty() {
                let empty_msg = Paragraph::new("No messages yet. Type a message below and press Enter to send.")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).title("Messages"));
                f.render_widget(empty_msg, chunks[1]);
            } else {
                // Calculate visible range based on scroll offset
                let total_messages = chat.messages.len();
                let visible_height = chunks[1].height.saturating_sub(2) as usize; // Subtract borders
                let start_idx = screen.scroll_offset;
                let end_idx = (start_idx + visible_height).min(total_messages);

                let message_lines: Vec<Line> = chat.messages[start_idx..end_idx]
                    .iter()
                    .map(|msg| {
                        // Format timestamp
                        let timestamp = DateTime::from_timestamp_millis(msg.timestamp)
                            .map(|dt| dt.format("%H:%M:%S").to_string())
                            .unwrap_or_else(|| "??:??:??".to_string());

                        // Determine if message is from us or them
                        let is_from_me = msg.sender == app.keypair.uid.to_string();
                        let sender_label = if is_from_me { "You" } else { "Them" };
                        let sender_color = if is_from_me { Color::Green } else { Color::Blue };

                        // Decode message content
                        let content = String::from_utf8(msg.content.clone())
                            .unwrap_or_else(|_| "[binary data]".to_string());

                        Line::from(vec![
                            Span::styled(
                                format!("[{}] ", timestamp),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("{}: ", sender_label),
                                Style::default().fg(sender_color).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(content, Style::default().fg(Color::White)),
                        ])
                    })
                    .collect();

                let messages_widget = Paragraph::new(message_lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("Messages ({}/{})", end_idx, total_messages)),
                    );
                f.render_widget(messages_widget, chunks[1]);
            }

            // Input box
            let input_widget = Paragraph::new(screen.input.as_str())
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Type your message"),
                );
            f.render_widget(input_widget, chunks[2]);

            // Status/Help
            let help_text = if let Some(status) = &screen.status_message {
                status.clone()
            } else {
                "Enter: Send | PgUp/PgDn: Scroll | b/Esc: Back to Chat List | q: Quit".to_string()
            };
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(help, chunks[3]);
        }
    }
}

fn render_settings(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.settings_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(5),  // Retry interval field
                Constraint::Length(5),  // Help/Info
                Constraint::Length(3),  // Status message
                Constraint::Length(3),  // Help text
            ])
            .split(size);

        // Title
        let title = Paragraph::new("Settings")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Retry Interval Field
        let retry_interval_text = vec![
            Line::from(Span::styled(
                "Retry Interval (minutes)",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &screen.retry_interval_input,
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
        ];

        let retry_field = Paragraph::new(retry_interval_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Field"));
        f.render_widget(retry_field, chunks[1]);

        // Help/Info
        let info_text = vec![
            Line::from(Span::styled(
                "This controls how often the app retries sending",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "pending messages (range: 1-1440 minutes).",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let info_widget = Paragraph::new(info_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Info"));
        f.render_widget(info_widget, chunks[2]);

        // Status message
        let status_text = screen
            .status_message
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let status_color = if screen.is_error {
            Color::Red
        } else if status_text.contains("✓") {
            Color::Green
        } else {
            Color::Yellow
        };
        let status_widget = Paragraph::new(status_text)
            .style(Style::default().fg(status_color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status_widget, chunks[3]);

        // Help text
        let help_text = "Enter: Save | Delete: Clear | b/Esc: Back | q: Quit";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[4]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pure2p::storage::parse_contact_token;

    #[test]
    fn test_share_contact_screen_creation() {
        // Create a keypair for testing
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";

        // Create share contact screen
        let screen = ShareContactScreen::new(&keypair, local_ip);

        // Verify token is non-empty
        assert!(!screen.token.is_empty(), "Token should not be empty");

        // Verify expiry is in the future
        assert!(
            screen.expiry > Utc::now(),
            "Expiry should be in the future"
        );

        // Verify default expiry is approximately 30 days
        let expiry_duration = screen.expiry.signed_duration_since(Utc::now());
        assert!(
            expiry_duration.num_days() >= 29 && expiry_duration.num_days() <= 30,
            "Default expiry should be approximately 30 days, got {} days",
            expiry_duration.num_days()
        );

        // Verify no initial status message
        assert!(
            screen.status_message.is_none(),
            "Status message should be None initially"
        );
    }

    #[test]
    fn test_share_contact_screen_token_valid() {
        // Create a keypair for testing
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";

        // Create share contact screen
        let screen = ShareContactScreen::new(&keypair, local_ip);

        // Parse the token to verify it's valid
        let parsed_contact =
            parse_contact_token(&screen.token).expect("Token should be valid and parseable");

        // Verify parsed contact matches expected values
        assert_eq!(
            parsed_contact.ip, local_ip,
            "Parsed IP should match input"
        );
        assert_eq!(
            parsed_contact.pubkey,
            keypair.public_key,
            "Parsed public key should match keypair"
        );
        assert_eq!(
            parsed_contact.uid,
            keypair.uid.to_string(),
            "Parsed UID should match keypair UID"
        );
    }

    #[test]
    fn test_share_contact_screen_save_to_file() {
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let original_dir = std::env::current_dir().expect("Failed to get current dir");

        // Change to temp directory for the test
        std::env::set_current_dir(temp_dir.path()).expect("Failed to change dir");

        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";

        let mut screen = ShareContactScreen::new(&keypair, local_ip);
        let original_token = screen.token.clone();

        // Save to file
        screen.save_to_file();

        // Verify status message indicates success
        assert!(
            screen
                .status_message
                .as_ref()
                .expect("Status message should be set")
                .starts_with("Saved to"),
            "Status message should indicate successful save"
        );

        // Find the generated file
        let entries = std::fs::read_dir(temp_dir.path()).expect("Failed to read temp dir");
        let mut found_file = false;

        for entry in entries {
            let entry = entry.expect("Failed to read entry");
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if filename_str.starts_with("contact_token_") && filename_str.ends_with(".txt") {
                // Read the file contents
                let contents = std::fs::read_to_string(entry.path())
                    .expect("Failed to read saved token file");

                // Verify contents match the token
                assert_eq!(
                    contents.trim(),
                    original_token,
                    "Saved token should match original"
                );
                found_file = true;
                break;
            }
        }

        assert!(found_file, "Token file should have been created");

        // Restore original directory
        std::env::set_current_dir(original_dir).expect("Failed to restore dir");
    }

    #[test]
    fn test_format_duration_until_days() {
        let expiry = Utc::now() + Duration::days(15);
        let formatted = format_duration_until(expiry);
        // Allow for slight timing variance (14-15 days)
        assert!(
            formatted == "14 days" || formatted == "15 days",
            "Expected 14 or 15 days, got: {}",
            formatted
        );
    }

    #[test]
    fn test_format_duration_until_hours() {
        let expiry = Utc::now() + Duration::hours(12);
        let formatted = format_duration_until(expiry);
        // Allow for slight timing variance (11-12 hours)
        assert!(
            formatted == "11 hours" || formatted == "12 hours",
            "Expected 11 or 12 hours, got: {}",
            formatted
        );
    }

    #[test]
    fn test_format_duration_until_minutes() {
        let expiry = Utc::now() + Duration::minutes(45);
        let formatted = format_duration_until(expiry);
        // Allow for slight timing variance (44-45 minutes)
        assert!(
            formatted == "44 minutes" || formatted == "45 minutes",
            "Expected 44 or 45 minutes, got: {}",
            formatted
        );
    }

    #[test]
    fn test_format_duration_until_expired() {
        let expiry = Utc::now() - Duration::hours(1);
        let formatted = format_duration_until(expiry);
        assert_eq!(formatted, "expired");
    }

    #[test]
    fn test_app_initialization() {
        let app = App::new().expect("Failed to create app");

        // Verify initial state
        assert_eq!(
            app.current_screen,
            Screen::MainMenu,
            "Should start on main menu"
        );
        assert_eq!(app.selected_index, 0, "Should start with first item selected");
        assert!(!app.should_quit, "Should not be quitting initially");
        assert!(
            app.share_contact_screen.is_none(),
            "Share contact screen should not be active initially"
        );
        assert_eq!(
            app.menu_items.len(),
            5,
            "Should have 5 menu items"
        );
    }

    #[test]
    fn test_app_navigation() {
        let mut app = App::new().expect("Failed to create app");

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

        // Test wrap around
        app.next();
        assert_eq!(app.selected_index, 0, "Should wrap to beginning");

        // Test previous navigation
        app.previous();
        assert_eq!(app.selected_index, 4, "Should wrap to end");
        app.previous();
        assert_eq!(app.selected_index, 3);
    }

    #[test]
    fn test_app_show_share_contact_screen() {
        let mut app = App::new().expect("Failed to create app");

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
        assert!(screen.expiry > Utc::now());
    }

    #[test]
    fn test_app_back_to_main_menu() {
        let mut app = App::new().expect("Failed to create app");

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
    fn test_app_select_share_contact() {
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

        // Navigate to Exit item (index 4)
        app.selected_index = 4;
        assert_eq!(app.selected_item(), MenuItem::Exit);
        assert!(!app.should_quit);

        // Trigger selection
        app.select();

        // Should set quit flag
        assert!(app.should_quit);
    }

    #[test]
    fn test_token_consistency() {
        // Same keypair and IP should generate same token
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";
        let expiry = Utc::now() + Duration::days(30);

        let token1 = generate_contact_token(local_ip, &keypair.public_key, expiry);
        let token2 = generate_contact_token(local_ip, &keypair.public_key, expiry);

        assert_eq!(
            token1, token2,
            "Same inputs should generate identical tokens"
        );
    }

    #[test]
    fn test_different_keypairs_different_tokens() {
        // Different keypairs should generate different tokens
        let keypair1 = KeyPair::generate().expect("Failed to generate keypair");
        let keypair2 = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";
        let expiry = Utc::now() + Duration::days(30);

        let token1 = generate_contact_token(local_ip, &keypair1.public_key, expiry);
        let token2 = generate_contact_token(local_ip, &keypair2.public_key, expiry);

        assert_ne!(
            token1, token2,
            "Different keypairs should generate different tokens"
        );
    }

    #[test]
    fn test_import_contact_screen_creation() {
        let screen = ImportContactScreen::new();

        assert!(screen.input.is_empty(), "Input should be empty initially");
        assert!(
            screen.parsed_contact.is_none(),
            "No contact should be parsed initially"
        );
        assert!(
            screen.status_message.is_some(),
            "Should have initial status message"
        );
        assert!(!screen.is_error, "Should not be in error state initially");
    }

    #[test]
    fn test_import_contact_screen_add_char() {
        let mut screen = ImportContactScreen::new();

        screen.add_char('a');
        assert_eq!(screen.input, "a");

        screen.add_char('b');
        assert_eq!(screen.input, "ab");

        screen.add_char('c');
        assert_eq!(screen.input, "abc");
    }

    #[test]
    fn test_import_contact_screen_backspace() {
        let mut screen = ImportContactScreen::new();
        screen.input = "hello".to_string();

        screen.backspace();
        assert_eq!(screen.input, "hell");

        screen.backspace();
        assert_eq!(screen.input, "hel");

        // Backspace on empty should not panic
        screen.input.clear();
        screen.backspace();
        assert_eq!(screen.input, "");
    }

    #[test]
    fn test_import_contact_screen_clear() {
        let mut screen = ImportContactScreen::new();
        screen.input = "some token".to_string();
        screen.is_error = true;

        screen.clear();

        assert!(screen.input.is_empty(), "Input should be cleared");
        assert!(
            screen.parsed_contact.is_none(),
            "Parsed contact should be cleared"
        );
        assert!(!screen.is_error, "Error flag should be cleared");
        assert!(screen.status_message.is_some(), "Should have status message");
    }

    #[test]
    fn test_import_contact_screen_parse_empty() {
        let mut screen = ImportContactScreen::new();

        screen.parse_token();

        assert!(screen.is_error, "Should be in error state for empty input");
        assert!(
            screen.parsed_contact.is_none(),
            "Should not have parsed contact"
        );
        assert!(
            screen
                .status_message
                .as_ref()
                .unwrap()
                .contains("empty"),
            "Status should mention empty token"
        );
    }

    #[test]
    fn test_import_contact_screen_parse_invalid() {
        let mut screen = ImportContactScreen::new();
        screen.input = "invalid_token_data".to_string();

        screen.parse_token();

        assert!(screen.is_error, "Should be in error state for invalid token");
        assert!(
            screen.parsed_contact.is_none(),
            "Should not have parsed contact"
        );
        assert!(
            screen
                .status_message
                .as_ref()
                .unwrap()
                .contains("Error"),
            "Status should indicate error"
        );
    }

    #[test]
    fn test_import_contact_screen_parse_valid() {
        // Generate a valid token
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";
        let expiry = Utc::now() + Duration::days(30);
        let token = generate_contact_token(local_ip, &keypair.public_key, expiry);

        let mut screen = ImportContactScreen::new();
        screen.input = token.clone();

        screen.parse_token();

        assert!(!screen.is_error, "Should not be in error state");
        assert!(
            screen.parsed_contact.is_some(),
            "Should have parsed contact"
        );

        let contact = screen.parsed_contact.as_ref().unwrap();
        assert_eq!(contact.ip, local_ip);
        assert_eq!(contact.uid, keypair.uid.to_string());
    }

    #[test]
    fn test_import_contact_screen_get_contact() {
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let local_ip = "192.168.1.100:8080";
        let expiry = Utc::now() + Duration::days(30);
        let token = generate_contact_token(local_ip, &keypair.public_key, expiry);

        let mut screen = ImportContactScreen::new();

        // Initially no contact
        assert!(screen.get_contact().is_none());

        // Parse valid token
        screen.input = token;
        screen.parse_token();

        // Should have contact now
        assert!(screen.get_contact().is_some());
        assert_eq!(screen.get_contact().unwrap().ip, local_ip);
    }

    #[test]
    fn test_app_show_import_contact_screen() {
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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
    fn test_chat_list_screen_creation() {
        let screen = ChatListScreen::new();

        assert_eq!(screen.selected_index, 0, "Should start at index 0");
        assert!(
            screen.status_message.is_none(),
            "Should have no status message initially"
        );
    }

    #[test]
    fn test_chat_list_screen_navigation() {
        let mut screen = ChatListScreen::new();

        // Test next with 3 chats
        screen.next(3);
        assert_eq!(screen.selected_index, 1);
        screen.next(3);
        assert_eq!(screen.selected_index, 2);

        // Test wrap around
        screen.next(3);
        assert_eq!(screen.selected_index, 0, "Should wrap to beginning");

        // Test previous
        screen.previous(3);
        assert_eq!(screen.selected_index, 2, "Should wrap to end");
        screen.previous(3);
        assert_eq!(screen.selected_index, 1);

        // Test with empty list
        screen.next(0);
        assert_eq!(screen.selected_index, 1, "Should not change with empty list");
    }

    #[test]
    fn test_chat_list_screen_status() {
        let mut screen = ChatListScreen::new();

        assert!(screen.status_message.is_none());

        screen.set_status("Test status".to_string());
        assert_eq!(screen.status_message.as_ref().unwrap(), "Test status");

        screen.clear_status();
        assert!(screen.status_message.is_none());
    }

    #[test]
    fn test_app_show_chat_list_screen() {
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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
    fn test_app_delete_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add some test chats
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.add_chat("bob_uid".to_string());
        app.app_state.add_chat("charlie_uid".to_string());
        assert_eq!(app.app_state.chats.len(), 3);

        // Show chat list screen
        app.show_chat_list_screen();

        // Delete second chat (bob) - now requires confirmation
        if let Some(screen) = &mut app.chat_list_screen {
            screen.selected_index = 1;
        }
        app.show_delete_confirmation();
        app.confirm_delete_chat();

        // Should have 2 chats left
        assert_eq!(app.app_state.chats.len(), 2);
        assert_eq!(app.app_state.chats[0].contact_uid, "alice_uid");
        assert_eq!(app.app_state.chats[1].contact_uid, "charlie_uid");

        // Status message should be set
        assert!(app.chat_list_screen.as_ref().unwrap().status_message.is_some());
    }

    #[test]
    fn test_app_delete_last_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add one chat
        app.app_state.add_chat("alice_uid".to_string());
        assert_eq!(app.app_state.chats.len(), 1);

        // Show chat list screen
        app.show_chat_list_screen();

        // Delete the chat - now requires confirmation
        app.show_delete_confirmation();
        app.confirm_delete_chat();

        // Should have no chats
        assert_eq!(app.app_state.chats.len(), 0);
    }

    #[test]
    fn test_app_delete_adjusts_selection() {
        let mut app = App::new().expect("Failed to create app");

        // Add chats
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.add_chat("bob_uid".to_string());

        // Show chat list and select last chat
        app.show_chat_list_screen();
        if let Some(screen) = &mut app.chat_list_screen {
            screen.selected_index = 1;
        }

        // Delete it - now requires confirmation
        app.show_delete_confirmation();
        app.confirm_delete_chat();

        // Selection should be adjusted to 0
        assert_eq!(app.chat_list_screen.as_ref().unwrap().selected_index, 0);
    }

    #[test]
    fn test_app_state_initialized() {
        let app = App::new().expect("Failed to create app");

        assert!(app.app_state.chats.is_empty(), "Should have no chats initially");
        assert!(app.app_state.contacts.is_empty(), "Should have no contacts initially");
    }

    #[test]
    fn test_menu_items_updated() {
        // Verify ChatList is first item
        assert_eq!(MenuItem::ChatList.label(), "Chat List");
        assert_eq!(
            MenuItem::ChatList.description(),
            "View and manage your conversations"
        );

        // Verify menu has 5 items now
        let items = MenuItem::all();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0], MenuItem::ChatList);
        assert_eq!(items[1], MenuItem::ShareContact);
    }

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

    #[test]
    fn test_app_open_selected_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add a chat
        app.app_state.add_chat("alice_uid".to_string());

        // Show chat list
        app.show_chat_list_screen();

        // Open the chat
        app.open_selected_chat();

        // Should be on chat view screen
        assert_eq!(app.current_screen, Screen::ChatView);
        assert!(app.chat_view_screen.is_some());
        assert_eq!(
            app.chat_view_screen.as_ref().unwrap().contact_uid,
            "alice_uid"
        );
    }

    #[test]
    fn test_app_back_to_chat_list() {
        let mut app = App::new().expect("Failed to create app");

        // Add chat and open it
        app.app_state.add_chat("alice_uid".to_string());
        app.show_chat_list_screen();
        app.open_selected_chat();

        assert_eq!(app.current_screen, Screen::ChatView);

        // Go back
        app.back_to_chat_list();

        assert_eq!(app.current_screen, Screen::ChatList);
        assert!(app.chat_view_screen.is_none());
    }

    #[test]
    fn test_app_send_message() {
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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

    #[test]
    fn test_chat_list_screen_delete_popup() {
        let mut screen = ChatListScreen::new();

        // Initially no popup shown
        assert!(!screen.show_delete_confirmation);
        assert!(screen.pending_delete_index.is_none());

        // Show delete popup
        screen.show_delete_popup(2);
        assert!(screen.show_delete_confirmation);
        assert_eq!(screen.pending_delete_index, Some(2));

        // Hide delete popup
        screen.hide_delete_popup();
        assert!(!screen.show_delete_confirmation);
        assert!(screen.pending_delete_index.is_none());
    }

    #[test]
    fn test_app_show_delete_confirmation() {
        let mut app = App::new().expect("Failed to create app");

        // Add some chats
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.add_chat("bob_uid".to_string());

        // Show chat list
        app.show_chat_list_screen();

        // Initially no popup
        assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

        // Show delete confirmation
        app.show_delete_confirmation();

        // Popup should be shown
        assert!(app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
        assert_eq!(app.chat_list_screen.as_ref().unwrap().pending_delete_index, Some(0));
    }

    #[test]
    fn test_app_cancel_delete_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add chat
        app.app_state.add_chat("alice_uid".to_string());

        // Show chat list and delete confirmation
        app.show_chat_list_screen();
        app.show_delete_confirmation();

        assert!(app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

        // Cancel deletion
        app.cancel_delete_chat();

        // Popup should be hidden
        assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
        assert!(app.chat_list_screen.as_ref().unwrap().pending_delete_index.is_none());

        // Chat should still exist
        assert_eq!(app.app_state.chats.len(), 1);
    }

    #[test]
    fn test_app_confirm_delete_inactive_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add inactive chat
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.chats[0].is_active = false;

        // Show chat list and delete confirmation
        app.show_chat_list_screen();
        app.show_delete_confirmation();

        // Confirm deletion
        app.confirm_delete_chat();

        // Chat should be deleted
        assert_eq!(app.app_state.chats.len(), 0);

        // Popup should be hidden
        assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

        // Status should indicate inactive chat deletion
        let status = app.chat_list_screen.as_ref().unwrap().status_message.as_ref();
        assert!(status.is_some());
        assert!(status.unwrap().contains("inactive"));
    }

    #[test]
    fn test_app_confirm_delete_active_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add active chat
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.chats[0].is_active = true;

        // Show chat list and delete confirmation
        app.show_chat_list_screen();
        app.show_delete_confirmation();

        // Confirm deletion
        app.confirm_delete_chat();

        // Chat should be deleted
        assert_eq!(app.app_state.chats.len(), 0);

        // Popup should be hidden
        assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);

        // Status should indicate delete request was sent
        let status = app.chat_list_screen.as_ref().unwrap().status_message.as_ref();
        assert!(status.is_some());
        assert!(status.unwrap().contains("delete request"));
    }

    #[test]
    fn test_app_delete_chat_adjusts_selection() {
        let mut app = App::new().expect("Failed to create app");

        // Add chats
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.add_chat("bob_uid".to_string());
        app.app_state.add_chat("charlie_uid".to_string());

        // Show chat list and select last chat
        app.show_chat_list_screen();
        if let Some(screen) = &mut app.chat_list_screen {
            screen.selected_index = 2;
        }

        // Show confirmation for last chat and confirm
        app.show_delete_confirmation();
        app.confirm_delete_chat();

        // Selection should be adjusted to index 1 (the new last item)
        assert_eq!(app.chat_list_screen.as_ref().unwrap().selected_index, 1);
        assert_eq!(app.app_state.chats.len(), 2);
    }

    #[test]
    fn test_app_delete_middle_chat_keeps_selection() {
        let mut app = App::new().expect("Failed to create app");

        // Add chats
        app.app_state.add_chat("alice_uid".to_string());
        app.app_state.add_chat("bob_uid".to_string());
        app.app_state.add_chat("charlie_uid".to_string());

        // Show chat list and select middle chat
        app.show_chat_list_screen();
        if let Some(screen) = &mut app.chat_list_screen {
            screen.selected_index = 1;
        }

        // Delete middle chat
        app.show_delete_confirmation();
        app.confirm_delete_chat();

        // Selection should stay at index 1 (now pointing to charlie)
        assert_eq!(app.chat_list_screen.as_ref().unwrap().selected_index, 1);
        assert_eq!(app.app_state.chats.len(), 2);
        assert_eq!(app.app_state.chats[0].contact_uid, "alice_uid");
        assert_eq!(app.app_state.chats[1].contact_uid, "charlie_uid");
    }

    #[test]
    fn test_app_delete_only_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add one chat
        app.app_state.add_chat("alice_uid".to_string());

        // Show chat list
        app.show_chat_list_screen();

        // Delete the only chat
        app.show_delete_confirmation();
        app.confirm_delete_chat();

        // Should have no chats
        assert_eq!(app.app_state.chats.len(), 0);

        // Popup should be hidden
        assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
    }

    #[test]
    fn test_app_delete_empty_list_does_nothing() {
        let mut app = App::new().expect("Failed to create app");

        // Show chat list (with no chats)
        app.show_chat_list_screen();

        // Try to show delete confirmation
        app.show_delete_confirmation();

        // Popup should not be shown
        assert!(!app.chat_list_screen.as_ref().unwrap().show_delete_confirmation);
    }

    #[test]
    fn test_chat_list_screen_popup_state_independent() {
        let mut screen = ChatListScreen::new();

        // Can call hide without showing first
        screen.hide_delete_popup();
        assert!(!screen.show_delete_confirmation);

        // Can call show multiple times
        screen.show_delete_popup(0);
        screen.show_delete_popup(1);
        assert!(screen.show_delete_confirmation);
        assert_eq!(screen.pending_delete_index, Some(1));
    }

    #[test]
    fn test_confirm_delete_with_invalid_index() {
        let mut app = App::new().expect("Failed to create app");

        // Add one chat
        app.app_state.add_chat("alice_uid".to_string());

        // Show chat list
        app.show_chat_list_screen();

        // Manually set an invalid pending delete index
        if let Some(screen) = &mut app.chat_list_screen {
            screen.show_delete_confirmation = true;
            screen.pending_delete_index = Some(999);
        }

        let initial_count = app.app_state.chats.len();

        // Try to confirm delete
        app.confirm_delete_chat();

        // Chat should not be deleted
        assert_eq!(app.app_state.chats.len(), initial_count);
    }

    #[test]
    fn test_settings_screen_creation() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let screen = SettingsScreen::new(path.clone());

        // Should load default settings
        assert_eq!(screen.retry_interval_input, "10"); // Default is 10 minutes
        assert_eq!(screen.selected_field, 0);
        assert!(screen.status_message.is_some());
        assert!(!screen.is_error);
        assert_eq!(screen.settings_path, path);
    }

    #[test]
    fn test_settings_screen_add_char() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.clear_input();

        // Should accept digits
        screen.add_char('1');
        assert_eq!(screen.retry_interval_input, "1");

        screen.add_char('5');
        assert_eq!(screen.retry_interval_input, "15");

        // Should reject non-digits
        screen.add_char('a');
        assert_eq!(screen.retry_interval_input, "15");

        screen.add_char('!');
        assert_eq!(screen.retry_interval_input, "15");
    }

    #[test]
    fn test_settings_screen_backspace() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.retry_interval_input = "123".to_string();

        screen.backspace();
        assert_eq!(screen.retry_interval_input, "12");

        screen.backspace();
        assert_eq!(screen.retry_interval_input, "1");

        screen.backspace();
        assert_eq!(screen.retry_interval_input, "");

        // Backspace on empty should not panic
        screen.backspace();
        assert_eq!(screen.retry_interval_input, "");
    }

    #[test]
    fn test_settings_screen_clear_input() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.retry_interval_input = "123".to_string();

        screen.clear_input();
        assert_eq!(screen.retry_interval_input, "");
    }

    #[test]
    fn test_settings_screen_validate_empty() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.clear_input();

        let result = screen.validate_and_save();

        assert!(!result);
        assert!(screen.is_error);
        assert!(screen.status_message.as_ref().unwrap().contains("empty"));
    }

    #[test]
    fn test_settings_screen_validate_zero() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.retry_interval_input = "0".to_string();

        let result = screen.validate_and_save();

        assert!(!result);
        assert!(screen.is_error);
        assert!(screen.status_message.as_ref().unwrap().contains("at least 1"));
    }

    #[test]
    fn test_settings_screen_validate_too_large() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.retry_interval_input = "2000".to_string();

        let result = screen.validate_and_save();

        assert!(!result);
        assert!(screen.is_error);
        assert!(screen.status_message.as_ref().unwrap().contains("1440"));
    }

    #[test]
    fn test_settings_screen_validate_valid() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path);
        screen.retry_interval_input = "30".to_string();

        let result = screen.validate_and_save();

        assert!(result);
        assert!(!screen.is_error);
        assert!(screen.status_message.as_ref().unwrap().contains("✓"));
        assert!(screen.status_message.as_ref().unwrap().contains("30"));
    }

    #[test]
    fn test_settings_screen_save_persists() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp_file.path().to_string_lossy().to_string();

        let mut screen = SettingsScreen::new(path.clone());
        screen.retry_interval_input = "45".to_string();

        let result = screen.validate_and_save();
        assert!(result);

        // Load settings from file to verify
        let settings = pure2p::storage::Settings::load(&path).expect("Failed to load settings");
        assert_eq!(settings.retry_interval_minutes, 45);
        assert_eq!(settings.global_retry_interval_ms, 45 * 60 * 1000);
    }

    #[test]
    fn test_app_show_settings_screen() {
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

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
        let mut app = App::new().expect("Failed to create app");

        // Navigate to Settings item (index 3)
        app.selected_index = 3;
        assert_eq!(app.selected_item(), MenuItem::Settings);

        // Trigger selection
        app.select();

        // Should navigate to settings screen
        assert_eq!(app.current_screen, Screen::Settings);
        assert!(app.settings_screen.is_some());
    }

    #[test]
    fn test_status_indicators_priority_expired_contact() {
        use chrono::{Duration, Utc};

        let mut app = App::new().expect("Failed to create app");

        // Add a chat
        app.app_state.add_chat("alice_uid".to_string());

        // Add an expired contact
        let contact = Contact::new(
            "alice_uid".to_string(),
            "192.168.1.100:8080".to_string(),
            vec![1, 2, 3, 4],
            Utc::now() - Duration::hours(1), // Expired 1 hour ago
        );
        app.app_state.contacts.push(contact);

        // Set chat to have pending messages AND be active
        app.app_state.chats[0].has_pending_messages = true;
        app.app_state.chats[0].is_active = true;

        // Expired contact should have highest priority
        // Even with pending and active flags set, should show warning indicator
        // We can't directly test the rendering, but we can verify the contact is expired
        let is_expired = app.app_state.contacts[0].is_expired();
        assert!(is_expired, "Contact should be expired");
    }

    #[test]
    fn test_status_indicators_priority_pending_messages() {
        let mut app = App::new().expect("Failed to create app");

        // Add a chat with pending messages
        app.app_state.add_chat("bob_uid".to_string());
        app.app_state.chats[0].has_pending_messages = true;
        app.app_state.chats[0].is_active = true;

        // Add a non-expired contact
        let contact = Contact::new(
            "bob_uid".to_string(),
            "192.168.1.101:8080".to_string(),
            vec![5, 6, 7, 8],
            chrono::Utc::now() + chrono::Duration::days(30),
        );
        app.app_state.contacts.push(contact);

        // Verify pending messages flag is set
        assert!(app.app_state.chats[0].has_pending_messages);
        assert!(!app.app_state.contacts[0].is_expired());
    }

    #[test]
    fn test_status_indicators_priority_active_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add an active chat (new messages)
        app.app_state.add_chat("charlie_uid".to_string());
        app.app_state.chats[0].is_active = true;
        app.app_state.chats[0].has_pending_messages = false;

        // Add a non-expired contact
        let contact = Contact::new(
            "charlie_uid".to_string(),
            "192.168.1.102:8080".to_string(),
            vec![9, 10, 11, 12],
            chrono::Utc::now() + chrono::Duration::days(30),
        );
        app.app_state.contacts.push(contact);

        // Verify chat is active and contact is not expired
        assert!(app.app_state.chats[0].is_active);
        assert!(!app.app_state.chats[0].has_pending_messages);
        assert!(!app.app_state.contacts[0].is_expired());
    }

    #[test]
    fn test_status_indicators_priority_inactive_chat() {
        let mut app = App::new().expect("Failed to create app");

        // Add an inactive chat (read/no new messages)
        app.app_state.add_chat("dave_uid".to_string());
        app.app_state.chats[0].is_active = false;
        app.app_state.chats[0].has_pending_messages = false;

        // Verify chat is inactive
        assert!(!app.app_state.chats[0].is_active);
        assert!(!app.app_state.chats[0].has_pending_messages);
    }

    #[test]
    fn test_contact_expiry_check() {
        use chrono::{Duration, Utc};

        // Test expired contact
        let expired_contact = Contact::new(
            "expired_uid".to_string(),
            "192.168.1.100:8080".to_string(),
            vec![1, 2, 3, 4],
            Utc::now() - Duration::hours(1),
        );
        assert!(expired_contact.is_expired(), "Contact should be expired");

        // Test valid contact
        let valid_contact = Contact::new(
            "valid_uid".to_string(),
            "192.168.1.101:8080".to_string(),
            vec![5, 6, 7, 8],
            Utc::now() + Duration::days(30),
        );
        assert!(!valid_contact.is_expired(), "Contact should not be expired");
    }

    #[test]
    fn test_chat_with_no_matching_contact() {
        let mut app = App::new().expect("Failed to create app");

        // Add a chat without adding a corresponding contact
        app.app_state.add_chat("orphan_uid".to_string());
        app.app_state.chats[0].is_active = true;

        // Verify chat exists but no contact exists
        assert_eq!(app.app_state.chats.len(), 1);
        assert_eq!(app.app_state.contacts.len(), 0);

        // The rendering code should handle this gracefully (defaults to false for is_expired)
    }

    #[test]
    fn test_multiple_chats_different_states() {
        use chrono::{Duration, Utc};

        let mut app = App::new().expect("Failed to create app");

        // Chat 1: Expired contact
        app.app_state.add_chat("expired_uid".to_string());
        let expired_contact = Contact::new(
            "expired_uid".to_string(),
            "192.168.1.100:8080".to_string(),
            vec![1, 2, 3, 4],
            Utc::now() - Duration::hours(1),
        );
        app.app_state.contacts.push(expired_contact);

        // Chat 2: Pending messages
        app.app_state.add_chat("pending_uid".to_string());
        app.app_state.chats[1].has_pending_messages = true;
        let pending_contact = Contact::new(
            "pending_uid".to_string(),
            "192.168.1.101:8080".to_string(),
            vec![5, 6, 7, 8],
            Utc::now() + Duration::days(30),
        );
        app.app_state.contacts.push(pending_contact);

        // Chat 3: Active (new messages)
        app.app_state.add_chat("active_uid".to_string());
        app.app_state.chats[2].is_active = true;
        let active_contact = Contact::new(
            "active_uid".to_string(),
            "192.168.1.102:8080".to_string(),
            vec![9, 10, 11, 12],
            Utc::now() + Duration::days(30),
        );
        app.app_state.contacts.push(active_contact);

        // Chat 4: Inactive (read)
        app.app_state.add_chat("inactive_uid".to_string());
        app.app_state.chats[3].is_active = false;

        // Verify all states
        assert_eq!(app.app_state.chats.len(), 4);
        assert_eq!(app.app_state.contacts.len(), 3);

        assert!(app.app_state.contacts[0].is_expired());
        assert!(!app.app_state.contacts[1].is_expired());
        assert!(app.app_state.chats[1].has_pending_messages);
        assert!(app.app_state.chats[2].is_active);
        assert!(!app.app_state.chats[3].is_active);
    }

    #[test]
    fn test_chat_pending_flag_methods() {
        let mut app = App::new().expect("Failed to create app");
        app.app_state.add_chat("test_uid".to_string());

        // Initially no pending messages
        assert!(!app.app_state.chats[0].has_pending_messages);

        // Mark as having pending
        app.app_state.chats[0].mark_has_pending();
        assert!(app.app_state.chats[0].has_pending_messages);

        // Mark as no pending
        app.app_state.chats[0].mark_no_pending();
        assert!(!app.app_state.chats[0].has_pending_messages);
    }

    #[test]
    fn test_startup_sync_screen_creation() {
        let screen = StartupSyncScreen::new(10);

        assert_eq!(screen.total_messages, 10);
        assert_eq!(screen.succeeded, 0);
        assert_eq!(screen.failed, 0);
        assert_eq!(screen.current, 0);
        assert!(!screen.is_complete);
    }

    #[test]
    fn test_startup_sync_screen_process_message_success() {
        let mut screen = StartupSyncScreen::new(3);

        screen.process_message(true);
        assert_eq!(screen.succeeded, 1);
        assert_eq!(screen.failed, 0);
        assert_eq!(screen.current, 1);
        assert!(!screen.is_complete);

        screen.process_message(true);
        assert_eq!(screen.succeeded, 2);
        assert_eq!(screen.current, 2);
        assert!(!screen.is_complete);

        screen.process_message(true);
        assert_eq!(screen.succeeded, 3);
        assert_eq!(screen.current, 3);
        assert!(screen.is_complete);
    }

    #[test]
    fn test_startup_sync_screen_process_message_failure() {
        let mut screen = StartupSyncScreen::new(2);

        screen.process_message(false);
        assert_eq!(screen.succeeded, 0);
        assert_eq!(screen.failed, 1);
        assert_eq!(screen.current, 1);

        screen.process_message(false);
        assert_eq!(screen.succeeded, 0);
        assert_eq!(screen.failed, 2);
        assert_eq!(screen.current, 2);
        assert!(screen.is_complete);
    }

    #[test]
    fn test_startup_sync_screen_mixed_results() {
        let mut screen = StartupSyncScreen::new(5);

        screen.process_message(true);   // Success
        screen.process_message(false);  // Fail
        screen.process_message(true);   // Success
        screen.process_message(true);   // Success
        screen.process_message(false);  // Fail

        assert_eq!(screen.succeeded, 3);
        assert_eq!(screen.failed, 2);
        assert_eq!(screen.current, 5);
        assert!(screen.is_complete);
    }

    #[test]
    fn test_startup_sync_screen_progress_percentage() {
        let mut screen = StartupSyncScreen::new(10);

        assert_eq!(screen.get_progress_percentage(), 0);

        screen.current = 2;
        assert_eq!(screen.get_progress_percentage(), 20);

        screen.current = 5;
        assert_eq!(screen.get_progress_percentage(), 50);

        screen.current = 10;
        assert_eq!(screen.get_progress_percentage(), 100);
    }

    #[test]
    fn test_startup_sync_screen_progress_percentage_empty() {
        let screen = StartupSyncScreen::new(0);
        assert_eq!(screen.get_progress_percentage(), 100);
    }

    #[test]
    fn test_startup_sync_screen_elapsed_time() {
        let screen = StartupSyncScreen::new(5);

        // Should return a formatted time string
        let elapsed = screen.get_elapsed_time();
        assert!(elapsed.ends_with('s'));
        assert!(elapsed.contains('.'));
    }

    #[test]
    fn test_app_startup_with_no_pending_messages() {
        let app = App::new().expect("Failed to create app");

        // With no pending messages, should start on MainMenu
        assert_eq!(app.current_screen, Screen::MainMenu);
        assert!(app.startup_sync_screen.is_none());
    }

    #[test]
    fn test_app_update_startup_sync() {
        let mut app = App::new().expect("Failed to create app");

        // Manually create a startup sync screen with messages
        app.startup_sync_screen = Some(StartupSyncScreen::new(5));
        app.current_screen = Screen::StartupSync;

        let initial_current = app.startup_sync_screen.as_ref().unwrap().current;

        // Update should process one message
        app.update_startup_sync();

        let updated_current = app.startup_sync_screen.as_ref().unwrap().current;
        assert_eq!(updated_current, initial_current + 1);
    }

    #[test]
    fn test_app_complete_startup_sync() {
        let mut app = App::new().expect("Failed to create app");

        // Set up startup sync screen
        app.startup_sync_screen = Some(StartupSyncScreen::new(1));
        app.current_screen = Screen::StartupSync;

        // Complete the sync
        app.complete_startup_sync();

        assert_eq!(app.current_screen, Screen::MainMenu);
        assert!(app.startup_sync_screen.is_none());
    }

    #[test]
    fn test_startup_sync_completes_after_all_messages() {
        let mut screen = StartupSyncScreen::new(3);

        assert!(!screen.is_complete);

        screen.process_message(true);
        assert!(!screen.is_complete);

        screen.process_message(true);
        assert!(!screen.is_complete);

        screen.process_message(true);
        assert!(screen.is_complete);

        // Processing after complete should not panic
        screen.process_message(true);
        assert_eq!(screen.current, 4); // Still increments
    }

    #[test]
    fn test_startup_sync_screen_zero_messages() {
        let screen = StartupSyncScreen::new(0);

        assert_eq!(screen.total_messages, 0);
        assert!(screen.is_complete); // Should be complete immediately
        assert_eq!(screen.get_progress_percentage(), 100);
    }
}
