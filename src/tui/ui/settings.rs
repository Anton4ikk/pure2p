//! Settings screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

/// Renders the screen

pub fn render_settings(f: &mut Frame, app: &App) {
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
