//! UI rendering module - screen-specific rendering functions
//!
//! This module contains the UI rendering logic organized by screen type.
//! Each screen has its own file for better maintainability.

mod main_menu;
mod share_contact;
mod import_contact;
mod chat_list;
mod chat_view;
mod settings;
mod diagnostics;
mod helpers;

use ratatui::Frame;
use crate::tui::types::Screen;
use crate::tui::app::App;

// Re-export render functions
pub use main_menu::render_main_menu;
pub use share_contact::render_share_contact;
pub use import_contact::render_import_contact;
pub use chat_list::render_chat_list;
pub use chat_view::render_chat_view;
pub use settings::render_settings;
pub use diagnostics::render_diagnostics;

// Re-export helper functions
pub use helpers::format_duration_until;

/// Main UI rendering function - dispatches to screen-specific render functions
pub fn ui(f: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::MainMenu => render_main_menu(f, app),
        Screen::ShareContact => render_share_contact(f, app),
        Screen::ImportContact => render_import_contact(f, app),
        Screen::ChatList => render_chat_list(f, app),
        Screen::ChatView => render_chat_view(f, app),
        Screen::Settings => render_settings(f, app),
        Screen::Diagnostics => render_diagnostics(f, app),
    }
}
