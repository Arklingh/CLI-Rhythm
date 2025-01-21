use crate::app::MyApp;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rodio::Sink;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;
use crate::utils::SearchCriteria;

pub fn handle_key_event(
    key: KeyEvent,
    myapp: &mut MyApp,
    sink: &Arc<Mutex<Sink>>,
    visible_song_count: usize,
    visible_playlist_count: usize,
    exit_flag: &mut bool,
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
            if let Some(selected_id) = myapp.selected_song_id {
                // Find the index of the currently selected song by Uuid
                if let Some(index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == selected_id)
                {
                    if index < myapp.filtered_songs.len() - 1 {
                        let next_song = &myapp.filtered_songs[index + 1];
                        myapp.selected_song_id = Some(next_song.id);

                        // Scroll down if selected index goes out of view
                        if let Some(new_index) = myapp
                            .filtered_songs
                            .iter()
                            .position(|song| song.id == myapp.selected_song_id.unwrap())
                        {
                            if new_index >= myapp.list_offset + visible_song_count - 1 {
                                myapp.list_offset = (new_index - visible_song_count + 2).max(0);

                                // Ensure the list_offset does not exceed the maximum allowed offset
                                myapp.list_offset = myapp.list_offset.min(
                                    myapp
                                        .filtered_songs
                                        .len()
                                        .saturating_sub(visible_song_count),
                                );
                            }
                        }
                    } else {
                        // Wrap around to the beginning
                        let first_song = &myapp.filtered_songs[0];
                        myapp.selected_song_id = Some(first_song.id);
                        myapp.list_offset = 0;
                    }
                }
            } else if !myapp.filtered_songs.is_empty() {
                // Select the first song if none is selected
                let first_song = &myapp.filtered_songs[0];
                myapp.selected_song_id = Some(first_song.id);
                myapp.list_offset = 0;
            }
        }
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            if let Some(selected_id) = myapp.selected_song_id {
                // Find the index of the currently selected song by Uuid
                if let Some(index) = myapp
                    .filtered_songs
                    .iter()
                    .position(|song| song.id == selected_id)
                {
                    if index > 0 {
                        let previous_song = &myapp.filtered_songs[index - 1];
                        myapp.selected_song_id = Some(previous_song.id);

                        // Scroll up if selected index goes out of view
                        if index <= myapp.list_offset + 1 {
                            myapp.list_offset = myapp.list_offset.saturating_sub(1);
                        }
                    } else {
                        // Wrap around to the last song
                        let last_song = &myapp.filtered_songs[myapp.filtered_songs.len() - 1];
                        myapp.selected_song_id = Some(last_song.id);
                        myapp.list_offset = myapp
                            .filtered_songs
                            .len()
                            .saturating_sub(visible_song_count);
                    }
                }
            } else if !myapp.filtered_songs.is_empty() {
                // Select the last song if none is selected
                let last_song = &myapp.filtered_songs[myapp.filtered_songs.len() - 1];
                myapp.selected_song_id = Some(last_song.id);
                myapp.list_offset = myapp
                    .filtered_songs
                    .len()
                    .saturating_sub(visible_song_count);
            }
        }
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            // Move playlist selection down
            if myapp.selected_playlist_index < myapp.playlists.len() - 1 {
                myapp.selected_playlist_index = myapp.selected_playlist_index + 1;
                if myapp.selected_playlist_index
                    >= myapp.playlist_list_offset + visible_playlist_count - 1
                {
                    myapp.playlist_list_offset =
                        (myapp.selected_playlist_index - visible_playlist_count + 2).max(0);

                    // Ensure the playlist_list_offset does not exceed the maximum allowed offset
                    myapp.playlist_list_offset = myapp
                        .playlist_list_offset
                        .min(myapp.playlists.len().saturating_sub(visible_playlist_count));
                }
            } else {
                myapp.selected_playlist_index = 0;
                myapp.playlist_list_offset = 0;
            }
            myapp.selected_song_id = None;
        }
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        } => {
            // Move playlist selection up
            if myapp.selected_playlist_index > 0 {
                myapp.selected_playlist_index = myapp.selected_playlist_index - 1;

                // Scroll up if selected index goes out of view
                if myapp.selected_playlist_index <= myapp.playlist_list_offset + 1 {
                    myapp.playlist_list_offset = myapp.playlist_list_offset.saturating_sub(1);
                }
            } else {
                myapp.selected_playlist_index = myapp.playlists.len() - 1;
                myapp.playlist_list_offset =
                    myapp.playlists.len().saturating_sub(visible_playlist_count);
            }
            myapp.selected_song_id = None;
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
                    let current_position = sink.get_pos();
                    let new_position = current_position + Duration::from_secs(5);
                    sink.try_seek(new_position).unwrap();
                    myapp.song_time = Some(new_position);
                }
            }
            /*if let Some(current_id) = myapp.currently_playing_song {
                if let Some(current_song) = myapp.songs.iter().find(|song| song.id == current_id) {
                    let file = fs::File::open(&current_song.path).unwrap();
                    let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();

                    let elapsed_time = if let Some(paused_time) = myapp.paused_time {
                        myapp.song_time
                            .unwrap_or_else(Instant::now)
                            .elapsed()
                            .saturating_sub(paused_time.elapsed())
                    } else {
                        myapp.song_time
                            .unwrap_or_else(Instant::now)
                            .elapsed()
                    };
                    let new_time = elapsed_time.saturating_add(Duration::from_secs(5));
                    myapp.song_time = Some(Instant::now() - new_time);

                    let source = source.skip_duration(new_time);

                    let sink = sink.lock().unwrap();
                    sink.clear();
                    sink.append(source);
                    sink.play();
                }
            }*/
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
                    let current_position = sink.get_pos();
                    let new_position = current_position.saturating_sub(Duration::from_secs(5));
                    sink.try_seek(new_position).unwrap();
                    myapp.song_time = Some(new_position);
                }
            }
            /*if let Some(current_id) = myapp.currently_playing_song {
                if let Some(current_song) = myapp.songs.iter().find(|song| song.id == current_id) {
                    let file = fs::File::open(&current_song.path).unwrap();
                    let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();

                    let elapsed_time = if let Some(paused_time) = myapp.paused_time {
                        myapp.song_time
                            .unwrap_or_else(Duration::default)
                            .saturating_sub(paused_time)
                    } else {
                        myapp.song_time
                            .unwrap_or_else(Duration::default)
                    };
                    let new_time = elapsed_time.saturating_sub(Duration::from_secs(5));
                    myapp.song_time = Some(Duration::now() - new_time);

                    let source = source.skip_duration(new_time);

                    let sink = sink.lock().unwrap();
                    sink.clear();
                    sink.append(source);
                    sink.play();
                }
            }*/
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
            }
        }
        _ => {}
    }
}
