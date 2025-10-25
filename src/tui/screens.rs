//! Screen state structures for TUI

use arboard::Clipboard;
use chrono::{DateTime, Duration, Utc};
use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token, Contact, Settings};
use std::fs;

/// Share Contact screen state
#[derive(Debug)]
pub struct ShareContactScreen {
    /// Generated contact token
    pub token: String,
    /// Token expiry timestamp
    pub expiry: DateTime<Utc>,
    /// Status message (for copy/save feedback)
    pub status_message: Option<String>,
}

impl ShareContactScreen {
    /// Create new share contact screen
    pub fn new(keypair: &KeyPair, local_ip: &str) -> Self {
        // Default: 30 days expiry
        let expiry = Utc::now() + Duration::days(30);
        let token = generate_contact_token(
            local_ip,
            &keypair.public_key,
            &keypair.x25519_public,
            expiry,
        );

        Self {
            token,
            expiry,
            status_message: None,
        }
    }

    /// Copy token to clipboard
    pub fn copy_to_clipboard(&mut self) {
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

    /// Save token to file
    pub fn save_to_file(&mut self) {
        let filename = format!("contact_token_{}.txt", Utc::now().format("%Y%m%d_%H%M%S"));
        match fs::write(&filename, &self.token) {
            Ok(_) => self.status_message = Some(format!("Saved to {}", filename)),
            Err(e) => self.status_message = Some(format!("Save failed: {}", e)),
        }
    }
}

/// Import Contact screen state
#[derive(Debug)]
pub struct ImportContactScreen {
    /// Input buffer for token
    pub input: String,
    /// Parsed contact (if valid)
    pub parsed_contact: Option<Contact>,
    /// Status message
    pub status_message: Option<String>,
    /// Whether the status is an error
    pub is_error: bool,
}

impl ImportContactScreen {
    /// Create new import contact screen
    pub fn new() -> Self {
        Self {
            input: String::new(),
            parsed_contact: None,
            status_message: Some("Paste contact token and press Enter to import".to_string()),
            is_error: false,
        }
    }

    /// Add character to input
    pub fn add_char(&mut self, c: char) {
        self.input.push(c);
    }

    /// Remove last character from input
    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// Clear input and reset state
    pub fn clear(&mut self) {
        self.input.clear();
        self.parsed_contact = None;
        self.status_message = Some("Input cleared. Paste contact token and press Enter".to_string());
        self.is_error = false;
    }

    /// Paste from clipboard
    pub fn paste_from_clipboard(&mut self) {
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

    /// Parse input token
    pub fn parse_token(&mut self) {
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

    /// Get parsed contact
    pub fn get_contact(&self) -> Option<&Contact> {
        self.parsed_contact.as_ref()
    }
}

/// Chat List screen state
#[derive(Debug)]
pub struct ChatListScreen {
    /// Selected chat index
    pub selected_index: usize,
    /// Status message
    pub status_message: Option<String>,
    /// Confirmation popup state
    pub show_delete_confirmation: bool,
    /// Index of chat pending deletion
    pub pending_delete_index: Option<usize>,
}

impl ChatListScreen {
    /// Create new chat list screen
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            status_message: None,
            show_delete_confirmation: false,
            pending_delete_index: None,
        }
    }

    /// Move to next chat
    pub fn next(&mut self, chat_count: usize) {
        if chat_count > 0 {
            self.selected_index = (self.selected_index + 1) % chat_count;
        }
    }

    /// Move to previous chat
    pub fn previous(&mut self, chat_count: usize) {
        if chat_count > 0 {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = chat_count - 1;
            }
        }
    }

    /// Set status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Show delete confirmation popup
    pub fn show_delete_popup(&mut self, chat_index: usize) {
        self.show_delete_confirmation = true;
        self.pending_delete_index = Some(chat_index);
    }

    /// Hide delete confirmation popup
    pub fn hide_delete_popup(&mut self) {
        self.show_delete_confirmation = false;
        self.pending_delete_index = None;
    }
}

/// Chat View screen state
#[derive(Debug)]
pub struct ChatViewScreen {
    /// UID of the contact we're chatting with
    pub contact_uid: String,
    /// Input buffer for message composition
    pub input: String,
    /// Scroll offset for message history
    pub scroll_offset: usize,
    /// Status message
    pub status_message: Option<String>,
}

impl ChatViewScreen {
    /// Create new chat view screen
    pub fn new(contact_uid: String) -> Self {
        Self {
            contact_uid,
            input: String::new(),
            scroll_offset: 0,
            status_message: None,
        }
    }

    /// Add character to input
    pub fn add_char(&mut self, c: char) {
        self.input.push(c);
    }

    /// Remove last character from input
    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// Clear input buffer
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// Scroll message history up
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll message history down
    pub fn scroll_down(&mut self, max_offset: usize) {
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }

    /// Set status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
    }
}

/// Settings screen state
#[derive(Debug)]
pub struct SettingsScreen {
    /// Input buffer for retry interval
    pub retry_interval_input: String,
    /// Currently selected field (0 = retry interval, can extend for more fields)
    pub selected_field: usize,
    /// Status/confirmation message
    pub status_message: Option<String>,
    /// Whether status is an error
    pub is_error: bool,
    /// Settings path for saving
    pub settings_path: String,
}

impl SettingsScreen {
    /// Create new settings screen
    pub fn new(settings_path: String) -> Self {
        // Load current settings to populate defaults
        let settings = Settings::load(&settings_path).unwrap_or_default();

        Self {
            retry_interval_input: settings.retry_interval_minutes.to_string(),
            selected_field: 0,
            status_message: Some("Edit retry interval and press Enter to save".to_string()),
            is_error: false,
            settings_path,
        }
    }

    /// Add character to input (only digits)
    pub fn add_char(&mut self, c: char) {
        if c.is_ascii_digit() {
            self.retry_interval_input.push(c);
        }
    }

    /// Remove last character from input
    pub fn backspace(&mut self) {
        self.retry_interval_input.pop();
    }

    /// Clear input buffer
    pub fn clear_input(&mut self) {
        self.retry_interval_input.clear();
    }

    /// Validate and save settings
    pub fn validate_and_save(&mut self) -> bool {
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
        let mut settings = Settings::load(&self.settings_path)?;

        // Update retry interval
        settings.retry_interval_minutes = retry_interval_minutes;
        settings.global_retry_interval_ms = (retry_interval_minutes as u64) * 60 * 1000;

        // Save settings
        settings.save(&self.settings_path)?;

        Ok(())
    }
}

/// Diagnostics screen state
#[derive(Debug)]
pub struct DiagnosticsScreen {
    /// PCP mapping status
    pub pcp_status: Option<Result<crate::connectivity::PortMappingResult, String>>,
    /// NAT-PMP mapping status
    pub natpmp_status: Option<Result<crate::connectivity::PortMappingResult, String>>,
    /// UPnP mapping status
    pub upnp_status: Option<Result<crate::connectivity::PortMappingResult, String>>,
    /// Whether CGNAT was detected
    pub cgnat_detected: bool,
    /// Whether diagnostics are being refreshed
    pub is_refreshing: bool,
    /// Status message
    pub status_message: Option<String>,
    /// Local port being tested
    pub local_port: u16,
}

impl DiagnosticsScreen {
    /// Create new diagnostics screen
    pub fn new(local_port: u16) -> Self {
        Self {
            pcp_status: None,
            natpmp_status: None,
            upnp_status: None,
            cgnat_detected: false,
            is_refreshing: false,
            status_message: None,
            local_port,
        }
    }

    /// Set PCP status
    pub fn set_pcp_status(&mut self, status: Result<crate::connectivity::PortMappingResult, String>) {
        self.pcp_status = Some(status);
    }

    /// Set NAT-PMP status
    pub fn set_natpmp_status(&mut self, status: Result<crate::connectivity::PortMappingResult, String>) {
        self.natpmp_status = Some(status);
    }

    /// Set UPnP status
    pub fn set_upnp_status(&mut self, status: Result<crate::connectivity::PortMappingResult, String>) {
        self.upnp_status = Some(status);
        self.is_refreshing = false;
    }

    /// Start refresh
    pub fn start_refresh(&mut self) {
        self.is_refreshing = true;
        self.status_message = Some("Refreshing diagnostics...".to_string());
    }

    /// Set status message
    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
    }

    /// Set CGNAT detection status
    pub fn set_cgnat_detected(&mut self, detected: bool) {
        self.cgnat_detected = detected;
    }

    /// Update diagnostics from ConnectivityResult
    pub fn update_from_connectivity_result(&mut self, result: &crate::connectivity::ConnectivityResult) {
        // Update CGNAT detection
        self.cgnat_detected = result.cgnat_detected;

        // Update individual protocol statuses
        match &result.ipv6 {
            crate::connectivity::StrategyAttempt::Success(mapping) => {
                // IPv6 succeeded, no need to test other protocols
                self.pcp_status = Some(Ok(mapping.clone()));
            }
            _ => {}
        }

        match &result.pcp {
            crate::connectivity::StrategyAttempt::Success(mapping) => {
                self.pcp_status = Some(Ok(mapping.clone()));
            }
            crate::connectivity::StrategyAttempt::Failed(e) => {
                self.pcp_status = Some(Err(e.clone()));
            }
            crate::connectivity::StrategyAttempt::NotAttempted => {}
        }

        match &result.natpmp {
            crate::connectivity::StrategyAttempt::Success(mapping) => {
                self.natpmp_status = Some(Ok(mapping.clone()));
            }
            crate::connectivity::StrategyAttempt::Failed(e) => {
                self.natpmp_status = Some(Err(e.clone()));
            }
            crate::connectivity::StrategyAttempt::NotAttempted => {}
        }

        match &result.upnp {
            crate::connectivity::StrategyAttempt::Success(mapping) => {
                self.upnp_status = Some(Ok(mapping.clone()));
            }
            crate::connectivity::StrategyAttempt::Failed(e) => {
                self.upnp_status = Some(Err(e.clone()));
            }
            crate::connectivity::StrategyAttempt::NotAttempted => {}
        }

        self.is_refreshing = false;
    }
}

/// Startup Sync screen state
#[derive(Debug)]
pub struct StartupSyncScreen {
    /// Total pending messages to sync
    pub total_messages: usize,
    /// Number of successfully delivered messages
    pub succeeded: usize,
    /// Number of failed deliveries
    pub failed: usize,
    /// Current message being processed (for progress bar)
    pub current: usize,
    /// Whether sync is complete
    pub is_complete: bool,
    /// Timestamp when sync started
    pub start_time: std::time::Instant,
}

impl StartupSyncScreen {
    /// Create new startup sync screen
    pub fn new(total_messages: usize) -> Self {
        Self {
            total_messages,
            succeeded: 0,
            failed: 0,
            current: 0,
            is_complete: total_messages == 0, // Complete immediately if no messages
            start_time: std::time::Instant::now(),
        }
    }

    /// Process a message delivery result
    pub fn process_message(&mut self, success: bool) {
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

    /// Get progress percentage
    pub fn get_progress_percentage(&self) -> u16 {
        if self.total_messages == 0 {
            100
        } else {
            ((self.current as f64 / self.total_messages as f64) * 100.0) as u16
        }
    }

    /// Get elapsed time as formatted string
    pub fn get_elapsed_time(&self) -> String {
        let elapsed = self.start_time.elapsed();
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}
