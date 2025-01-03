//! # CLI-Rhythm
//!
//! CLI-Rhythm is a terminal-based music player written in Rust, designed for a minimalistic and efficient command-line interface. It allows users to manage and play their music collection directly from the terminal, offering features such as sorting, searching, and playback controls. Built with a focus on simplicity and performance, CLI-Rhythm provides an intuitive experience for music enthusiasts who prefer a text-based environment.
//!
//! ## Features
//! - Play music files from supported formats.
//! - Sort and search for songs by title, artist, or album.
//! - Navigate through a list of songs with ease.
//! - Minimal resource usage with a clean terminal interface.

extern crate crossterm;
extern crate ratatui;

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{stdout, Write};
use std::ops::Sub;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, io};

use crossterm::event::{poll, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear};
use crossterm::ExecutableCommand;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::{symbols, Frame};
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use rodio::{OutputStream, Sink, Source};

use audiotags::{types::Album, Tag};
use dirs;
use mp3_metadata::read_from_file;
use textwrap::wrap;
use uuid::Uuid;
use image::{self, load_from_memory_with_format, DynamicImage, ImageBuffer, ImageFormat, Rgba};

/// Supported music file formats.
const MUSIC_FORMATS: [&str; 4] = ["mp3", "wav", "flac", "aac"];

/// Represents a song with metadata.
#[derive(Clone)]
struct Song {
    id: Uuid,
    /// Title of the song.
    title: String,
    /// Artist of the song.
    artist: String,
    /// Cover art of the song/album.
    cover: Option<DynamicImage>,
    /// File path to the song.
    path: PathBuf,
    /// Album name of the song.
    album: String,
    /// Duration of the song in seconds.
    duration: f64,
    /// Indicates if the song is currently playing.
    is_playing: bool,
}

impl Song {
    /// Creates a new `Song` instance.
    ///
    /// # Arguments
    /// * `title` - The title of the song.
    /// * `artist` - The artist of the song.
    /// * `path` - The file path to the song.
    /// * `album` - The album name of the song.
    /// * `duration` - The duration of the song in seconds.
    fn new(title: String, artist: String, cover: Option<DynamicImage>, path: PathBuf, album: String, duration: f64) -> Self {
        Song {
            id: Uuid::new_v5(&Uuid::NAMESPACE_DNS, path.to_str().unwrap().as_bytes()),
            title,
            artist,
            cover,
            path,
            album,
            duration,
            is_playing: false,
        }
    }

    /// Plays the song using the provided `Sink`.
    ///
    /// # Arguments
    /// * `sink` - The `Sink` to play the song through.
    fn play(&self, sink: &Arc<Mutex<Sink>>) {
        let file = fs::File::open(&self.path).unwrap();
        let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();
        sink.lock().unwrap().append(source);
        sink.lock().unwrap().play();
    }
}

#[derive(Debug)]
enum Tabs {
    Songs,
    Settings,
}

impl Tabs {
    fn next(&self) -> Tabs {
        match self {
            Tabs::Songs => Tabs::Settings,
            Tabs::Settings => Tabs::Songs,
        }
    }
}

impl ToString for Tabs {
    fn to_string(&self) -> String {
        match self {
            Tabs::Songs => "Songs".to_string(),
            Tabs::Settings => "Settings".to_string(),
        }
    }
}

/// Enum representing the criteria for searching songs.
enum SearchCriteria {
    Title,
    Artist,
    Album,
}

/// Enum representing the criteria for sorting songs.
#[derive(PartialEq, Eq, Debug)]
enum SortCriteria {
    Title,
    Artist,
    Duration,
}

impl SortCriteria {
    /// Returns the next sorting criteria in the sequence.
    fn next(&self) -> SortCriteria {
        match self {
            SortCriteria::Title => SortCriteria::Artist,
            SortCriteria::Artist => SortCriteria::Duration,
            SortCriteria::Duration => SortCriteria::Title,
        }
    }
}

impl ToString for SortCriteria {
    /// Converts the sorting criteria to a string representation.
    fn to_string(&self) -> String {
        match self {
            SortCriteria::Title => "Title".to_string(),
            SortCriteria::Artist => "Artist".to_string(),
            SortCriteria::Duration => "Duration".to_string(),
        }
    }
}

struct PopupState {
    visible: bool,
}

impl PopupState {
    fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

/// The main application struct.
pub struct MyApp {
    songs: Box<Vec<Song>>, // List of all songs
    filtered_songs: Vec<Song>,
    sink: Arc<Mutex<Sink>>,
    selected_song_id: Option<Uuid>, // Index of the currently selected song
    currently_playing_song: Option<Uuid>, // Index of the currently playing song
    search_criteria: SearchCriteria, // Criteria to filter/search songs
    sort_criteria: SortCriteria,    // Criteria to sort songs
    hint_popup_state: PopupState,   // Controls the visibility of popups
    playlist_input_popup: PopupState,
    selected_playlist_index: usize,
    playlist_name_input: String, // Input buffer for the playlist name
    playlists: BTreeMap<String, Vec<Uuid>>, // Playlists with song indices
    search_text: String,
    previous_volume: f32,
    list_offset: usize,
    playlist_list_offset: usize,
    paused_time: Option<Instant>,
    chosen_song_ids: Vec<Uuid>,
    song_time: Option<Instant>,
    current_tab: Tabs,
}

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
            current_tab: Tabs::Songs,
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

    fn find_song_by_id(&mut self, id: Uuid) -> Option<&mut Song> {
        self.songs.iter_mut().find(|song| song.id == id)
    }

    // Function to play a song
    pub fn play_song(&mut self) {
        if let Some(index) = self.selected_song_id {
            self.currently_playing_song = Some(index);
            let song = self.find_song_by_id(index).unwrap().clone();
            song.play(&self.sink);
            self.find_song_by_id(index).unwrap().is_playing = true;
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
    fn set_sort_criteria(&mut self, criteria: SortCriteria) {
        self.sort_criteria = criteria;
        self.sort_songs(); // Re-sort the songs based on new criteria
    }

    // Sort the list of songs based on the current sort criteria
    fn sort_songs(&mut self) {
        sort_songs(&mut self.songs, &self.sort_criteria);
    }

    /// Saves the current playlists to a file.
    ///
    /// # Returns
    /// A `Result` indicating success or failure.
    fn save_playlist(&self) -> std::io::Result<()> {
        let serialized = serde_json::to_string(&self.playlists)?;

        if let Some(roaming_dir) = dirs::config_local_dir() {
            let myapp_dir: PathBuf = roaming_dir.join("cli-rhythm");
            fs::create_dir_all(&myapp_dir)?;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    let mut terminal = ratatui::init();
    let picker = Picker::from_fontsize((7, 14));

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    let mut myapp = MyApp::new();
    match myapp.load_playlists(
        dirs::config_local_dir()
            .unwrap()
            .join("cli-rhythm")
            .join("data.json")
            .to_str()
            .unwrap(),
    ) {
        Ok(_) => {}
        Err(_) => {}
    }
    myapp.load_songs();

    let mut visible_song_count: usize = 0;
    let mut visible_playlist_count: usize = 0;

    sort_songs(&mut myapp.songs, &myapp.sort_criteria);

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap()));

    // Run event loop
    loop {
        let search_bar_title = match myapp.search_criteria {
            SearchCriteria::Title => "Search by Title",
            SearchCriteria::Artist => "Search by Artist",
            SearchCriteria::Album => "Search by Album",
        };

        // Render search bar
        let search_bar = Paragraph::new(Text::raw(format!("{}", myapp.search_text)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(search_bar_title),
            )
            .style(Style::default().fg(Color::White));

        let playlist_name = match myapp.playlists.keys().nth(myapp.selected_playlist_index) {
            Some(name) => name,
            None => &String::new(),
        };

        let playlist_songs = match myapp.playlists.get(playlist_name) {
            Some(songs) => songs,
            None => &vec![],
        };

        // Filter songs based on search text
        myapp.filtered_songs = myapp
            .songs
            .iter()
            .filter(|s| match myapp.search_criteria {
                SearchCriteria::Title => s
                    .title
                    .to_lowercase()
                    .contains(&myapp.search_text.to_lowercase()),
                SearchCriteria::Artist => s
                    .artist
                    .to_lowercase()
                    .contains(&myapp.search_text.to_lowercase()),
                SearchCriteria::Album => s
                    .album
                    .to_lowercase()
                    .contains(&myapp.search_text.to_lowercase()),
            })
            .filter(|song| playlist_songs.contains(&song.id))
            .cloned()
            .collect();

        if let Some(selected_id) = myapp.selected_song_id {
            if !myapp
                .filtered_songs
                .iter()
                .any(|song| song.id == selected_id)
            {
                if let Some(first_song) = myapp.filtered_songs.first() {
                    myapp.selected_song_id = Some(first_song.id);
                    myapp.list_offset = 0;
                } else {
                    myapp.selected_song_id = None;
                }
            }
        } else if !myapp.filtered_songs.is_empty() {
            if let Some(first_song) = myapp.filtered_songs.first() {
                myapp.selected_song_id = Some(first_song.id);
                myapp.list_offset = 0;
            }
        };

        let selected_song = match myapp.selected_song_id {
            Some(index) => myapp.find_song_by_id(index),
            None => None,
        };

        let selected_song_details = if let Some(song) = selected_song {
            let contents = format!(
                "Artist: {}\nSong: {}\nAlbum: {}\nDuration: {:02}:{:02}",
                song.artist,
                song.title,
                song.album,
                (song.duration / 60.0).floor(),
                (song.duration % 60.0).round()
            );
            let wrapped_details = wrap(&contents, 29);

            wrapped_details.join("\n")
        } else {
            "No song selected".to_string()
        };

        let playing_song_details = if let Some(song_id) = myapp.currently_playing_song {
            let song = myapp.find_song_by_id(song_id).unwrap();
            let contents = format!(
                "Artist: {}\nSong: {}\nAlbum: {}\nDuration: {:02}:{:02}",
                song.artist,
                song.title,
                song.album,
                (song.duration / 60.0).floor(),
                (song.duration % 60.0).round()
            );
            let wrapped_details = wrap(&contents, 29);

            wrapped_details.join("\n")
        } else {
            "No song playing".to_string()
        };

        let selected_song_info = Paragraph::new(selected_song_details)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Selected Song"),
            )
            .style(Style::default().fg(Color::White));

        let playing_song_info = Paragraph::new(playing_song_details)
            .block(Block::default()).style(Style::default().fg(Color::White));
        
        let playing_song_cover = if let Some(song_id) = myapp.currently_playing_song {
            myapp.find_song_by_id(song_id)
                .and_then(|song| song.cover.clone())
                .unwrap_or_else(|| {
                    let img = ImageBuffer::from_fn(4, 4, |_, _| Rgba([0, 0, 0, 0]));
                    DynamicImage::ImageRgba8(img)
                })
        } else {
                let img = ImageBuffer::from_fn(4, 4, |_, _| Rgba([0, 0, 0, 0]));
                DynamicImage::ImageRgba8(img)
        };
        let mut pic = picker.new_resize_protocol(playing_song_cover);
        let img = StatefulImage::default();
        
        // Check if a song is playing
        if let Some(current_song_id) = myapp.currently_playing_song {
            if let Some(song) = myapp.find_song_by_id(current_song_id).cloned() {
                if song.is_playing {
                    // Update song time
                    myapp.song_time = Some(
                        myapp.song_time.unwrap_or(Instant::now()) + Duration::from_secs_f64(0.1),
                    );

                    // If the song is finished, play the next one
                    if myapp.song_time.unwrap().elapsed().as_secs_f64() >= song.duration {
                        if let Some(current_song) = myapp.find_song_by_id(current_song_id) {
                            current_song.is_playing = false;
                        }

                        let next_index = myapp
                            .filtered_songs
                            .iter()
                            .position(|s| s.id == current_song_id)
                            .map(|idx| (idx + 1) % myapp.filtered_songs.len())
                            .unwrap_or(0);

                        // Play the next song
                        let next_song = myapp
                            .find_song_by_id(myapp.filtered_songs[next_index].id)
                            .cloned();

                        if let Some(song) = next_song {
                            let file = fs::File::open(&song.path).unwrap();
                            let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();
                            myapp.song_time = Some(Instant::now());
                            myapp.currently_playing_song =
                                Some(myapp.filtered_songs[next_index].id);
                            myapp.selected_song_id = Some(myapp.filtered_songs[next_index].id);
                            myapp.paused_time = None;
                            myapp.filtered_songs[next_index].is_playing = true; // !!!!!BIG PROBLEMO!!!!
                            sink.lock().unwrap().clear();
                            sink.lock().unwrap().append(source);
                            sink.lock().unwrap().play();
                        }
                    }
                }
            }
        }

        let song_id = myapp
            .currently_playing_song
            .or(myapp.selected_song_id)
            .unwrap_or_else(|| myapp.songs.first().map(|song| song.id).unwrap_or_default());

        let progress_ratio = match myapp.find_song_by_id(song_id).cloned() {
            Some(song) if song.duration > 0.0 && !sink.lock().unwrap().is_paused() => {
                if let Some(song_time) = myapp.song_time {
                    let elapsed_time = song_time.elapsed().as_secs_f64().min(song.duration);
                    if elapsed_time >= song.duration {
                        // If the song is over, set progress to 0
                        0.0
                    } else {
                        elapsed_time / song.duration
                    }
                } else {
                    0.0
                }
            }
            Some(song) if song.duration > 0.0 && sink.lock().unwrap().is_paused() => {
                let mut ratio: f64 = 0.0;
                if let Some(song_time) = myapp.song_time {
                    if let Some(paused_time) = myapp.paused_time {
                        let elapsed_time = song_time.elapsed().as_secs_f64().min(song.duration);
                        ratio =
                            (elapsed_time - paused_time.elapsed().as_secs_f64()).max(0.0) / song.duration;
                    }
                }
                ratio
            }
            _ => 0.0,
        };

        let song_progress = if let Some(song) = myapp.find_song_by_id(song_id).cloned() {
            let elapsed_time = if let Some(paused_time) = myapp.paused_time {
                myapp
                    .song_time
                    .unwrap_or(Instant::now())
                    .elapsed()
                    .as_secs_f64()
                    .sub(paused_time.elapsed().as_secs_f64())
                    .min(song.duration)
            } else {
                myapp
                    .song_time
                    .unwrap_or(Instant::now())
                    .elapsed()
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

        // Volume bar
        let volume_bar = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Volume"))
            .gauge_style(Style::default().fg(Color::LightBlue))
            .label(format!("{:.0}%", sink.lock().unwrap().volume() * 100.0))
            .ratio(sink.lock().unwrap().volume() as f64);

        let hint = Paragraph::new("F1 for controls")
            .style(
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )
            .alignment(Alignment::Right);

        terminal.draw(|f| {
            let vertical_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Fill(1),
                ])
                .split(f.area());

            let tabs = ratatui::widgets::Tabs::new(vec![
                Tabs::Songs.to_string(),
                Tabs::Settings.to_string(),
            ])
            .block(Block::bordered().title("Tabs"))
            .style(Style::default().white())
            .highlight_style(Style::default().red())
            .divider(symbols::DOT)
            .padding(" ", " ")
            .select(match myapp.current_tab {
                Tabs::Songs => 0,
                Tabs::Settings => 1,
            });
            f.render_widget(tabs, vertical_layout[0]);

            match myapp.current_tab {
                Tabs::Songs => {
                    let song_tab_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(7),
                            Constraint::Percentage(86),
                            Constraint::Percentage(7),
                        ])
                        .split(vertical_layout[1]);
                    f.render_widget(search_bar, song_tab_layout[0]);

                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(20),
                            Constraint::Percentage(60),
                            Constraint::Percentage(20),
                        ])
                        .split(song_tab_layout[1]);

                    visible_playlist_count = (chunks[0].height - 2) as usize;
                    visible_song_count = (chunks[1].height - 2) as usize;

                    let song_items: Vec<ListItem> = myapp
                        .filtered_songs
                        .iter()
                        .enumerate()
                        .skip(myapp.list_offset)
                        .take(visible_song_count as usize)
                        .map(|(index, song)| {
                            let mut style = Style::default();
                            if myapp.chosen_song_ids.contains(&myapp.songs[index].id) {
                                style = Style::default()
                                    .fg(Color::LightRed)
                                    .add_modifier(Modifier::RAPID_BLINK);
                            }
                            if let Some(selected_id) = myapp.selected_song_id {
                                if selected_id == song.id {
                                    style = Style::default()
                                        .fg(Color::LightBlue)
                                        .add_modifier(Modifier::BOLD);
                                }
                            }
                            ListItem::new(song.title.clone()).style(style)
                        })
                        .collect();

                    let song_list = List::new(song_items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("Songs----------------------------------------------------------------------Sort by: {}", 
                                    myapp.sort_criteria.to_string(),))
                        )
                        .highlight_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        );

                    let playlist_items: Vec<ListItem> = myapp
                        .playlists
                        .iter()
                        .enumerate()
                        .map(|(index, (playlist_name, _songs))| {
                            let mut style = Style::default();
                            if myapp.selected_playlist_index == index {
                                style = Style::default()
                                    .fg(Color::LightBlue)
                                    .add_modifier(Modifier::BOLD);
                            }
                            ListItem::new(playlist_name.clone()).style(style)
                        })
                        .collect();

                    let playlist_list = List::new(playlist_items)
                        .block(Block::default().borders(Borders::ALL).title("Playlists"))
                        .highlight_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        );

                    f.render_widget(playlist_list, chunks[0]);

                    f.render_widget(song_list, chunks[1]);

                    let songs_info = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .split(chunks[2]);

                    f.render_widget(selected_song_info, songs_info[0]);

                    let playing_song_block = Block::default()
                        .borders(Borders::ALL)
                        .title("Currently playing");
                    let inner_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(40), Constraint::Fill(1)])
                        .split(playing_song_block.inner(songs_info[1]));

                    f.render_widget(playing_song_info, inner_layout[0]);
                    f.render_stateful_widget(img, inner_layout[1], &mut pic);
                    f.render_widget(playing_song_block, songs_info[1]);
                    
                    let footer = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                        .split(song_tab_layout[2]);

                    f.render_widget(song_progress, footer[0]);

                    f.render_widget(volume_bar, footer[1]);

                    if myapp.hint_popup_state.visible {
                        let _ = draw_popup(f);
                    }

                    if myapp.playlist_input_popup.visible {
                        let _ = draw_playlist_name_input_popup(f, &myapp.playlist_name_input);
                    }
                }
                Tabs::Settings => {}
            }
            f.render_widget(
                hint,
                Rect::new(
                    f.area().width.saturating_sub(20 as u16),
                    f.area().height - 1,
                    20 as u16,
                    1,
                ),
            );
        })?;

        // Handle input events
        if poll(Duration::from_millis(200))? {
            if let Event::Key(key) = crossterm::event::read()? {
                match key {
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        let _ = myapp.save_playlist();
                        break;
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
                                            myapp.list_offset =
                                                (new_index - visible_song_count + 2).max(0);

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
                                    let last_song =
                                        &myapp.filtered_songs[myapp.filtered_songs.len() - 1];
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
                                    (myapp.selected_playlist_index - visible_playlist_count + 2)
                                        .max(0);

                                // Ensure the playlist_list_offset does not exceed the maximum allowed offset
                                myapp.playlist_list_offset = myapp.playlist_list_offset.min(
                                    myapp.playlists.len().saturating_sub(visible_playlist_count),
                                );
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
                                myapp.playlist_list_offset =
                                    myapp.playlist_list_offset.saturating_sub(1);
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
                                    myapp.song_time = Some(Instant::now());
                                    myapp.currently_playing_song = Some(selected_id);

                                    // Set is_playing field to true
                                    if let Some(song) =
                                        myapp.songs.iter_mut().find(|s| s.id == selected_id)
                                    {
                                        song.is_playing = true;
                                    }
                                } else {
                                    // Stop the currently playing song
                                    sink.lock().unwrap().clear();
                                    myapp.song_time = None;
                                    myapp.currently_playing_song = None;

                                    // Set is_playing field to false
                                    if let Some(song) =
                                        myapp.songs.iter_mut().find(|s| s.id == selected_id)
                                    {
                                        song.is_playing = false;
                                    }

                                    // Update song_time if the song was playing
                                    if let Some(start_time) = myapp.song_time {
                                        let elapsed_time = start_time.elapsed().as_secs_f64().min(
                                            myapp
                                                .songs
                                                .iter()
                                                .find(|s| s.id == selected_id)
                                                .map_or(0.0, |s| s.duration),
                                        );
                                        let adjusted_time =
                                            if let Some(paused_at) = myapp.paused_time {
                                                elapsed_time + paused_at.elapsed().as_secs_f64()
                                            } else {
                                                elapsed_time
                                            };
                                        myapp.song_time = Some(
                                            start_time + Duration::from_secs_f64(adjusted_time),
                                        );
                                    } else {
                                        myapp.song_time = Some(Instant::now());
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
                                if let Some(song) =
                                    myapp.songs.iter_mut().find(|s| s.id == current_id)
                                {
                                    sink.lock().unwrap().play();
                                    song.is_playing = true;
                                }
                                // Calculate elapsed time during the pause
                                if let Some(paused_at) = myapp.paused_time {
                                    let elapsed_during_pause = paused_at.elapsed();
                                    myapp.song_time =
                                        myapp.song_time.map(|t| t + elapsed_during_pause);
                                    myapp.paused_time = None;
                                }
                            }
                        } else {
                            if let Some(current_id) = myapp.currently_playing_song {
                                if let Some(song) =
                                    myapp.songs.iter_mut().find(|s| s.id == current_id)
                                {
                                    sink.lock().unwrap().pause();
                                    song.is_playing = false;
                                    // Record the time when playback was paused
                                    myapp.paused_time = Some(Instant::now());
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
                                        myapp.song_time = Some(Instant::now());
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
                                        myapp.song_time = Some(Instant::now());
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
                            if let Some(current_song) =
                                myapp.songs.iter().find(|song| song.id == current_id)
                            {
                                let file = fs::File::open(&current_song.path).unwrap();
                                let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();

                                let time = myapp
                                    .song_time
                                    .unwrap_or_else(Instant::now)
                                    .elapsed()
                                    .saturating_add(Duration::from_secs(5));
                                myapp.song_time = Some(Instant::now() - time);

                                let source = source.skip_duration(time);

                                let sink = sink.lock().unwrap();
                                sink.clear();
                                sink.append(source);
                                sink.play();
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
                            if let Some(current_song) =
                                myapp.songs.iter().find(|song| song.id == current_id)
                            {
                                let file = fs::File::open(&current_song.path).unwrap();
                                let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();

                                let time = myapp
                                    .song_time
                                    .unwrap_or_else(Instant::now)
                                    .elapsed()
                                    .saturating_sub(Duration::from_secs(5));
                                myapp.song_time = Some(Instant::now() - time);

                                let source = source.skip_duration(time);

                                let sink = sink.lock().unwrap();
                                sink.clear();
                                sink.append(source);
                                sink.play();
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
                                myapp.playlist_name_input =
                                    "Need a name and at least 1 song".to_string()
                            }
                            (true, false) => myapp.playlist_name_input = "Need a name ".to_string(),
                            (false, true) => {
                                myapp.playlist_name_input = "Need at least 1 song".to_string()
                            }
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
                    KeyEvent {
                        code: KeyCode::Tab,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        myapp.current_tab = myapp.current_tab.next();
                    }

                    _ => {}
                }
            } else {
                continue;
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
    Ok(())
}

fn scan_folder_for_music() -> Vec<Song> {
    let current_folder = match dirs::audio_dir() {
        Some(dir) => dir,
        None => env::current_dir().unwrap(),
    };

    let song_paths = match fs::read_dir(&current_folder) {
        Ok(entries) => {
            let music_files: Vec<PathBuf> = entries
                .filter_map(|entry| {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if let Some(ext_str) = ext.to_str() {
                                    if MUSIC_FORMATS.contains(&ext_str) {
                                        Some(path)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            music_files
        }
        Err(e) => {
            eprintln!("Error reading directory: {}", e);
            panic!("awawawa");
        }
    };

    let mut song_list: Vec<Song> = Vec::new();
    for song in song_paths {
        let current_song;
        if song.ends_with("mp3") {
            let mp3_meta = read_from_file(&song).unwrap();
            let mp3_clone = read_from_file(&song).unwrap();
            let mp3_a = read_from_file(&song).unwrap();

            current_song = Song::new(
                mp3_meta.tag.unwrap().title,
                mp3_a.tag.unwrap().artist,
                None,
                song.clone(),
                mp3_clone.tag.unwrap().album,
                mp3_meta.duration.as_secs_f64(),
            );
        } else {
            let mut mp3_duration: f64 = 0.0;
            if song.extension().unwrap().to_str().unwrap() == "mp3" {
                mp3_duration = read_from_file(&song).unwrap().duration.as_secs_f64();
            }
            let meta = Tag::new().read_from_path(&song).unwrap();

            current_song = Song::new(
                meta.title().unwrap_or("No Title").to_string(),
                meta.artist().unwrap_or("No Title").to_string(),
                {
                    meta.album_cover().and_then(|cover| {
                        let format = match cover.mime_type {
                            audiotags::MimeType::Jpeg => ImageFormat::Jpeg,
                            audiotags::MimeType::Png => ImageFormat::Png,
                            audiotags::MimeType::Gif => ImageFormat::Gif,
                            audiotags::MimeType::Bmp => ImageFormat::Bmp,
                            audiotags::MimeType::Tiff => ImageFormat::Tiff,
                        };

                        load_from_memory_with_format(cover.data, format).ok()
                    })
                },
                song.clone(),
                meta.album()
                    .unwrap_or(Album {
                        title: "None",
                        artist: None,
                        cover: None,
                    })
                    .title
                    .to_string(),
                if let Some(ext) = song.extension().and_then(|e| e.to_str()) {
                    match ext {
                        "mp3" => mp3_duration,
                        _ => meta.duration().unwrap_or(0.0_f64),
                    }
                } else {
                    meta.duration().unwrap_or(0.0_f64)
                },
            );
        }
        song_list.push(current_song);
    }

    if song_list.is_empty() {
        song_list.push(Song::new(
            "No songs in \"Music\" and current directory!".to_string(),
            "No Title".to_string(),
            None,
            PathBuf::new(),
            Album {
                title: "None",
                artist: None,
                cover: None,
            }
            .title
            .to_string(),
            0.0_f64,
        ));
    }

    song_list
}

fn draw_popup(f: &mut Frame) -> Result<(), io::Error> {
    let size = f.area();
    let popup_width = size.width / 3;
    let popup_height = size.height / 3 + 8;
    let popup_area = Rect::new(
        (size.width - popup_width) / 2,
        (size.height - popup_height) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded),
        popup_area,
    );

    let popup_text = Paragraph::new(
        "Controls
- Use Up/Down Arrow Keys to navigate songs
- Ctrl + Spacebar: Play/Stop
- Ctrl + P: Pause/Unpause
- Ctrl + M: Mute/Unmute
- Ctrl + S: Change search criteria
- Ctrl + T: Change sorting criteria
- Ctrl + Left/Right Arrow Keys: Adjust Volume
- Ctrl + L: Next song
- Ctrl + H: Previous song
- Left Arrow Key: -5 seconds on current song
- Right Arrow Key: +5 seconds on current song
- Backspace: Delete characters in the search bar
- Ctrl + A: Select a song to be added
 to the new playlist
- Ctrl + C: New playlist name input popup
- Ctrl + K: Move playlist selection up
- Ctrl + J: Move playlist selection down
- Enter: Create a new playlist with given name
- Ctrl + X: Delete selected playlist
- F1: Toggle Controls Popup
- Esc or F1: Close Popup",
    )
    .block(Block::default().borders(Borders::NONE))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::White));
    f.render_widget(popup_text, popup_area);

    Ok(())
}

fn draw_playlist_name_input_popup(f: &mut Frame, input: &str) -> Result<(), io::Error> {
    let size = f.area();
    let popup_width = size.width / 4;
    let popup_height = size.height / 8;
    let popup_area = Rect::new(
        (size.width - popup_width) / 2,
        (size.height - popup_height) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(
        Block::default()
            .title("Enter Playlist Name")
            .borders(Borders::ALL),
        popup_area,
    );

    let inner_area = Rect::new(
        popup_area.x,
        popup_area.y + 2,
        popup_area.width,
        popup_area.height - 4,
    );

    // Display the current input inside the popup
    let input_text = Paragraph::new(input)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(input_text, inner_area);

    Ok(())
}

fn sort_songs(songs: &mut Vec<Song>, criteria: &SortCriteria) {
    match criteria {
        SortCriteria::Title => {
            songs.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
        SortCriteria::Artist => {
            songs.sort_by(|a, b| a.artist.to_lowercase().cmp(&b.artist.to_lowercase()));
        }
        SortCriteria::Duration => {
            songs.sort_by(|a, b| {
                a.duration
                    .partial_cmp(&b.duration)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_song_creation() {
        let title = String::from("Test Song");
        let artist = String::from("Test Artist");
        let path = PathBuf::from("/path/to/test/song.mp3");
        let album = String::from("Test Album");
        let duration = 180.0;
        let song = Song::new(
            title.clone(),
            artist.clone(),
            None,
            path.clone(),
            album.clone(),
            duration,
        );

        assert_eq!(song.title, title);
        assert_eq!(song.artist, artist);
        assert_eq!(song.path, path);
        assert_eq!(song.album, album);
        assert_eq!(song.duration, duration);
        assert_eq!(song.is_playing, false);
        assert_ne!(song.id, Uuid::nil());
    }

    #[test]
    fn test_scan_folder_for_music() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create a mock MP3 file
        let file_path = temp_path.join("test.mp3");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        // Create a mock FLAC file
        let file_path_flac = temp_path.join("test.flac");
        let mut file_flac = File::create(&file_path_flac).unwrap();
        file_flac.write_all(b"dummy content").unwrap();

        // Simulate the function behavior
        let mut songs = Vec::new();

        for entry in fs::read_dir(temp_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().unwrap().to_str().unwrap().to_lowercase();
                if MUSIC_FORMATS.contains(&extension.as_str()) {
                    let song = Song::new(
                        "Test Song".to_string(),
                        "Test Artist".to_string(),
                        None,
                        path.clone(),
                        "Test Album".to_string(),
                        180.0,
                    );
                    songs.push(song);
                }
            }
        }

        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].path.extension().unwrap(), "flac");
        assert_eq!(songs[1].path.extension().unwrap(), "mp3");
    }

    #[test]
    fn test_popup_state_toggle() {
        let mut popup_state = PopupState { visible: false };

        popup_state.toggle();
        assert_eq!(popup_state.visible, true);

        popup_state.toggle();
        assert_eq!(popup_state.visible, false);
    }

    #[test]
    fn test_sort_criteria() {
        assert_eq!(SortCriteria::Title.to_string(), "Title");
        assert_eq!(SortCriteria::Artist.to_string(), "Artist");
        assert_eq!(SortCriteria::Duration.to_string(), "Duration");

        assert_eq!(SortCriteria::Title.next(), SortCriteria::Artist);
        assert_eq!(SortCriteria::Artist.next(), SortCriteria::Duration);
        assert_eq!(SortCriteria::Duration.next(), SortCriteria::Title);
    }

    #[test]
    fn test_search_criteria() {
        let song1 = Song::new(
            "Song One".to_string(),
            "Artist A".to_string(),
            None,
            PathBuf::from("/path/to/song1.mp3"),
            "Album X".to_string(),
            200.0,
        );
        let song2 = Song::new(
            "Song Two".to_string(),
            "Artist B".to_string(),
            None,
            PathBuf::from("/path/to/song2.mp3"),
            "Album Y".to_string(),
            220.0,
        );

        let songs = vec![song1, song2];

        let search_text = "Song".to_string();
        let search_criteria = SearchCriteria::Title;

        let filtered_songs: Vec<&Song> = songs
            .iter()
            .filter(|s| match search_criteria {
                SearchCriteria::Title => {
                    s.title.to_lowercase().contains(&search_text.to_lowercase())
                }
                SearchCriteria::Artist => s
                    .artist
                    .to_lowercase()
                    .contains(&search_text.to_lowercase()),
                SearchCriteria::Album => {
                    s.album.to_lowercase().contains(&search_text.to_lowercase())
                }
            })
            .collect();

        assert_eq!(filtered_songs.len(), 2);
        assert_eq!(filtered_songs[0].title, "Song One");
        assert_eq!(filtered_songs[1].title, "Song Two");
    }
}
