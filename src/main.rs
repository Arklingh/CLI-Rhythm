//! # CLI-Rhythm
//!
//! CLI-Rhythm is a terminal-based music player written in Rust, designed for a minimalistic and efficient command-line interface. It allows users to manage and play their music collection directly from the terminal, offering features such as sorting, searching, and playback controls. Built with a focus on simplicity and performance, CLI-Rhythm provides an intuitive experience for music enthusiasts who prefer a text-based environment.
//!
//! ## Features
//! - Play music files from supported formats.
//! - Sort and search for songs by title, artist, or album.
//! - Navigate through a list of songs with ease.
//! - Minimal resource usage with a clean terminal interface.
//! - Control playback with keyboard shortcuts.
//! - Save and load playlists for quick access.

extern crate crossterm;
extern crate ratatui;

mod app;
mod input_handler;
mod song;
mod ui;
mod utils;

use app::MyApp;
use crossterm::event::{poll, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use ratatui::widgets::ListState;
use ratatui_image::picker::Picker;
use rodio::{OutputStream, Sink};
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ui::render;
use utils::sort_songs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = ratatui::init();
    let picker = Picker::from_fontsize((7, 14));
    let mut exit_code = false;

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    let mut myapp = MyApp::new();
    if let Some(config_dir) = dirs::config_local_dir() {
        let config_path = config_dir.join("cli-rhythm");
        if let Some(path_str) = config_path.to_str() {
            if let Err(e) = myapp.load_playlists(path_str) {
                eprintln!("Warning: Could not load playlists: {}", e);
            }
        } else {
            eprintln!("Warning: Could not convert config path to string");
        }
    } else {
        eprintln!("Warning: Could not find config directory");
    }
    myapp.load_songs();

    let mut playlist_scroll_state = ListState::default();
    let mut song_scroll_state = ListState::default();

    sort_songs(&mut myapp.songs, &myapp.sort_criteria);

    let (_stream, stream_handle) = OutputStream::try_default().map_err(|e| {
        eprintln!("Error: Could not initialize audio output: {}", e);
        e
    })?;
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).map_err(|e| {
        eprintln!("Error: Could not create audio sink: {}", e);
        e
    })?));

    // Initial filtered songs update
    myapp.update_filtered_songs();

    // Run event loop
    loop {
        // Get all sink state in a single lock, handling poisoned lock
        let (song_time, _is_paused, _volume) = match sink.lock() {
            Ok(guard) => (guard.get_pos(), guard.is_paused(), guard.volume()),
            Err(poisoned) => {
                eprintln!("Warning: Audio sink lock was poisoned, recovering...");
                let guard = poisoned.into_inner();
                (guard.get_pos(), guard.is_paused(), guard.volume())
            }
        };
        myapp.song_time = Some(song_time);

        if let Some(playlist_index) = playlist_scroll_state.selected() {
            myapp.selected_playlist_index = playlist_index;
        };

        // Validate and update selected song from scroll state
        if let Some(index) = song_scroll_state.selected() {
            if let Some(song) = myapp.filtered_songs.get(index) {
                myapp.selected_song_id = Some(song.id);
            } else if !myapp.filtered_songs.is_empty() {
                // Index out of bounds, reset to first song
                song_scroll_state.select_first();
                myapp.selected_song_id = Some(myapp.filtered_songs[0].id);
                myapp.list_offset = 0;
            } else {
                myapp.selected_song_id = None;
            }
        } else if !myapp.filtered_songs.is_empty() {
            // No selection but songs exist, select first
            song_scroll_state.select_first();
            myapp.selected_song_id = Some(myapp.filtered_songs[0].id);
            myapp.list_offset = 0;
        } else {
            myapp.selected_song_id = None;
        }

        // Handle audio playback tick (song end detection, auto-advance)
        myapp.tick(&sink);

        // Render UI
        terminal.draw(|f| {
            render(f, &mut myapp, &sink, &picker, &mut playlist_scroll_state, &mut song_scroll_state);
        })?;

        // Handle input events
        if poll(Duration::from_millis(200))? {
            let old_search_text = myapp.search_text.clone();
            let old_search_criteria = myapp.search_criteria.clone();
            let old_playlist_index = myapp.selected_playlist_index;

            match crossterm::event::read()? {
                Event::Key(key) => {
                    input_handler::handle_key_event(
                        key,
                        &mut myapp,
                        &sink,
                        &mut exit_code,
                        &mut playlist_scroll_state,
                        &mut song_scroll_state,
                    );
                    if exit_code {
                        break;
                    }
                }
                _ => {}
            }

            // Update filtered songs only after input handling when state actually changes
            let needs_filter_update = old_search_text != myapp.search_text
                || old_search_criteria != myapp.search_criteria
                || old_playlist_index != myapp.selected_playlist_index;

            if needs_filter_update {
                myapp.update_filtered_songs();
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
    Ok(())
}
