//! Chat list screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::storage::Chat;
use crate::tui::app::App;

/// Renders the screen

pub fn render_chat_list(f: &mut Frame, app: &App) {
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
