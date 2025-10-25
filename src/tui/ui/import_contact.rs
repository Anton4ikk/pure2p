//! Import contact screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::tui::app::App;

/// Renders the screen

pub fn render_import_contact(f: &mut Frame, app: &App) {
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
