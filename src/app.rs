//! File: app.rs
//!
//! Description:
//! This file defines the `MyApp` struct, which serves as the core of a CLI-based
//! music player application. It manages song data, playback control using `rodio`,
//! playlist handling, search and sort functionality, and user interface state.
//! The application supports loading songs from a directory, filtering and sorting
//! them, playing selected tracks, and saving/loading playlists from disk.
//!
//! Key Features:
//! - Maintains a list of all and filtered songs
//! - Controls playback with support for pause/resume and seek
//! - Allows creation and management of playlists
//! - Handles search and sorting based on customizable criteria
//! - Saves and restores application state (playlists) via JSON
//!
//! Dependencies: rodio, serde_json, dirs, uuid, std libraries

use crate::song::Song;
use crate::utils::sort_songs;
use crate::utils::{scan_folder_for_music, PopupState, SearchCriteria, SortCriteria};
use dirs;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// The main application struct.
#[allow(dead_code)]
pub struct MyApp {
    pub songs: Box<Vec<Song>>, // List of all songs
    pub filtered_songs: Vec<Song>,
    pub selected_song_id: Option<Uuid>, // Index of the currently selected song
    pub currently_playing_song: Option<Uuid>, // Index of the currently playing song
    pub search_criteria: SearchCriteria, // Criteria to filter/search songs
    pub sort_criteria: SortCriteria,    // Criteria to sort songs
    pub hint_popup_state: PopupState,   // Controls the visibility of popups
    pub playlist_input_popup: PopupState,
    pub selected_playlist_index: usize,
    pub playlist_name_input: String, // Input buffer for the playlist name
    pub playlists: BTreeMap<String, Vec<Uuid>>, // Playlists with song indices
    pub search_text: String,
    pub previous_volume: f32,
    pub list_offset: usize,
    pub playlist_list_offset: usize,
    pub paused_time: Option<Duration>,
    pub chosen_song_ids: Vec<Uuid>,
    pub song_time: Option<Duration>,
    pub repeat_playlist: bool,
    pub repeat_song: bool,
}

#[allow(dead_code)]
impl MyApp {
    // Initialize a new MyApp instance with default values
    pub fn new() -> MyApp {
        MyApp {
            songs: Box::new(Vec::new()),
            filtered_songs: Vec::new(),
            selected_song_id: None,
            currently_playing_song: None,
            search_criteria: SearchCriteria::Title,
            sort_criteria: SortCriteria::Title,
            selected_playlist_index: 0,
            hint_popup_state: PopupState { visible: false },
            playlist_input_popup: PopupState { visible: false },
            playlist_name_input: String::new(),
            playlists: BTreeMap::new(),
            search_text: String::new(),
            previous_volume: 0.0,
            list_offset: 0,
            playlist_list_offset: 0,
            paused_time: None,
            chosen_song_ids: vec![],
            song_time: None,
            repeat_playlist: false,
            repeat_song: false,
        }
    }

    // Function to load songs into the app
    pub fn load_songs(&mut self) {
        self.songs = Box::new(scan_folder_for_music());
        let ids: Vec<Uuid> = self.songs.iter().map(|song| song.id).collect();
        self.playlists.insert("All Songs".to_string(), ids);
        self.sort_songs(); // Sort based on current criteria after loading
    }

    // Function to handle song selection
    pub fn select_song(&mut self, index: Uuid) {
        self.selected_song_id = Some(index);
    }

    pub fn find_song_by_id(&mut self, id: Uuid) -> Option<&mut Song> {
        self.songs.iter_mut().find(|song| song.id == id)
    }

    // Function to stop the current song
    pub fn stop_song(&mut self) {
        if let Some(index) = self.currently_playing_song {
            self.songs[index.as_u128() as usize].is_playing = false;
            self.currently_playing_song = None;
        }
    }

    // Function to toggle popup visibility
    pub fn toggle_popup(&mut self) {
        self.hint_popup_state.toggle();
    }

    // Function to change sorting criteria
    pub fn set_sort_criteria(&mut self, criteria: SortCriteria) {
        self.sort_criteria = criteria;
        self.sort_songs(); // Re-sort the songs based on new criteria
    }

    // Sort the list of songs based on the current sort criteria
    pub fn sort_songs(&mut self) {
        sort_songs(&mut self.songs, &self.sort_criteria);
    }

    /// Saves the current playlists to a file.
    ///
    /// # Returns
    /// A `Result` indicating success or failure.
    pub fn save_playlist(&self) -> std::io::Result<()> {
        if let Some(roaming_dir) = dirs::config_local_dir() {
            let myapp_dir: PathBuf = roaming_dir.join("cli-rhythm");
            std::fs::create_dir_all(&myapp_dir)?;

            for (playlist_name, song_uuids) in &self.playlists {
                let playlist_file_path = myapp_dir.join(format!("{playlist_name}.m3u"));
                let mut file = File::create(playlist_file_path)?;

                writeln!(file, "#EXTM3U")?;

                for uuid in song_uuids {
                    if let Some(song) = self.songs.iter().find(|song| song.id == *uuid) {
                        writeln!(file, "{}", song.path.display())?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Loads playlists from a file.
    ///
    /// # Arguments
    /// * `filepath` - The path to the file containing the playlists.
    ///
    /// # Returns
    /// A `Result` indicating success or failure.
    pub fn load_playlists(&mut self, filepath: &str) -> std::io::Result<()> {
        let mut loaded_playlists: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();

        // Read all .m3u files
        for entry in std::fs::read_dir(&filepath)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("m3u") {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let mut song_uuids = Vec::new();

                for line in reader.lines() {
                    let line = line?;
                    if line.starts_with("#") || line.trim().is_empty() {
                        continue; // skip comments or empty lines
                    }

                    let abs_path = PathBuf::from(line);
                    let uuid =
                        Uuid::new_v5(&Uuid::NAMESPACE_DNS, abs_path.to_str().unwrap().as_bytes());
                    song_uuids.push(uuid);
                }

                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    loaded_playlists.insert(filename.to_string(), song_uuids);
                }
            }
        }

        self.playlists = loaded_playlists;
        Ok(())
    }
}
