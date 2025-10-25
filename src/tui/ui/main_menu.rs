//! Main menu screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::app::App;

/// Renders the screen

pub fn render_main_menu(f: &mut Frame, app: &App) {
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
    let help_text = vec![
        Line::from(selected.description()).style(Style::default().fg(Color::White)),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation: ", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓ or j/k", Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Select: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Quick: ", Style::default().fg(Color::DarkGray)),
            Span::styled("c/s/i/n", Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Quit: ", Style::default().fg(Color::DarkGray)),
            Span::styled("q/Esc", Style::default().fg(Color::Red)),
        ]),
    ];
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[3]);
}
