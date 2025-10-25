//! Chat view screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use chrono::DateTime;
use crate::tui::app::App;

/// Renders the screen

pub fn render_chat_view(f: &mut Frame, app: &App) {
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
