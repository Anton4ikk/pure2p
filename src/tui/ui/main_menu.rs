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

    // Create main layout - adjust based on whether we need to show connectivity warning/error
    let show_warning = app.connectivity_result.is_none();
    let show_error = app.connectivity_result.as_ref().map_or(false, |result| !result.is_success());

    // Check transport server status
    let transport_status = app.transport_server_status.lock().unwrap().clone();
    let show_transport_error = matches!(transport_status, crate::tui::app::TransportServerStatus::Failed(_));
    let show_transport_starting = matches!(transport_status, crate::tui::app::TransportServerStatus::Starting);

    let show_notification = show_warning || show_error || show_transport_error || show_transport_starting;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(if show_notification {
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // IP display
                Constraint::Length(3), // Connectivity warning/error
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Help text
            ]
        } else {
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // IP display
                Constraint::Min(10),   // Menu
                Constraint::Length(3), // Help text
            ]
        })
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

    // Connectivity and transport server warnings/errors
    let menu_chunk_index = if show_notification {
        if show_transport_error {
            // Critical error: Transport server failed to start
            if let crate::tui::app::TransportServerStatus::Failed(ref error_msg) = transport_status {
                let error_text = Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled("Transport server failed: ", Style::default().fg(Color::Red)),
                    Span::styled(error_msg, Style::default().fg(Color::DarkGray)),
                ]);
                let error_widget = Paragraph::new(error_text)
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)).title("Critical Error"));
                f.render_widget(error_widget, chunks[2]);
            }
        } else if show_transport_starting {
            // Info: Transport server is starting
            let info_text = Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled("Starting transport server... ", Style::default().fg(Color::Cyan)),
            ]);
            let info_widget = Paragraph::new(info_text)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
            f.render_widget(info_widget, chunks[2]);
        } else if show_warning {
            // Warning while connectivity is being configured
            let warning_text = Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("Configuring network connectivity... ", Style::default().fg(Color::Yellow)),
                Span::styled("Contact sharing may not work until setup completes", Style::default().fg(Color::DarkGray)),
            ]);
            let warning_widget = Paragraph::new(warning_text)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
            f.render_widget(warning_widget, chunks[2]);
        } else if show_error {
            // Error when all connectivity attempts failed
            let error_text = Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("All connectivity attempts failed. ", Style::default().fg(Color::Red)),
                Span::styled("Go to Diagnostics to see details and retry.", Style::default().fg(Color::DarkGray)),
            ]);
            let error_widget = Paragraph::new(error_text)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)));
            f.render_widget(error_widget, chunks[2]);
        }
        3 // Menu is at index 3 when notification is shown
    } else {
        2 // Menu is at index 2 when no notification
    };

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
    f.render_widget(menu, chunks[menu_chunk_index]);

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
    f.render_widget(help, chunks[menu_chunk_index + 1]);
}
