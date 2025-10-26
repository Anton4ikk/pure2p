//! Share contact screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::tui::app::App;
use super::helpers::format_duration_until;

/// Renders the screen

pub fn render_share_contact(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.share_contact_screen {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // UID and Port info
                Constraint::Length(3),  // Expiry info
                Constraint::Min(3),     // Token display (reduced from 5 to 3)
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

        // UID, Port and Expiry info (combined into one block)
        let info_text = format!(
            "UID: {}... | Port: {} | Expires in {}",
            &app.keypair.uid.to_string()[..8],
            app.local_port,
            format_duration_until(screen.expiry)
        );
        let info_widget = Paragraph::new(info_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Your Identity"));
        f.render_widget(info_widget, chunks[1]);

        // Spacer for expiry (reuse chunk[2] if needed, or skip)
        // Keeping the expiry block for backward compatibility
        let expiry_text = format!(
            "Expires: {} ({})",
            screen.expiry.format("%Y-%m-%d %H:%M:%S UTC"),
            format_duration_until(screen.expiry)
        );
        let expiry_widget = Paragraph::new(expiry_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Expiry"));
        f.render_widget(expiry_widget, chunks[2]);

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
        f.render_widget(token_widget, chunks[3]);

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
        f.render_widget(status_widget, chunks[4]);

        // Help text
        let help_text = "c: Copy to Clipboard | s: Save to File | Esc: Back to Menu";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[5]);
    }
}

