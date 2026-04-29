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

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use rodio::Sink;
use std::ops::Sub;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::app::MyApp;
use crate::utils::SearchCriteria;

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

pub fn render(
    f: &mut Frame,
    app: &mut MyApp,
    sink: &Arc<Mutex<Sink>>,
    picker: &Picker,
    playlist_scroll_state: &mut ratatui::widgets::ListState,
    song_scroll_state: &mut ratatui::widgets::ListState,
) {
    // Get sink state (volume, is_paused) - handle poisoned lock
    let (volume, is_paused) = match sink.lock() {
        Ok(guard) => (guard.volume(), guard.is_paused()),
        Err(poisoned) => {
            eprintln!("Warning: Audio sink lock was poisoned, recovering...");
            let guard = poisoned.into_inner();
            (guard.volume(), guard.is_paused())
        }
    };

    // Build search bar title
    let search_bar_title = match app.search_criteria {
        SearchCriteria::Title => "Search by Title",
        SearchCriteria::Artist => "Search by Artist",
        SearchCriteria::Album => "Search by Album",
    };

    // Search bar widget
    let search_bar = Paragraph::new(Text::raw(format!("{}", app.search_text)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(search_bar_title),
        )
        .style(Style::default().fg(Color::White));

    // Selected song info
    let selected_song_details = if let Some(song) = app.selected_song_id.and_then(|id| app.find_song_by_id(id)) {
        format!(
            "Artist: {}\nSong: {}\nAlbum: {}\nDuration: {:02}:{:02}",
            song.artist,
            song.title,
            song.album,
            (song.duration / 60.0).floor(),
            (song.duration % 60.0).round()
        )
    } else {
        "No song selected".to_string()
    };

    let selected_song_info = Paragraph::new(selected_song_details)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Selected Song"),
        )
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    // Currently playing song info
    let playing_song_details = if let Some(song_id) = app.currently_playing_song {
        if let Some(song) = app.find_song_by_id(song_id) {
            format!(
                "Artist: {}\nSong: {}\nAlbum: {}\nDuration: {:02}:{:02}",
                song.artist,
                song.title,
                song.album,
                (song.duration / 60.0).floor(),
                (song.duration % 60.0).round()
            )
        } else {
            "No song playing".to_string()
        }
    } else {
        "No song playing".to_string()
    };

    let playing_song_info = Paragraph::new(playing_song_details)
        .block(Block::default())
        .style(Style::default().fg(Color::White));

    // Playing song cover - loaded on demand to save memory
    let playing_song_cover = app.currently_playing_song
        .and_then(|song_id| app.find_song_by_id(song_id))
        .and_then(|song| song.load_cover());

    // Determine song for progress bar
    let song_id = app.currently_playing_song
        .or(app.selected_song_id)
        .unwrap_or_else(|| app.songs.first().map(|song| song.id).unwrap_or_default());

    // Progress ratio calculation
    let progress_ratio = match app.find_song_by_id(song_id).cloned() {
        Some(song) if song.duration > 0.0 && !is_paused => {
            if let Some(song_time) = app.song_time {
                let elapsed_time = song_time.as_secs_f64().min(song.duration);
                if elapsed_time >= song.duration {
                    0.0
                } else {
                    elapsed_time / song.duration
                }
            } else {
                0.0
            }
        }
        Some(song) if song.duration > 0.0 && is_paused => {
            let mut ratio: f64 = 0.0;
            if let Some(song_time) = app.song_time {
                if let Some(paused_time) = app.paused_time {
                    let elapsed_time = song_time.as_secs_f64().min(song.duration);
                    ratio = (elapsed_time - paused_time.as_secs_f64()).max(0.0) / song.duration;
                }
            }
            ratio
        }
        _ => 0.0,
    };

    // Song progress widget
    let song_progress = if let Some(song) = app.find_song_by_id(song_id).cloned() {
        let elapsed_time = if let Some(paused_time) = app.paused_time {
            app.song_time
                .unwrap_or(Duration::default())
                .as_secs_f64()
                .sub(paused_time.as_secs_f64())
                .min(song.duration)
        } else {
            app.song_time
                .unwrap_or(Duration::default())
                .as_secs_f64()
                .min(song.duration)
        };
        let elapsed_minutes = (elapsed_time / 60.0).floor() as u64;
        let elapsed_seconds = (elapsed_time % 60.0).round() as u64;
        let duration_minutes = (song.duration / 60.0).floor() as u64;
        let duration_seconds = (song.duration % 60.0).round() as u64;

        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::LightBlue))
            .label(format!(
                "{:02}:{:02}/{:02}:{:02}",
                elapsed_minutes, elapsed_seconds, duration_minutes, duration_seconds
            ))
            .ratio(progress_ratio)
    } else {
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::LightBlue))
            .label("No song selected")
            .ratio(0.0)
    };

    // Volume bar widget
    let volume_bar = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Volume"))
        .gauge_style(Style::default().fg(Color::LightBlue))
        .label(format!("{:.0}%", volume * 100.0))
        .ratio(volume as f64);

    // Hint widget
    let hint = Paragraph::new("F1 for controls")
        .style(
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )
        .alignment(Alignment::Right);

    // Layout structure
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Fill(1)])
        .split(f.area());

    let song_tab_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(7),
            Constraint::Percentage(86),
            Constraint::Percentage(7),
        ])
        .split(vertical_layout[0]);

    f.render_widget(search_bar, song_tab_layout[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(song_tab_layout[1]);

    // Song list items
    let song_items: Vec<ListItem> = app
        .filtered_songs
        .iter()
        .map(|song| {
            let mut style = Style::default();
            if app.chosen_song_ids.contains(&song.id) {
                style = Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::RAPID_BLINK);
            }
            ListItem::new(song.title.clone()).style(style)
        })
        .collect();

    let song_list = List::new(song_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Songs-------------------------------------------------------------------Sort by: {}",
                    app.sort_criteria.to_string(),
                )),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    // Playlist items
    let playlist_items: Vec<ListItem> = app
        .playlists
        .iter()
        .map(|(playlist_name, _songs)| ListItem::new(playlist_name.clone()))
        .collect();

    let playlist_list = List::new(playlist_items)
        .block(Block::default().borders(Borders::ALL).title("Playlists"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(playlist_list, chunks[0], playlist_scroll_state);
    f.render_stateful_widget(song_list, chunks[1], song_scroll_state);

    // Songs info area
    let songs_info = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[2]);

    f.render_widget(selected_song_info, songs_info[0]);

    // Currently playing song block
    let playing_song_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Current song----Repeat:{}", if app.repeat_song { "✅" } else { "❌" }));

    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Fill(1)])
        .split(playing_song_block.inner(songs_info[1]));

    f.render_widget(playing_song_info, inner_layout[0]);
    if let Some(cover) = playing_song_cover {
        let mut pic = picker.new_resize_protocol((*cover).clone());
        let img = StatefulImage::default();
        f.render_stateful_widget(img, inner_layout[1], &mut pic);
    }
    f.render_widget(playing_song_block, songs_info[1]);

    // Footer with progress and volume
    let footer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(song_tab_layout[2]);

    f.render_widget(song_progress, footer[0]);
    f.render_widget(volume_bar, footer[1]);

    // Popups
    if app.hint_popup_state.visible {
        let _ = draw_popup(f);
    }

    if app.playlist_input_popup.visible {
        let _ = draw_playlist_name_input_popup(f, &app.playlist_name_input);
    }

    // Hint text at bottom right
    f.render_widget(
        hint,
        Rect::new(
            f.area().width.saturating_sub(20),
            f.area().height - 1,
            20,
            1,
        ),
    );
}
