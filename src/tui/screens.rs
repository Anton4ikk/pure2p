//! Screen state structures for TUI

use arboard::Clipboard;
use chrono::{DateTime, Duration, Utc};
use crate::crypto::KeyPair;
use crate::storage::{generate_contact_token, parse_contact_token, Contact};
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
        // Default: 1 day expiry
        let expiry = Utc::now() + Duration::days(1);
        let token = generate_contact_token(
            local_ip,
            &keypair.public_key,
            &keypair.private_key,
            &keypair.x25519_public,
            expiry,
        ).expect("Failed to generate contact token");

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
}

impl SettingsScreen {
    /// Create new settings screen
    pub fn new(current_retry_interval: u32) -> Self {
        Self {
            retry_interval_input: current_retry_interval.to_string(),
            selected_field: 0,
            status_message: Some("Edit retry interval and press Enter to save".to_string()),
            is_error: false,
        }
    }

    /// Add character to input (only digits, max 4 characters)
    pub fn add_char(&mut self, c: char) {
        if c.is_ascii_digit() && self.retry_interval_input.len() < 4 {
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

    /// Validate input and return the validated value
    /// Returns Some(minutes) if valid, None if invalid
    pub fn validate(&mut self) -> Option<u32> {
        if self.retry_interval_input.is_empty() {
            self.status_message = Some("Error: Retry interval cannot be empty".to_string());
            self.is_error = true;
            return None;
        }

        match self.retry_interval_input.parse::<u32>() {
            Ok(minutes) if minutes > 0 && minutes <= 1440 => {
                // Valid range: 1 minute to 24 hours (1440 minutes)
                self.is_error = false;
                Some(minutes)
            }
            Ok(minutes) if minutes == 0 => {
                self.status_message = Some("Error: Retry interval must be at least 1 minute".to_string());
                self.is_error = true;
                None
            }
            Ok(_) => {
                self.status_message = Some("Error: Retry interval cannot exceed 1440 minutes (24 hours)".to_string());
                self.is_error = true;
                None
            }
            Err(_) => {
                self.status_message = Some("Error: Invalid number".to_string());
                self.is_error = true;
                None
            }
        }
    }

    /// Set success message after saving
    pub fn set_saved_message(&mut self, minutes: u32) {
        self.status_message = Some(format!("✓ Saved! Retry interval set to {} minutes", minutes));
        self.is_error = false;
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
    /// IPv4 address detected
    pub ipv4_address: Option<String>,
    /// IPv6 address detected
    pub ipv6_address: Option<String>,
    /// External endpoint (IP:Port from successful mapping)
    pub external_endpoint: Option<String>,
    /// Last ping round-trip time in milliseconds
    pub last_ping_rtt_ms: Option<u64>,
    /// Number of messages in queue
    pub queue_size: usize,
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
            ipv4_address: None,
            ipv6_address: None,
            external_endpoint: None,
            last_ping_rtt_ms: None,
            queue_size: 0,
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

    /// Set IPv4 address
    pub fn set_ipv4_address(&mut self, address: Option<String>) {
        self.ipv4_address = address;
    }

    /// Set IPv6 address
    pub fn set_ipv6_address(&mut self, address: Option<String>) {
        self.ipv6_address = address;
    }

    /// Set external endpoint
    pub fn set_external_endpoint(&mut self, endpoint: Option<String>) {
        self.external_endpoint = endpoint;
    }

    /// Set last ping RTT
    pub fn set_last_ping_rtt(&mut self, rtt_ms: Option<u64>) {
        self.last_ping_rtt_ms = rtt_ms;
    }

    /// Set queue size
    pub fn set_queue_size(&mut self, size: usize) {
        self.queue_size = size;
    }

    /// Calculate remaining lifetime seconds for the active mapping
    pub fn get_remaining_lifetime_secs(&self) -> Option<i64> {
        let mapping = if let Some(Ok(m)) = &self.pcp_status {
            Some(m)
        } else if let Some(Ok(m)) = &self.natpmp_status {
            Some(m)
        } else if let Some(Ok(m)) = &self.upnp_status {
            Some(m)
        } else {
            None
        }?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        let elapsed_secs = ((now_ms - mapping.created_at_ms) / 1000).max(0);
        let remaining_secs = (mapping.lifetime_secs as i64) - elapsed_secs;

        Some(remaining_secs.max(0))
    }

    /// Calculate time until renewal (80% of lifetime)
    pub fn get_renewal_countdown_secs(&self) -> Option<i64> {
        let mapping = if let Some(Ok(m)) = &self.pcp_status {
            Some(m)
        } else if let Some(Ok(m)) = &self.natpmp_status {
            Some(m)
        } else if let Some(Ok(m)) = &self.upnp_status {
            Some(m)
        } else {
            None
        }?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        let elapsed_secs = ((now_ms - mapping.created_at_ms) / 1000).max(0);
        let renewal_threshold_secs = ((mapping.lifetime_secs as f64) * 0.8) as i64;
        let countdown_secs = renewal_threshold_secs - elapsed_secs;

        Some(countdown_secs.max(0))
    }

    /// Format remaining time as human-readable string
    pub fn format_time_remaining(secs: i64) -> String {
        if secs >= 3600 {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else if secs >= 60 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}s", secs)
        }
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

        // Update external endpoint from successful mapping
        if let Some(mapping) = &result.mapping {
            self.external_endpoint = Some(format!("{}:{}", mapping.external_ip, mapping.external_port));

            // Detect IPv4/IPv6 from external IP
            if mapping.external_ip.is_ipv4() {
                self.ipv4_address = Some(mapping.external_ip.to_string());
            } else if mapping.external_ip.is_ipv6() {
                self.ipv6_address = Some(mapping.external_ip.to_string());
            }
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
