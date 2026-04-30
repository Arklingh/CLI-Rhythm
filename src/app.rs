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
use rodio::Sink;
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;

/// The main application struct.
pub struct MyApp {
    pub songs: Vec<Song>, // List of all songs (kept for iteration/sorting that needs Vec)
    pub songs_by_id: BTreeMap<Uuid, Song>, // Map for efficient song lookups by ID
    pub filtered_songs: Vec<Song>,
    pub selected_song_id: Option<Uuid>, // ID of the currently selected song
    pub currently_playing_song: Option<Uuid>, // ID of the currently playing song
    pub search_criteria: SearchCriteria, // Criteria to filter/search songs
    pub sort_criteria: SortCriteria,    // Criteria to sort songs
    pub hint_popup_state: PopupState,   // Controls the visibility of popups
    pub playlist_input_popup: PopupState,
    pub selected_playlist_index: usize,
    pub playlist_name_input: String, // Input buffer for the playlist name
    pub playlists: BTreeMap<String, Vec<Uuid>>, // Playlists with song IDs
    pub search_text: String,
    pub previous_volume: f32,
    pub list_offset: usize,

    pub paused_time: Option<Duration>,
    pub chosen_song_ids: Vec<Uuid>,
    pub song_time: Option<Duration>,
    pub repeat_song: bool,
}

impl MyApp {
    /// Creates a new `MyApp` instance with default values.
    ///
    /// # Returns
    /// A new `MyApp` instance with empty song lists and default settings.
    pub fn new() -> MyApp {
        Self::default()
    }

    /// Loads songs from the music directory into the application.
    ///
    /// Scans the user's music folder (or current directory as fallback) for
    /// supported audio files (MP3, WAV, FLAC, AAC) and populates the song
    /// database with metadata including title, artist, album, duration, and
    /// cover art (stored as raw bytes for memory efficiency).
    pub fn load_songs(&mut self) {
        let loaded_songs = scan_folder_for_music();
        self.songs_by_id = loaded_songs
            .into_iter()
            .map(|song| (song.id, song))
            .collect();
        self.songs = self.songs_by_id.values().cloned().collect(); // Keep `songs` for sorting/iteration if needed by other parts of the app

        let ids: Vec<Uuid> = self.songs_by_id.keys().cloned().collect();
        self.playlists.insert("All Songs".to_string(), ids);
        sort_songs(&mut self.songs, &self.sort_criteria); // Sort the `songs` vector
    }

    /// Retrieves a mutable reference to a song by its UUID.
    ///
    /// Uses the internal `songs_by_id` map for O(log n) lookup efficiency.
    ///
    /// # Arguments
    /// * `id` - The UUID of the song to find
    ///
    /// # Returns
    /// `Some(&mut Song)` if found, `None` otherwise
    pub fn find_song_by_id(&mut self, id: Uuid) -> Option<&mut Song> {
        self.songs_by_id.get_mut(&id)
    }

    /// Stops the currently playing song and updates its state.
    ///
    /// Sets `is_playing` to false for the currently playing song and
    /// clears the `currently_playing_song` field.
    pub fn stop_song(&mut self) {
        if let Some(song_id) = self.currently_playing_song {
            if let Some(song) = self.songs_by_id.get_mut(&song_id) {
                song.is_playing = false;
            }
            self.currently_playing_song = None;
        }
    }

    /// Changes the song sorting criteria and re-sorts the song list.
    ///
    /// # Arguments
    /// * `criteria` - The new sorting criteria (Title, Artist, Duration, or Shuffle)
    pub fn set_sort_criteria(&mut self, criteria: SortCriteria) {
        self.sort_criteria = criteria;
        sort_songs(&mut self.songs, &self.sort_criteria); // Re-sort the songs based on new criteria
    }

    /// Updates the filtered song list based on current search and playlist selection.
    ///
    /// Applies the following filters in order:
    /// 1. Selected playlist (if any playlist is selected)
    /// 2. Search text matching the current criteria (Title, Artist, or Album)
    ///
    /// The filtered results are stored in `filtered_songs` for display.
    pub fn update_filtered_songs(&mut self) {
        let playlist_song_ids: HashSet<Uuid> = self
            .playlists
            .values()
            .nth(self.selected_playlist_index)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let search_text_lower = self.search_text.to_lowercase();

        self.filtered_songs = self
            .songs
            .iter()
            .filter(|s| match self.search_criteria {
                SearchCriteria::Title => s.title.to_lowercase().contains(&search_text_lower),
                SearchCriteria::Artist => s.artist.to_lowercase().contains(&search_text_lower),
                SearchCriteria::Album => s.album.to_lowercase().contains(&search_text_lower),
            })
            .filter(|song| playlist_song_ids.contains(&song.id))
            .cloned()
            .collect();
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
                    if let Some(song) = self.songs_by_id.get(uuid) {
                        writeln!(file, "{}", song.path.display())?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Loads playlists from a directory.
    ///
    /// # Arguments
    /// * `directory_path` - The path to the directory containing the playlists.
    ///
    /// # Returns
    /// A `Result` indicating success or failure.
    pub fn load_playlists(&mut self, directory_path: &str) -> std::io::Result<()> {
        let mut loaded_playlists: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();

        // Read all .m3u files
        for entry in std::fs::read_dir(&directory_path)? {
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
                    // Ensure the path can be converted to a str before hashing
                    if let Some(path_str) = abs_path.to_str() {
                        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, path_str.as_bytes());
                        song_uuids.push(uuid);
                    } else {
                        eprintln!(
                            "Warning: Could not convert path to string for UUID generation: {:?}",
                            abs_path
                        );
                    }
                }

                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    loaded_playlists.insert(filename.to_string(), song_uuids);
                }
            }
        }

        self.playlists.extend(loaded_playlists);
        Ok(())
    }

    /// Processes audio playback state on each application tick.
    ///
    /// Checks if the current song has finished playing and auto-advances
    /// to the next song in the filtered list if repeat is not enabled.
    /// Handles song transitions safely with error handling.
    ///
    /// # Arguments
    /// * `sink` - The audio sink for checking playback position and controlling playback
    ///
    /// # Note
    /// Handles poisoned mutex locks gracefully when accessing the audio sink.
    pub fn tick(&mut self, sink: &Arc<Mutex<Sink>>) {
        if let Some(current_song_id) = self.currently_playing_song {
            // Find the song using songs_by_id for efficiency
            let song_clone = match self.songs_by_id.get(&current_song_id).cloned() {
                Some(s) => s,
                None => return, // Song not found, can't proceed
            };

            let current_time = self.song_time.unwrap_or_default().as_secs_f64();

            // Check if song is finished
            if song_clone.is_playing
                && (song_clone.duration - current_time < 0.1 || song_clone.duration < current_time)
            {
                if self.repeat_song {
                    // If repeat is on, replay the current song
                    self.play_file_safely(&song_clone.path, sink);
                } else {
                    // Find the next song in the filtered list
                    let current_song_index = self
                        .filtered_songs
                        .iter()
                        .position(|s| s.id == current_song_id)
                        .unwrap_or(0); // Default to 0 if not found, though it should be

                    let next_song_id_option = self
                        .filtered_songs
                        .get((current_song_index + 1) % self.filtered_songs.len())
                        .map(|s| s.id);

                    if let Some(next_song_id) = next_song_id_option {
                        // --- IMPORTANT: Extract path BEFORE any mutable borrows of self ---
                        // Retrieve the path to the next song and clone it. This drops the
                        // immutable borrow of `self` from `self.songs_by_id.get()` immediately.
                        let next_song_path: Option<PathBuf> =
                            self.songs_by_id.get(&next_song_id).map(|s| s.path.clone());

                        // Update song state using mutable borrows
                        if let Some(next_song) = self.find_song_by_id(next_song_id) {
                            next_song.is_playing = true;
                        }
                        // Update global playback state
                        self.currently_playing_song = Some(next_song_id);
                        self.selected_song_id = Some(next_song_id); // Also select it

                        // Now call play_file_safely with the cloned path
                        if let Some(path) = next_song_path {
                            self.play_file_safely(&path, sink);
                        } else {
                            // If song data is unexpectedly missing, stop playback
                            eprintln!("Error: Song data not found for ID: {}", next_song_id);
                            self.stop_song();
                            self.song_time = None;
                            self.paused_time = None;
                        }
                    } else {
                        // No more songs or list is empty, stop playback
                        self.stop_song(); // This will set currently_playing_song to None
                        self.song_time = None;
                        self.paused_time = None;
                    }
                }
            }
        } else {
            // If no song is currently playing, reset related states
            self.song_time = None;
            self.paused_time = None;
        }
    }

    /// Safely plays an audio file with comprehensive error handling.
    ///
    /// Handles file opening, audio decoding, and playback setup. Logs
    /// errors without panicking on failure (file not found, decode errors,
    /// poisoned mutex locks, etc.).
    ///
    /// # Arguments
    /// * `path` - Path to the audio file to play
    /// * `sink` - The audio sink to append the decoded source to
    ///
    /// # Note
    /// - Resets `paused_time` when starting playback
    /// - Sets `song_time` to zero (start of track)
    /// - Handles poisoned mutex locks by recovering with `into_inner()`
    fn play_file_safely(&mut self, path: &Path, sink: &Arc<Mutex<Sink>>) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error: Could not open audio file: {} - {}", path.display(), e);
                return;
            }
        };
        
        let reader = BufReader::new(file);
        let source = match rodio::Decoder::new(reader) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: Could not decode audio file: {} - {}", path.display(), e);
                return;
            }
        };
        
        let sink_guard = match sink.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Warning: Audio sink lock was poisoned, recovering...");
                poisoned.into_inner()
            }
        };
        
        self.paused_time = None; // Reset pause time when starting a new file
        sink_guard.clear(); // Stop current playback
        sink_guard.append(source);
        sink_guard.play();
        // `sink_guard` is dropped here, releasing the lock.

        // Reset song time to start when a new file is played
        self.song_time = Some(Duration::from_secs(0));
    }
}

impl Default for MyApp {
    /// Creates a default `MyApp` instance with empty collections and default settings.
    ///
    /// # Default Values
    /// - Empty song collections (`songs`, `songs_by_id`, `filtered_songs`)
    /// - Search by Title, Sort by Title
    /// - Hidden popups, no selected song/playlist
    /// - Volume memory at 1.0 (100%)
    /// - Repeat disabled
    fn default() -> Self {
        Self {
            songs: Vec::new(),
            songs_by_id: BTreeMap::new(),
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
            paused_time: None,
            chosen_song_ids: vec![],
            song_time: None,
            repeat_song: false,
        }
    }
}
