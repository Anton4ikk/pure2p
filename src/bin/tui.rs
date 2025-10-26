//! Pure2P TUI (Terminal User Interface)
//!
//! A terminal-based user interface for Pure2P messaging.

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pure2p::tui::{App, Screen, ui::ui};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new()?;

    // Trigger background connectivity diagnostics on startup
    app.trigger_startup_connectivity();

    // Run main loop
    let res = run_app(&mut terminal, &mut app);

    // Save application state before exit
    if let Err(e) = app.save_state() {
        eprintln!("Warning: Failed to save application state: {}", e);
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Poll for startup connectivity completion (runs in background on all screens)
        // BUT: skip if on Diagnostics screen, since poll_diagnostics_result() handles it
        if app.connectivity_result.is_none() && app.current_screen != Screen::Diagnostics {
            app.poll_startup_connectivity();
        }

        // Handle startup sync screen updates
        if app.current_screen == Screen::StartupSync {
            app.update_startup_sync();

            // Check if sync is complete
            if let Some(sync) = &app.startup_sync_screen {
                if sync.is_complete {
                    // Wait a moment to show final stats
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }

        // Poll for diagnostics refresh completion
        // This handles BOTH startup connectivity and manual refresh when on Diagnostics screen
        if app.current_screen == Screen::Diagnostics {
            app.poll_diagnostics_result();
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.current_screen {
                    Screen::StartupSync => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                // Allow user to skip or dismiss when complete
                                if let Some(sync) = &app.startup_sync_screen {
                                    if sync.is_complete {
                                        app.complete_startup_sync();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::MainMenu => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous();
                            }
                            KeyCode::Enter => {
                                app.select();
                            }
                            // Quick access hotkeys
                            KeyCode::Char('n') => {
                                app.show_diagnostics_screen();
                            }
                            KeyCode::Char('c') => {
                                app.show_chat_list_screen();
                            }
                            KeyCode::Char('s') => {
                                app.show_share_contact_screen();
                            }
                            KeyCode::Char('i') => {
                                app.show_import_contact_screen();
                            }
                            _ => {}
                        }
                    }
                    Screen::ShareContact => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char('c') => {
                                if let Some(screen) = &mut app.share_contact_screen {
                                    screen.copy_to_clipboard();
                                }
                            }
                            KeyCode::Char('s') => {
                                if let Some(screen) = &mut app.share_contact_screen {
                                    screen.save_to_file();
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::ImportContact => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.add_char(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.backspace();
                                }
                            }
                            KeyCode::Enter => {
                                // Parse token first
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.parse_token();
                                }
                                // Then get contact and import (separate scope to avoid double borrow)
                                let contact_to_import = app.import_contact_screen.as_ref()
                                    .and_then(|screen| screen.get_contact().cloned());
                                if let Some(contact) = contact_to_import {
                                    app.import_contact(contact);
                                }
                            }
                            KeyCode::Char('v') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.paste_from_clipboard();
                                }
                            }
                            KeyCode::Delete => {
                                if let Some(screen) = &mut app.import_contact_screen {
                                    screen.clear();
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::ChatList => {
                        // Check if delete confirmation popup is shown
                        if let Some(chat_list) = &app.chat_list_screen {
                            if chat_list.show_delete_confirmation {
                                // Handle popup navigation
                                match key.code {
                                    KeyCode::Char('y') | KeyCode::Enter => {
                                        app.confirm_delete_chat();
                                    }
                                    KeyCode::Char('n') | KeyCode::Esc => {
                                        app.cancel_delete_chat();
                                    }
                                    _ => {}
                                }
                                continue; // Don't process other keys while popup is shown
                            }
                        }

                        // Normal chat list navigation
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if let Some(screen) = &mut app.chat_list_screen {
                                    screen.next(app.app_state.chats.len());
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if let Some(screen) = &mut app.chat_list_screen {
                                    screen.previous(app.app_state.chats.len());
                                }
                            }
                            KeyCode::Enter => {
                                app.open_selected_chat();
                            }
                            KeyCode::Char('d') | KeyCode::Delete => {
                                app.show_delete_confirmation();
                            }
                            _ => {}
                        }
                    }
                    Screen::ChatView => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_chat_list();
                            }
                            KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    screen.add_char(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    screen.backspace();
                                }
                            }
                            KeyCode::Enter => {
                                app.send_message_in_chat();
                            }
                            KeyCode::Up => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    screen.scroll_up();
                                }
                            }
                            KeyCode::Down => {
                                if let Some(screen) = &mut app.chat_view_screen {
                                    // Calculate max offset based on message count
                                    let max_offset = app.app_state.chats
                                        .iter()
                                        .find(|c| c.contact_uid == screen.contact_uid)
                                        .map(|c| c.messages.len().saturating_sub(10))
                                        .unwrap_or(0);
                                    screen.scroll_down(max_offset);
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::Settings => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.add_char(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.backspace();
                                }
                            }
                            KeyCode::Enter => {
                                // Validate input first
                                let validated_minutes = app.settings_screen.as_mut()
                                    .and_then(|screen| screen.validate());

                                // If valid, update app_state and save
                                if let Some(minutes) = validated_minutes {
                                    app.app_state.settings.retry_interval_minutes = minutes;
                                    app.app_state.settings.global_retry_interval_ms = (minutes as u64) * 60 * 1000;

                                    // Save app state
                                    let _ = app.save_state();

                                    // Update screen with success message
                                    if let Some(screen) = &mut app.settings_screen {
                                        screen.set_saved_message(minutes);
                                    }
                                }
                            }
                            KeyCode::Delete => {
                                if let Some(screen) = &mut app.settings_screen {
                                    screen.clear_input();
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::Diagnostics => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Char('r') | KeyCode::F(5) => {
                                if let Some(screen) = &mut app.diagnostics_screen {
                                    if !screen.is_refreshing {
                                        screen.start_refresh();
                                        app.trigger_diagnostics_refresh();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
