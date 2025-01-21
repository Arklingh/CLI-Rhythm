use crate::song::Song;
use crate::utils::sort_songs;
use dirs;
use rodio::{OutputStream, Sink};
use serde_json;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;
use crate::utils::{PopupState, SearchCriteria, SortCriteria, scan_folder_for_music};

/// The main application struct.
#[allow(dead_code)]
pub struct MyApp {
    pub songs: Box<Vec<Song>>, // List of all songs
    pub filtered_songs: Vec<Song>,
    pub sink: Arc<Mutex<Sink>>,
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
}

#[allow(dead_code)]
impl MyApp {
    // Initialize a new MyApp instance with default values
    pub fn new() -> MyApp {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        MyApp {
            songs: Box::new(Vec::new()),
            filtered_songs: Vec::new(),
            sink: Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap())),
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

    // Function to play a song
    pub fn play_song(&mut self) {
        if let Some(index) = self.selected_song_id {
            self.currently_playing_song = Some(index);
            let song = self.find_song_by_id(index).unwrap().clone();
            song.play(&self.sink);
            self.find_song_by_id(index).unwrap().is_playing = true;
            self.song_time = Some(Duration::default());
        }
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
        let serialized = serde_json::to_string(&self.playlists)?;

        if let Some(roaming_dir) = dirs::config_local_dir() {
            let myapp_dir: PathBuf = roaming_dir.join("cli-rhythm");
            std::fs::create_dir_all(&myapp_dir)?;

            let playlist_file_path = myapp_dir.join("data.json");

            let mut file = File::create(playlist_file_path)?;
            file.write_all(serialized.as_bytes())?;
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
        let file = File::open(filepath)?;
        let playlists: BTreeMap<String, Vec<Uuid>> = serde_json::from_reader(file)?;
        self.playlists = playlists;
        Ok(())
    }
}
