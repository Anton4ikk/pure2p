//! Startup sync screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};
use crate::tui::app::App;

/// Renders the screen

pub fn render_startup_sync(f: &mut Frame, app: &App) {
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
