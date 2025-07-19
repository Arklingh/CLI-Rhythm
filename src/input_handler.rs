//! Key Event Handler for TUI Music Application
//!
//! This module defines the `handle_key_event` function which processes user input
//! via keyboard events using the `crossterm` crate. It manages user interactions
//! such as navigating playlists and songs, playing/pausing/stopping music,
//! adjusting volume, editing search and playlist input fields, and toggling playback states.
//!
//! Supported Features:
//! - Song and playlist selection with wrapping and scrolling
//! - Playback control (play, pause, next, previous, stop)
//! - Volume control and mute toggle
//! - Playlist input popup activation
//! - Search field character input and deletion
//! - Switching between search criteria (title, artist, album)
//! - Changing sort criteria
//!
//! The function modifies the `MyApp` application state, controls a shared `rodio::Sink`
//! for audio playback, and tracks view-related parameters for rendering playlists/songs.

use crate::app::MyApp;
use crate::utils::SearchCriteria;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::widgets::ListState;
use rodio::Sink;
use std::fs::{self};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

pub fn handle_key_event(
    key: KeyEvent,
    myapp: &mut MyApp,
    sink: &Arc<Mutex<Sink>>,
    exit_flag: &mut bool,
    playlist_scroll_state: &mut ListState,
    song_scroll_state: &mut ListState,
) {
    match key {
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            let _ = myapp.save_playlist();
            *exit_flag = true;
        }
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(curr_index) = song_scroll_state.selected() {
                if myapp.filtered_songs.get(curr_index + 1).is_some() {
                    song_scroll_state.select_next();
                } else {
                    song_scroll_state.select_first();
                }
            } else {
                song_scroll_state.select_first();
            }
        }
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(curr_index) = song_scroll_state.selected() {
                if myapp.filtered_songs.get(curr_index - 1).is_some() {
                    song_scroll_state.select_previous();
                } else {
                    song_scroll_state.select(Some(myapp.filtered_songs.len() - 1));
                }
            } else {
                song_scroll_state.select_first();
            }
        }
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(curr_index) = playlist_scroll_state.selected() {
                if curr_index != myapp.playlists.len() - 1 {
                    playlist_scroll_state.select_next();
                } else {
                    playlist_scroll_state.select_first();
                }
            } else {
                playlist_scroll_state.select_first();
            }
        }
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(curr_index) = playlist_scroll_state.selected() {
                if curr_index != 0 {
                    playlist_scroll_state.select_previous();
                } else {
                    playlist_scroll_state.select(Some(myapp.playlists.len() - 1));
                }
            } else {
                playlist_scroll_state.select_first();
            }
        }
        KeyEvent {
            code: KeyCode::Char(' '),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(selected_id) = myapp.selected_song_id {
                if let Some(index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == selected_id)
                {
                    if myapp.currently_playing_song.is_none()
                        || Some(selected_id) != myapp.currently_playing_song
                    {
                        sink.lock().unwrap().clear();
                        let selected_song = &myapp.filtered_songs[index];
                        selected_song.play(&sink);
                        myapp.song_time = Some(Duration::default());
                        myapp.currently_playing_song = Some(selected_id);

                        // Set is_playing field to true
                        if let Some(song) = myapp.songs.iter_mut().find(|s| s.id == selected_id) {
                            song.is_playing = true;
                        }
                    } else {
                        // Stop the currently playing song
                        sink.lock().unwrap().clear();
                        myapp.song_time = None;
                        myapp.currently_playing_song = None;

                        // Set is_playing field to false
                        if let Some(song) = myapp.songs.iter_mut().find(|s| s.id == selected_id) {
                            song.is_playing = false;
                        }
                    }
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if sink.lock().unwrap().is_paused() {
                if let Some(current_id) = myapp.currently_playing_song {
                    if let Some(song) = myapp.songs.iter_mut().find(|s| s.id == current_id) {
                        sink.lock().unwrap().play();
                        song.is_playing = true;
                    }
                    // Calculate elapsed time during the pause
                    if let Some(paused_at) = myapp.paused_time {
                        let elapsed_during_pause = paused_at;
                        myapp.song_time = myapp.song_time.map(|t| t + elapsed_during_pause);
                        myapp.paused_time = None;
                    }
                }
            } else {
                if let Some(current_id) = myapp.currently_playing_song {
                    if let Some(song) = myapp.songs.iter_mut().find(|s| s.id == current_id) {
                        sink.lock().unwrap().pause();
                        song.is_playing = false;
                        // Record the time when playback was paused
                        myapp.paused_time = Some(Duration::default());
                    }
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            myapp.playlist_input_popup.visible = true;
        }
        KeyEvent {
            code: KeyCode::Char('h'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(current_id) = myapp.currently_playing_song {
                if let Some(current_index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == current_id)
                {
                    if current_index > 0 {
                        let previous_id = myapp.filtered_songs[current_index - 1].id;
                        sink.lock().unwrap().clear();
                        if let Some(previous_song) = myapp
                            .filtered_songs
                            .iter()
                            .find(|song| song.id == previous_id)
                        {
                            previous_song.play(&sink);
                            myapp.currently_playing_song = Some(previous_id);
                            myapp.selected_song_id = Some(previous_id);
                            myapp.song_time = Some(Duration::default());
                            myapp.paused_time = None; // Reset paused time when starting a new song
                        }
                    }
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(current_id) = myapp.currently_playing_song {
                if let Some(current_index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == current_id)
                {
                    if current_index < myapp.filtered_songs.len() - 1 {
                        let next_id = myapp.filtered_songs[current_index + 1].id;
                        sink.lock().unwrap().clear();
                        if let Some(next_song) =
                            myapp.filtered_songs.iter().find(|song| song.id == next_id)
                        {
                            next_song.play(&sink);
                            myapp.selected_song_id = Some(next_id);
                            myapp.currently_playing_song = Some(next_id);
                            myapp.song_time = Some(Duration::default());
                            myapp.paused_time = None; // Reset paused time when starting a new song
                        }
                    }
                }
            }
        }
        KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            // Decrease volume by 5%
            let sink = &mut sink.lock().unwrap();
            let volume = sink.volume();
            if volume >= 0.05 {
                sink.set_volume(volume - 0.05);
            }
        }
        KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            // Increase volume by 5%
            let sink = &mut sink.lock().unwrap();
            let volume = sink.volume();
            if volume <= 0.95 {
                sink.set_volume(volume + 0.05);
            }
        }
        KeyEvent {
            code: KeyCode::Char('m'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            let sink = &mut sink.lock().unwrap();
            if sink.volume() > 0.0 {
                // Mute music
                myapp.previous_volume = sink.volume(); // Save current volume
                sink.set_volume(0.0);
            } else {
                // Unmute music
                sink.set_volume(myapp.previous_volume); // Restore previous volume
            }
        }
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if myapp.playlist_input_popup.visible {
                myapp.playlist_name_input.push(c);
            } else {
                myapp.search_text.push(c);
            }
        }
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if myapp.playlist_input_popup.visible {
                myapp
                    .playlist_name_input
                    .push(c.to_uppercase().last().unwrap());
            } else {
                myapp.search_text.push(c.to_uppercase().last().unwrap());
            }
        }
        KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if myapp.playlist_input_popup.visible {
                myapp.playlist_name_input.pop();
            } else {
                myapp.search_text.pop();
            }
        }
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            myapp.search_criteria = match myapp.search_criteria {
                SearchCriteria::Title => SearchCriteria::Artist,
                SearchCriteria::Artist => SearchCriteria::Album,
                SearchCriteria::Album => SearchCriteria::Title,
            };
        }
        KeyEvent {
            code: KeyCode::Char('t'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            myapp.set_sort_criteria(myapp.sort_criteria.next());
        }
        KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(current_id) = myapp.currently_playing_song {
                if let Some(_) = myapp.songs.iter().find(|song| song.id == current_id) {
                    let sink = sink.lock().unwrap();
                    let new_position = sink.get_pos() + Duration::from_secs(5);
                    if let Ok(_) = sink.try_seek(new_position) {
                    } else {
                    };
                    myapp.song_time = Some(new_position);
                }
            }
        }
        KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(current_id) = myapp.currently_playing_song {
                if let Some(_) = myapp.songs.iter().find(|song| song.id == current_id) {
                    let sink = sink.lock().unwrap();
                    let new_position = sink.get_pos().saturating_sub(Duration::from_secs(5));
                    sink.try_seek(new_position).unwrap();
                    myapp.song_time = Some(new_position);
                }
            }
        }
        KeyEvent {
            code: KeyCode::F(1),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            myapp.hint_popup_state.toggle();
        }
        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            // Close the popup if it's open
            myapp.playlist_input_popup.visible = false;
            myapp.playlist_name_input = String::new();
            myapp.hint_popup_state.visible = false;
        }
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            match (
                myapp.playlist_name_input.is_empty(),
                myapp.chosen_song_ids.is_empty(),
            ) {
                (true, true) => {
                    myapp.playlist_name_input = "Need a name and at least 1 song".to_string()
                }
                (true, false) => myapp.playlist_name_input = "Need a name ".to_string(),
                (false, true) => myapp.playlist_name_input = "Need at least 1 song".to_string(),
                (false, false) => {
                    myapp.playlist_input_popup.visible = false;
                    myapp.playlists.insert(
                        myapp.playlist_name_input.clone(),
                        myapp.chosen_song_ids.clone(),
                    );
                    myapp.chosen_song_ids.clear();
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            let selected_song_id = myapp
                .selected_song_id
                .unwrap_or(Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"rust-lang.org"));
            match myapp.chosen_song_ids.contains(&selected_song_id) {
                true => {
                    myapp.chosen_song_ids.retain(|id| *id != selected_song_id);
                }
                false => {
                    myapp.chosen_song_ids.push(selected_song_id);
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            // Get the playlist name at the selected index
            let playlist_name = myapp
                .playlists
                .keys()
                .nth(myapp.selected_playlist_index)
                .cloned();

            if let Some(name) = playlist_name {
                myapp.playlists.remove(&name);
                myapp.selected_playlist_index = 0;
                if let Some(roaming_dir) = dirs::config_local_dir() {
                    let myapp_dir: PathBuf = roaming_dir.join("cli-rhythm");
                    let playlist_file_path = myapp_dir.join(format!("{name}.m3u"));
                    let _ = fs::remove_file(playlist_file_path);
                }
            }
        }
        KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            myapp.repeat_song = !myapp.repeat_song;
        }
        _ => {}
    }
}
