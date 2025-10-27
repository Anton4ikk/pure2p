//! Diagnostics screen rendering

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::App;

/// Renders the screen

pub fn render_diagnostics(f: &mut Frame, app: &App) {
    let size = f.size();

    if let Some(screen) = &app.diagnostics_screen {
        // Create layout with two columns
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(10),    // Main content (two columns)
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
        f.render_widget(title, main_chunks[0]);

        // Split main content into two columns
        let content_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),  // Left column: Protocol statuses
                Constraint::Percentage(50),  // Right column: System info
            ])
            .split(main_chunks[1]);

        // Left column - Protocol status sections
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // PCP status
                Constraint::Length(5),  // NAT-PMP status
                Constraint::Length(5),  // UPnP status
                Constraint::Min(3),     // Additional info / CGNAT warning
            ])
            .split(content_columns[0]);

        // Right column - System info sections
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),  // IPv4/IPv6 & External endpoint
                Constraint::Length(5),  // Mapping lifetime & renewal
                Constraint::Length(4),  // Network metrics (RTT, Queue)
                Constraint::Min(3),     // Reserved for future use
            ])
            .split(content_columns[1]);

        // Determine if any mapping succeeded (for color logic)
        let any_success = screen.pcp_status.as_ref().map_or(false, |r| r.is_ok())
            || screen.natpmp_status.as_ref().map_or(false, |r| r.is_ok())
            || screen.upnp_status.as_ref().map_or(false, |r| r.is_ok());

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
                    // Use warning color if any protocol succeeded, error color if all failed
                    let error_color = if any_success { Color::Yellow } else { Color::Red };
                    vec![
                        Line::from(Span::styled(
                            "PCP: ✗ Failed",
                            Style::default().fg(error_color).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("Error: {}", e),
                            Style::default().fg(error_color),
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
        f.render_widget(pcp_widget, left_chunks[0]);

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
                    // Use warning color if any protocol succeeded, error color if all failed
                    let error_color = if any_success { Color::Yellow } else { Color::Red };
                    vec![
                        Line::from(Span::styled(
                            "NAT-PMP: ✗ Failed",
                            Style::default().fg(error_color).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("Error: {}", e),
                            Style::default().fg(error_color),
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
        f.render_widget(natpmp_widget, left_chunks[1]);

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
                    // Use warning color if any protocol succeeded, error color if all failed
                    let error_color = if any_success { Color::Yellow } else { Color::Red };
                    vec![
                        Line::from(Span::styled(
                            "UPnP: ✗ Failed",
                            Style::default().fg(error_color).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("Error: {}", e),
                            Style::default().fg(error_color),
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
        f.render_widget(upnp_widget, left_chunks[2]);

        // Additional info / CGNAT warning (left column bottom)
        let mut info_text = vec![
            Line::from(vec![
                Span::styled("Local Port: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", screen.local_port),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(""),
        ];

        // Add CGNAT warning if detected
        if screen.cgnat_detected {
            info_text.push(Line::from(Span::styled(
                "⚠️  CGNAT detected",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            info_text.push(Line::from(Span::styled(
                "Relay required for P2P",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            info_text.push(Line::from(Span::styled(
                "Port mapping active",
                Style::default().fg(Color::Green),
            )));
        }

        let info_widget = Paragraph::new(info_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(info_widget, left_chunks[3]);

        // Right column: IPv4/IPv6 & External endpoint
        let ip_text = vec![
            Line::from(vec![
                Span::styled("IPv4: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    screen.ipv4_address.as_ref().map(|s| s.as_str()).unwrap_or("Not detected"),
                    if screen.ipv4_address.is_some() { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) },
                ),
            ]),
            Line::from(vec![
                Span::styled("IPv6: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    screen.ipv6_address.as_ref().map(|s| s.as_str()).unwrap_or("Not detected"),
                    if screen.ipv6_address.is_some() { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("External: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    screen.external_endpoint.as_ref().map(|s| s.as_str()).unwrap_or("N/A"),
                    if screen.external_endpoint.is_some() { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) },
                ),
            ]),
        ];

        let ip_widget = Paragraph::new(ip_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("IP Detection"));
        f.render_widget(ip_widget, right_chunks[0]);

        // Mapping lifetime & renewal countdown
        let lifetime_text = if let Some(remaining_secs) = screen.get_remaining_lifetime_secs() {
            let renewal_secs = screen.get_renewal_countdown_secs().unwrap_or(0);
            vec![
                Line::from(vec![
                    Span::styled("Lifetime: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        crate::tui::screens::DiagnosticsScreen::format_time_remaining(remaining_secs),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Renewal in: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        crate::tui::screens::DiagnosticsScreen::format_time_remaining(renewal_secs),
                        if renewal_secs < 300 { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Green) },
                    ),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "No active mapping",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let lifetime_widget = Paragraph::new(lifetime_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Mapping Lifecycle"));
        f.render_widget(lifetime_widget, right_chunks[1]);

        // Network metrics (RTT, Queue size)
        let metrics_text = vec![
            Line::from(vec![
                Span::styled("Last Ping RTT: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    screen.last_ping_rtt_ms.map(|rtt| format!("{}ms", rtt)).unwrap_or_else(|| "N/A".to_string()),
                    if let Some(rtt) = screen.last_ping_rtt_ms {
                        if rtt < 50 { Style::default().fg(Color::Green) }
                        else if rtt < 150 { Style::default().fg(Color::Yellow) }
                        else { Style::default().fg(Color::Red) }
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Queue Size: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}", screen.queue_size),
                    if screen.queue_size == 0 { Style::default().fg(Color::Green) }
                    else if screen.queue_size < 10 { Style::default().fg(Color::Yellow) }
                    else { Style::default().fg(Color::Red) },
                ),
            ]),
        ];

        let metrics_widget = Paragraph::new(metrics_text)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Network Metrics"));
        f.render_widget(metrics_widget, right_chunks[2]);

        // Help text
        let help_text = "r/F5: Refresh | Esc: Back";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, main_chunks[2]);
    }
}
