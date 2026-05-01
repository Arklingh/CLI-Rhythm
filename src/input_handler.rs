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
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;
use rodio::Sink;
use std::fs::{self};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Processes keyboard input and updates application state accordingly.
///
/// Handles all user interactions including:
/// - Navigation (Up/Down arrows for songs, Ctrl+K/J for playlists)
/// - Playback control (Ctrl+Space play/stop, Ctrl+P pause, Ctrl+H/L prev/next)
/// - Seeking (Left/Right arrows for -5/+5 seconds)
/// - Volume control (Ctrl+Left/Right arrows, Ctrl+M mute)
/// - Search and filtering (Ctrl+S change criteria, typing for search)
/// - Playlist management (Ctrl+C create, Ctrl+A add song, Ctrl+X delete)
/// - UI toggles (F1 help, Esc close popup)
///
/// # Arguments
/// * `key` - The keyboard event to process
/// * `myapp` - Mutable reference to application state
/// * `sink` - Audio sink for playback control
/// * `exit_flag` - Set to true when user requests to quit (Ctrl+Q)
/// * `playlist_scroll_state` - State for playlist list widget navigation
/// * `song_scroll_state` - State for song list widget navigation
///
/// # Note
/// Ignores key release events (only processes KeyEventKind::Press).
/// Handles poisoned mutex locks gracefully when accessing the audio sink.
pub fn handle_key_event(
    key: KeyEvent,
    myapp: &mut MyApp,
    sink: &Arc<Mutex<Sink>>,
    exit_flag: &mut bool,
    playlist_scroll_state: &mut ListState,
    song_scroll_state: &mut ListState,
) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
            let _ = myapp.save_playlist();
            *exit_flag = true;
        }
        (KeyCode::Down, KeyModifiers::NONE) => {
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
        (KeyCode::Up, KeyModifiers::NONE) => {
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
        (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
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
        (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
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
        (KeyCode::Char(' '), KeyModifiers::CONTROL) => {
            if let Some(selected_id) = myapp.selected_song_id {
                // Song is selected
                if let Some(filtered_song) = myapp
                    .filtered_songs
                    .iter()
                    .find(|song| song.id == selected_id)
                {
                    // Find the song
                    if let Some(mut_song_in_app) =
                        myapp.songs.iter_mut().find(|song| song.id == selected_id)
                    {
                        // Find the mutable song
                        if myapp.currently_playing_song.is_none()
                            || Some(selected_id) != myapp.currently_playing_song
                        {
                            // Play the song
                            if let Err(e) = filtered_song.play(&sink) {
                                eprintln!("Error playing song: {}", e);
                            }
                            myapp.song_time = Some(Duration::default());
                            myapp.currently_playing_song = Some(selected_id);
                            mut_song_in_app.is_playing = true;
                        } else {
                            // Stop the currently playing song
                            {
                                let sink_guard = match sink.lock() {
                                    Ok(guard) => guard,
                                    Err(poisoned) => poisoned.into_inner(),
                                };
                                sink_guard.clear();
                            }
                            myapp.song_time = None;
                            myapp.currently_playing_song = None;
                            mut_song_in_app.is_playing = false;
                        }
                    } else {
                        // Should not happen!
                        eprintln!("Warning: Selected song in filtered list not found in main song collection.");
                    }
                }
            }
        }
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            let sink_guard = match sink.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            if sink_guard.is_paused() {
                if let Some(current_id) = myapp.currently_playing_song {
                    if let Some(song) = myapp.songs.iter_mut().find(|s| s.id == current_id) {
                        sink_guard.play();
                        song.is_playing = true;
                    }
                    if let Some(paused_at) = myapp.paused_time {
                        myapp.song_time = myapp.song_time.map(|t| t + paused_at);
                        myapp.paused_time = None;
                    }
                }
            } else {
                if let Some(current_id) = myapp.currently_playing_song {
                    if let Some(song) = myapp.songs.iter_mut().find(|s| s.id == current_id) {
                        sink_guard.pause();
                        song.is_playing = false;
                        myapp.paused_time = Some(Duration::default());
                    }
                }
            }
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            myapp.playlist_input_popup.visible = true;
        }
        (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
            if let Some(current_id) = myapp.currently_playing_song {
                if let Some(current_index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == current_id)
                {
                    if current_index > 0 {
                        let previous_song = &myapp.filtered_songs[current_index - 1];
                        if let Ok(sink_guard) = sink.lock() {
                            sink_guard.clear();
                        }
                        if let Err(e) = previous_song.play(&sink) {
                            eprintln!("Error playing previous song: {}", e);
                        }
                        myapp.currently_playing_song = Some(previous_song.id);
                        myapp.selected_song_id = Some(previous_song.id);
                        myapp.song_time = Some(Duration::default());
                        myapp.paused_time = None;
                    }
                }
            }
        }
        (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
            if let Some(current_id) = myapp.currently_playing_song {
                if let Some(current_index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == current_id)
                {
                    if current_index < myapp.filtered_songs.len() - 1 {
                        let next_song = myapp.filtered_songs[current_index + 1].clone();
                        if let Ok(sink_guard) = sink.lock() {
                            sink_guard.clear();
                        }
                        if let Err(e) = next_song.play(&sink) {
                            eprintln!("Error playing next song: {}", e);
                        }
                        myapp.selected_song_id = Some(next_song.id);
                        myapp.currently_playing_song = Some(next_song.id);
                        myapp.song_time = Some(Duration::default());
                        myapp.paused_time = None;
                    }
                }
            }
        }
        (KeyCode::Left, KeyModifiers::CONTROL) => {
            if let Ok(sink) = sink.lock().as_mut() {
                let volume = sink.volume();
                if volume >= 0.05 {
                    sink.set_volume(volume - 0.05);
                }
            }
        }
        (KeyCode::Right, KeyModifiers::CONTROL) => {
            if let Ok(sink) = sink.lock().as_mut() {
                let volume = sink.volume();
                if volume <= 0.95 {
                    sink.set_volume(volume + 0.05);
                }
            }
        }
        (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
            if let Ok(sink) = sink.lock().as_mut() {
                if sink.volume() > 0.0 {
                    myapp.previous_volume = sink.volume();
                    sink.set_volume(0.0);
                } else {
                    sink.set_volume(myapp.previous_volume);
                }
            }
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            if myapp.playlist_input_popup.visible {
                myapp.playlist_name_input.push(c);
            } else {
                myapp.search_text.push(c);
            }
        }
        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            if myapp.playlist_input_popup.visible {
                myapp.playlist_name_input.extend(c.to_uppercase());
            } else {
                myapp.search_text.extend(c.to_uppercase());
            }
        }
        (KeyCode::Backspace, KeyModifiers::NONE) => {
            if myapp.playlist_input_popup.visible {
                myapp.playlist_name_input.pop();
            } else {
                myapp.search_text.pop();
            }
        }
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
            myapp.search_criteria = match myapp.search_criteria {
                SearchCriteria::Title => SearchCriteria::Artist,
                SearchCriteria::Artist => SearchCriteria::Album,
                SearchCriteria::Album => SearchCriteria::Title,
            };
        }
        (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
            myapp.set_sort_criteria(myapp.sort_criteria.next());
        }
        (KeyCode::Right, KeyModifiers::NONE) => {
            if let Some(_) = myapp.currently_playing_song {
                if let Ok(sink) = sink.lock() {
                    let new_position = sink.get_pos() + Duration::from_secs(5);
                    let _ = sink.try_seek(new_position);
                    myapp.song_time = Some(new_position);
                }
            }
        }
        (KeyCode::Left, KeyModifiers::NONE) => {
            if let Some(_) = myapp.currently_playing_song {
                if let Ok(sink) = sink.lock() {
                    let new_position = sink.get_pos().saturating_sub(Duration::from_secs(5));
                    let _ = sink.try_seek(new_position);
                    myapp.song_time = Some(new_position);
                }
            }
        }
        (KeyCode::F(1), KeyModifiers::NONE) => {
            myapp.hint_popup_state.toggle();
        }
        (KeyCode::Esc, KeyModifiers::NONE) => {
            myapp.playlist_input_popup.visible = false;
            myapp.playlist_name_input.clear();
            myapp.hint_popup_state.visible = false;
        }
        (KeyCode::Enter, KeyModifiers::NONE) => {
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
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
            if let Some(selected_id) = myapp.selected_song_id {
                if myapp.chosen_song_ids.contains(&selected_id) {
                    myapp.chosen_song_ids.retain(|id| *id != selected_id);
                } else {
                    myapp.chosen_song_ids.push(selected_id);
                }
            }
        }
        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
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
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
            myapp.repeat_song = !myapp.repeat_song;
        }
        _ => {}
    }
}


/// Handles mouse events for the TUI application.
///
/// Maps mouse interactions to UI actions:
/// - Click on song list: select and play song
/// - Click on playlist: select playlist
/// - Click on progress bar: seek to position
/// - Click on volume bar: adjust volume
/// - Scroll wheel: scroll lists
fn handle_mouse_event(
    mouse: crossterm::event::MouseEvent,
    myapp: &mut MyApp,
    sink: &Arc<Mutex<Sink>>,
    playlist_scroll_state: &mut ListState,
    song_scroll_state: &mut ListState,
) {
    use crossterm::event::{MouseEventKind, MouseButton};

    let area = match myapp.last_frame_area {
        Some(a) => a,
        None => return, // No frame rendered yet
    };
    let margin = 1u16;
    let x = mouse.column;
    let y = mouse.row;

    // Calculate layout regions (must match render() layout)
    let content_area = ratatui::layout::Rect::new(
        area.x + margin,
        area.y + margin,
        area.width.saturating_sub(margin * 2),
        area.height.saturating_sub(margin * 2),
    );

    // Song tab layout: [7%, 86%, 7%]
    let top_height = (content_area.height as f32 * 0.07) as u16;
    let main_height = (content_area.height as f32 * 0.86) as u16;
    let footer_y = content_area.y + top_height + main_height;

    // Main horizontal chunks: [20%, 60%, 20%]
    let playlist_width = (content_area.width as f32 * 0.20) as u16;
    let song_list_width = (content_area.width as f32 * 0.60) as u16;
    let playlist_x = content_area.x;
    let song_list_x = playlist_x + playlist_width;
    let right_panel_x = song_list_x + song_list_width;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check if click is in song list area
            if x >= song_list_x && x < right_panel_x
                && y >= content_area.y + top_height
                && y < footer_y
            {
                let song_area_y_start = content_area.y + top_height;
                let relative_y = y.saturating_sub(song_area_y_start).saturating_sub(1); // -1 for border
                let visible_index = relative_y as usize;

                let list_offset = song_scroll_state.offset();
                let actual_index = visible_index + list_offset;
                if let Some(song) = myapp.filtered_songs.get(actual_index) {
                    song_scroll_state.select(Some(actual_index));
                    myapp.selected_song_id = Some(song.id);

                    // Double-click or click on already selected plays the song
                    if myapp.currently_playing_song != Some(song.id) {
                        if let Some(mut_song) = myapp.songs.iter_mut().find(|s| s.id == song.id) {
                            let _ = mut_song.play(sink);
                            myapp.song_time = Some(Duration::default());
                            myapp.currently_playing_song = Some(song.id);
                            mut_song.is_playing = true;
                        }
                    }
                }
            }

            // Check if click is in playlist area
            if x >= playlist_x && x < song_list_x
                && y >= content_area.y + top_height
                && y < footer_y
            {
                let playlist_area_y_start = content_area.y + top_height;
                let relative_y = y.saturating_sub(playlist_area_y_start).saturating_sub(1);
                let visible_index = relative_y as usize;
                let playlist_count = myapp.playlists.len();

                if visible_index < playlist_count {
                    playlist_scroll_state.select(Some(visible_index));
                    myapp.selected_playlist_index = visible_index;
                }
            }

            // Check if click is on progress bar (footer left 80%)
            let footer_height = content_area.height - top_height - main_height;
            if y >= footer_y && y < footer_y + footer_height
                && x >= content_area.x
                && x < content_area.x + (content_area.width as f32 * 0.80) as u16
            {
                // Seek to clicked position
                if let Some(song_id) = myapp.currently_playing_song.or(myapp.selected_song_id) {
                    if let Some(song) = myapp.find_song_by_id(song_id) {
                        if song.duration > 0.0 {
                            let progress_width = (content_area.width as f32 * 0.80) as u16;
                            let relative_x = x.saturating_sub(content_area.x).saturating_sub(1) as f64;
                            let seek_ratio = relative_x / progress_width as f64;
                            let seek_secs = (seek_ratio * song.duration).max(0.0);

                            // Update song_time to seek position
                            myapp.song_time = Some(Duration::from_secs_f64(seek_secs));

                            // Try to seek in sink
                            if let Ok(sink_guard) = sink.lock() {
                                let _ = sink_guard.try_seek(Duration::from_secs_f64(seek_secs));
                            }
                        }
                    }
                }
            }

            // Check if click is on volume bar (footer right 20%)
            let volume_bar_x = content_area.x + (content_area.width as f32 * 0.80) as u16;
            if y >= footer_y && y < footer_y + footer_height
                && x >= volume_bar_x
                && x < content_area.x + content_area.width
            {
                // Adjust volume based on click position
                let volume_width = content_area.width - (content_area.width as f32 * 0.80) as u16;
                let relative_x = x.saturating_sub(volume_bar_x).saturating_sub(1) as f32;
                let new_volume = (relative_x / volume_width as f32).clamp(0.0, 1.0);

                if let Ok(sink_guard) = sink.lock() {
                    sink_guard.set_volume(new_volume);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            // Scroll song list down
            if let Some(current) = song_scroll_state.selected() {
                let new_index = (current + 3).min(myapp.filtered_songs.len().saturating_sub(1));
                song_scroll_state.select(Some(new_index));
            } else if !myapp.filtered_songs.is_empty() {
                song_scroll_state.select(Some(0));
            }
        }
        MouseEventKind::ScrollUp => {
            // Scroll song list up
            if let Some(current) = song_scroll_state.selected() {
                let new_index = current.saturating_sub(3);
                song_scroll_state.select(Some(new_index));
            }
        }
        _ => {}
    }
}

