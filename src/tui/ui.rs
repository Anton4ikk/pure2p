//! UI rendering functions for TUI

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use chrono::{DateTime, Utc};
use crate::storage::Chat;
use crate::tui::types::Screen;
use crate::tui::app::App;

/// Main UI rendering function - dispatches to screen-specific render functions
pub fn ui(f: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::StartupSync => render_startup_sync(f, app),
        Screen::MainMenu => render_main_menu(f, app),
        Screen::ShareContact => render_share_contact(f, app),
        Screen::ImportContact => render_import_contact(f, app),
        Screen::ChatList => render_chat_list(f, app),
        Screen::ChatView => render_chat_view(f, app),
        Screen::Settings => render_settings(f, app),
        Screen::Diagnostics => render_diagnostics(f, app),
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

pub(crate) fn format_duration_until(expiry: DateTime<Utc>) -> String {
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

fn render_diagnostics(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.diagnostics_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(7),  // PCP status
                Constraint::Length(5),  // NAT-PMP status
                Constraint::Length(5),  // UPnP status
                Constraint::Min(3),     // Additional info
                Constraint::Length(3),  // Help text
            ])
            .split(size);

        // Title
        let title = Paragraph::new("Network Diagnostics & Port Mapping")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // PCP Status
        let pcp_text = if let Some(result) = &screen.pcp_status {
            match result {
                Ok(mapping) => {
                    vec![
                        Line::from(Span::styled(
                            "PCP: ✓ Success",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("External IP: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}", mapping.external_ip),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("External Port: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}", mapping.external_port),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("Lifetime: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}s", mapping.lifetime_secs),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                    ]
                }
                Err(e) => {
                    vec![
                        Line::from(Span::styled(
                            "PCP: ✗ Failed",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("Error: {}", e),
                            Style::default().fg(Color::Red),
                        )),
                    ]
                }
            }
        } else if screen.is_refreshing {
            vec![
                Line::from(Span::styled(
                    "PCP: Testing...",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Attempting to create port mapping...",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "PCP: Not tested",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press 'r' or F5 to test connectivity",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let pcp_widget = Paragraph::new(pcp_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Port Control Protocol (PCP)"));
        f.render_widget(pcp_widget, chunks[1]);

        // NAT-PMP Status
        let natpmp_text = if let Some(result) = &screen.natpmp_status {
            match result {
                Ok(mapping) => {
                    vec![
                        Line::from(Span::styled(
                            "NAT-PMP: ✓ Success",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("External IP: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}", mapping.external_ip),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("External Port: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}", mapping.external_port),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("Lifetime: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}s", mapping.lifetime_secs),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                    ]
                }
                Err(e) => {
                    vec![
                        Line::from(Span::styled(
                            "NAT-PMP: ✗ Failed",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("Error: {}", e),
                            Style::default().fg(Color::Red),
                        )),
                    ]
                }
            }
        } else if screen.is_refreshing && screen.pcp_status.is_some() {
            vec![
                Line::from(Span::styled(
                    "NAT-PMP: Testing...",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Attempting NAT-PMP fallback...",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "NAT-PMP: Not tested",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Fallback protocol (tested after PCP)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let natpmp_widget = Paragraph::new(natpmp_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("NAT Port Mapping Protocol (NAT-PMP)"));
        f.render_widget(natpmp_widget, chunks[2]);

        // UPnP Status
        let upnp_text = if let Some(result) = &screen.upnp_status {
            match result {
                Ok(mapping) => {
                    let renew_mins = (mapping.lifetime_secs as f64 * 0.8 / 60.0) as u32;
                    vec![
                        Line::from(Span::styled(
                            "UPnP: ✓ Success",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("External IP: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}", mapping.external_ip),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("External Port: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}", mapping.external_port),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled("Lifetime: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("{}s (renews in {} min)", mapping.lifetime_secs, renew_mins),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                    ]
                }
                Err(e) => {
                    vec![
                        Line::from(Span::styled(
                            "UPnP: ✗ Failed",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("Error: {}", e),
                            Style::default().fg(Color::Red),
                        )),
                    ]
                }
            }
        } else if screen.is_refreshing && screen.natpmp_status.is_some() {
            vec![
                Line::from(Span::styled(
                    "UPnP: Testing...",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Attempting UPnP fallback...",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "UPnP: Not tested",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Final fallback protocol (tested after NAT-PMP)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let upnp_widget = Paragraph::new(upnp_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Universal Plug and Play (UPnP)"));
        f.render_widget(upnp_widget, chunks[3]);

        // Additional info
        let info_text = vec![
            Line::from(vec![
                Span::styled("Local Port: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", screen.local_port),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Port mapping allows peers to connect to you directly",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let info_widget = Paragraph::new(info_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Info"));
        f.render_widget(info_widget, chunks[4]);

        // Help text
        let help_text = "r/F5: Refresh | b/Esc: Back | q: Quit";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[5]);
    }
}
