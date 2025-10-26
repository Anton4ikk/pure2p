//! Mapping consent screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::tui::app::App;

/// Render mapping consent dialog
pub fn render_mapping_consent(f: &mut Frame, app: &App) {
    let screen = match &app.mapping_consent_screen {
        Some(s) => s,
        None => return,
    };

    // Create centered dialog area
    let area = centered_rect(70, 60, f.size());

    // Split into title, description, options, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(6),  // Description
            Constraint::Length(10), // Options
            Constraint::Min(0),     // Spacer
            Constraint::Length(2),  // Footer
        ])
        .split(area);

    // Render border around entire dialog
    let dialog_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Network Configuration Consent ");
    f.render_widget(dialog_block, area);

    // Title
    let title = Paragraph::new("Port Mapping Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Description
    let description_text = vec![
        Line::from("Pure2P needs to configure your network to enable peer-to-peer"),
        Line::from("connections. This requires automatic port forwarding (UPnP/NAT-PMP/PCP)."),
        Line::from(""),
        Line::from(Span::styled(
            "Do you want to allow automatic network configuration?",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
    ];
    let description = Paragraph::new(description_text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(description, chunks[1]);

    // Options list
    let options: Vec<ListItem> = (0..3)
        .map(|i| {
            let label = screen.get_option_label(i);
            let desc = screen.get_option_description(i);

            let (style, prefix) = if i == screen.selected_option {
                (
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    "> ",
                )
            } else {
                (Style::default().fg(Color::Gray), "  ")
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(label, style),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(desc, Style::default().fg(Color::DarkGray)),
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let options_list = List::new(options);
    f.render_widget(options_list, chunks[2]);

    // Footer with instructions
    let footer_text = vec![
        Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" Confirm  "),
            Span::styled("q", Style::default().fg(Color::Red)),
            Span::raw(" Exit"),
        ]),
    ];
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    f.render_widget(footer, chunks[4]);
}

/// Create a centered rectangle for dialog
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
