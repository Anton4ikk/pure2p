//! Core types for TUI screens and navigation

/// Application screens
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// Main menu for navigation
    MainMenu,
    /// Share contact token screen
    ShareContact,
    /// Import contact from token screen
    ImportContact,
    /// List of all chats
    ChatList,
    /// Individual chat view
    ChatView,
    /// Settings configuration
    Settings,
    /// Network diagnostics
    Diagnostics,
}

/// Main menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    /// Navigate to chat list
    ChatList,
    /// Navigate to share contact
    ShareContact,
    /// Navigate to import contact
    ImportContact,
    /// Navigate to diagnostics
    Diagnostics,
    /// Navigate to settings
    Settings,
    /// Exit application
    Exit,
}

impl MenuItem {
    /// Get all menu items in order
    pub fn all() -> Vec<Self> {
        vec![
            Self::ChatList,
            Self::ShareContact,
            Self::ImportContact,
            Self::Diagnostics,
            Self::Settings,
            Self::Exit,
        ]
    }

    /// Get display label for menu item
    pub fn label(&self) -> &str {
        match self {
            Self::ChatList => "Chat List",
            Self::ShareContact => "Share Contact",
            Self::ImportContact => "Import Contact",
            Self::Diagnostics => "Diagnostics",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }

    /// Get description for menu item
    pub fn description(&self) -> &str {
        match self {
            Self::ChatList => "View and manage your conversations",
            Self::ShareContact => "Generate and share your contact token",
            Self::ImportContact => "Import a contact from their token",
            Self::Diagnostics => "View connectivity and network diagnostics",
            Self::Settings => "Configure application settings",
            Self::Exit => "Exit Pure2P",
        }
    }
}
