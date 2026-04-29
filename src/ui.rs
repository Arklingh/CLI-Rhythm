//! Popup Rendering Utilities for TUI Music App
//!
//! This module provides functions to render interactive popup components
//! using the `ratatui` library for terminal user interfaces.
//!
//! Included Functions:
//! - `draw_popup`: Renders a centered help popup showing all keybindings
//!   and controls for navigating and managing music and playlists.
//!
//! - `draw_playlist_name_input_popup`: Displays a small, centered input box
//!   allowing users to enter a new playlist name.
//!
//! Rendering Details:
//! - Uses `Paragraph`, `Block`, `Borders`, and `Alignment` from `ratatui::widgets`.
//! - Popup dimensions are dynamically calculated based on terminal size.
//! - Styled using `ratatui::style::{Color, Style}` for consistent appearance.
//!
//! These popups improve UX by giving users clear, accessible modal interfaces
//! for help and input without leaving the TUI context.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

// Helper struct for keybindings
struct KeyBinding {
    keys: &'static str,
    description: &'static str,
}

impl KeyBinding {
    const fn new(keys: &'static str, description: &'static str) -> Self {
        KeyBinding { keys, description }
    }

    fn to_string(&self) -> String {
        format!("{}: {}", self.keys, self.description)
    }
}

// List of all application keybindings
const KEY_BINDINGS: &[KeyBinding] = &[
    KeyBinding::new("Up/Down Arrow Keys", "Navigate songs"),
    KeyBinding::new("Ctrl + Spacebar", "Play/Stop"),
    KeyBinding::new("Ctrl + P", "Pause/Unpause"),
    KeyBinding::new("Ctrl + M", "Mute/Unmute"),
    KeyBinding::new("Ctrl + S", "Change search criteria"),
    KeyBinding::new("Ctrl + T", "Change sorting criteria"),
    KeyBinding::new("Ctrl + Left/Right Arrow Keys", "Adjust Volume"),
    KeyBinding::new("Ctrl + L", "Next song"),
    KeyBinding::new("Ctrl + H", "Previous song"),
    KeyBinding::new("Left Arrow Key", "-5 seconds on current song"),
    KeyBinding::new("Right Arrow Key", "+5 seconds on current song"),
    KeyBinding::new("Backspace", "Delete characters in the search bar"),
    KeyBinding::new("Ctrl + A", "Select a song to be added to the new playlist"),
    KeyBinding::new("Ctrl + C", "New playlist name input popup"),
    KeyBinding::new("Ctrl + K", "Move playlist selection up"),
    KeyBinding::new("Ctrl + J", "Move playlist selection down"),
    KeyBinding::new("Enter", "Create a new playlist with given name"),
    KeyBinding::new("Ctrl + X", "Delete selected playlist"),
    KeyBinding::new("Ctrl + R", "Enable/disable song repeat"),
    KeyBinding::new("F1", "Toggle Controls Popup"),
    KeyBinding::new("Esc or F1", "Close Popup"),
];

fn render_centered_popup(f: &mut Frame, title: Option<&str>, width: u16, height: u16) -> Rect {
    let size = f.area();
    let popup_area = Rect::new(
        (size.width.saturating_sub(width)) / 2,
        (size.height.saturating_sub(height)) / 2,
        width,
        height,
    );

    f.render_widget(ratatui::widgets::Clear, popup_area);

    let block_title = title.unwrap_or("").to_string();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(block_title);

    f.render_widget(&block, popup_area);

    block.inner(popup_area)
}

pub fn draw_popup(f: &mut Frame) {
    let size = f.area();
    let popup_width = size.width / 3;
    let popup_height = size.height / 3 + 10;

    let inner_area = render_centered_popup(f, Some("Controls"), popup_width, popup_height);

    let key_bindings_text: Vec<String> = KEY_BINDINGS.iter().map(|kb| kb.to_string()).collect();

    let popup_text = Paragraph::new(key_bindings_text.join("\n"))
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(popup_text, inner_area);

}

pub fn draw_playlist_name_input_popup(f: &mut Frame, input: &str) {
    let size = f.area();
    let popup_width = size.width / 4;
    let popup_height = size.height / 8;

    let inner_block_area = render_centered_popup(f, Some("Enter Playlist Name"), popup_width, popup_height);

    let input_text = Paragraph::new(input)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(input_text, inner_block_area);

}

/* fn render(f: &mut Frame, app: &mut MyApp, sink: &Sink) {

} */
