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
extern crate tui;

use std::env;
use std::io::stdout;
use std::ops::{Add, Sub};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, io};

use crossterm::event::{poll, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use rodio::{OutputStream, Sink, Source};
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use tui::Terminal;

use dirs;
use textwrap::wrap;

use audiotags::{types::Album, Tag};
use mp3_metadata::read_from_file;

/// Supported music file formats.
const MUSIC_FORMATS: [&str; 4] = ["mp3", "wav", "flac", "aac"];

/// Represents a song with metadata.
#[derive(Debug, PartialEq, PartialOrd, Default, Clone)]
struct Song {
    /// Title of the song.
    title: String,
    /// Artist of the song.
    artist: String,
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
    fn new(title: String, artist: String, path: PathBuf, album: String, duration: f64) -> Self {
        Song {
            title,
            artist,
            path,
            album,
            duration,
            is_playing: false,
        }
    }

    fn play(&self, sink: &Arc<Mutex<Sink>>) {
        let file = fs::File::open(&self.path).unwrap();
        let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();
        sink.lock().unwrap().append(source);
        sink.lock().unwrap().play();
    }
}

enum SearchCriteria {
    Title,
    Artist,
    Album,
}

#[derive(PartialEq, Eq, Debug)]
enum SortCriteria {
    Title,
    Artist,
    Duration,
}

impl SortCriteria {
    fn next(&self) -> SortCriteria {
        match self {
            SortCriteria::Title => SortCriteria::Artist,
            SortCriteria::Artist => SortCriteria::Duration,
            SortCriteria::Duration => SortCriteria::Title,
        }
    }

    fn to_string(&self) -> &str {
        match self {
            SortCriteria::Title => "Title",
            SortCriteria::Artist => "Artist",
            SortCriteria::Duration => "Duration",
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    let mut songs = Box::new(scan_folder_for_music());

    let mut filtered_songs: Vec<&Song>;
    let mut visible_song_count: usize = 0;

    let mut previous_volume = 1.0;

    let mut selected_song_index: Option<usize> = None;

    let mut list_offset: usize = 0;

    let mut search_text = String::new();
    let mut search_criteria = SearchCriteria::Title;
    let mut sort_criteria = SortCriteria::Title;
    let mut currently_playing_index: Option<usize> = None;
    let mut song_time: Option<Instant> = None;

    sort_songs(&mut songs, &sort_criteria);

    let mut paused_time: Option<Instant> = None;

    let mut popup_state = PopupState { visible: false };

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap()));

    // Run event loop
    loop {
        let search_bar_title = match search_criteria {
            SearchCriteria::Title => "Search by Title",
            SearchCriteria::Artist => "Search by Artist",
            SearchCriteria::Album => "Search by Album",
        };

        // Render search bar
        let search_bar = Paragraph::new(Text::raw(format!("{}", search_text)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(search_bar_title),
            )
            .style(Style::default().fg(Color::White));

        // Filter songs based on search text
        filtered_songs = songs
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

        let selected_song = match selected_song_index {
            Some(index) => filtered_songs.get(index),
            None => None,
        };

        let song_details = if let Some(song) = selected_song {
            let contents = format!(
                "Artist: {}\nSong: {}\nAlbum: {}\nDuration: {:02}:{:02}",
                song.artist,
                song.title,
                song.album,
                (song.duration / 60.0).floor(),
                (song.duration % 60.0).round()
            );
            let wrapped_details = wrap(&contents, 23);

            wrapped_details.join("\n")
        } else {
            "No song selected".to_string()
        };

        let selected_song_info = Paragraph::new(song_details)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Selected Song"),
            )
            .style(Style::default().fg(Color::Cyan));

        if let Some(index) = currently_playing_index {
            if songs[index].is_playing {
                song_time =
                    Some(song_time.unwrap_or(Instant::now()) + Duration::from_secs_f64(0.1));
            }
        };

        let progress_ratio =
            match songs.get(currently_playing_index.unwrap_or(selected_song_index.unwrap_or(0))) {
                Some(song) if song.duration > 0.0 && !sink.lock().unwrap().is_paused() => {
                    if let Some(song_time) = song_time {
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
                    if let Some(song_time) = song_time {
                        if let Some(paused_time) = paused_time {
                            let elapsed_time = song_time.elapsed().as_secs_f64().min(song.duration);
                            ratio = (elapsed_time - paused_time.elapsed().as_secs_f64())
                                / song.duration;
                        }
                    }
                    ratio
                }
                _ => 0.0,
            };

        let song_progress = if let Some(song) =
            songs.get(currently_playing_index.unwrap_or(selected_song_index.unwrap_or(0)))
        {
            let elapsed_time = if let Some(paused_time) = paused_time {
                song_time
                    .unwrap_or(Instant::now())
                    .elapsed()
                    .as_secs_f64()
                    .sub(paused_time.elapsed().as_secs_f64())
                    .min(song.duration)
            } else {
                song_time
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
                    Constraint::Percentage(10),
                    Constraint::Percentage(80),
                    Constraint::Percentage(10),
                ])
                .split(f.size());

            f.render_widget(search_bar, vertical_layout[0]);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(vertical_layout[1]);

            visible_song_count = (chunks[0].height - 2) as usize;

            let song_items: Vec<ListItem> = filtered_songs
                .iter()
                .enumerate()
                .skip(list_offset)
                .take(visible_song_count as usize)
                .map(|(index, song)| {
                    let mut style = Style::default();
                    if selected_song_index.is_some_and(|select| select == index) {
                        style = Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD);
                    }
                    ListItem::new(song.title.clone()).style(style)
                })
                .collect();

            let song_list = List::new(song_items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "Songs-----------------------------------------------------------------------------------------Sort by: {}",
                    sort_criteria.to_string()
                )))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(song_list, chunks[0]);

            f.render_widget(selected_song_info, chunks[1]);

            let footer = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(vertical_layout[2]);

            f.render_widget(song_progress, footer[0]);

            f.render_widget(volume_bar, footer[1]);

            if popup_state.visible {
                let _ = draw_popup(f);
            }

            f.render_widget(
                hint,
                Rect::new(
                    f.size().width.saturating_sub(20 as u16),
                    f.size().height - 1,
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
                        break;
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        // Move selection down
                        if let Some(index) = selected_song_index {
                            if index < filtered_songs.len() - 1 {
                                selected_song_index = Some(index + 1);

                                // Scroll down if selected index goes out of view
                                if let Some(selected_index) = selected_song_index {
                                    if selected_index >= list_offset + visible_song_count - 1 {
                                        list_offset =
                                            (selected_index - visible_song_count + 2).max(0);

                                        // Ensure the list_offset does not exceed the maximum allowed offset
                                        list_offset = (list_offset).min(
                                            filtered_songs.len().saturating_sub(visible_song_count),
                                        );
                                    }
                                }
                            } else {
                                selected_song_index = Some(0);
                                list_offset = 0;
                            }
                        } else if !filtered_songs.is_empty() {
                            selected_song_index = Some(0);
                            list_offset = 0;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        // Move selection up
                        if let Some(index) = selected_song_index {
                            if index > 0 {
                                selected_song_index = Some(index - 1);

                                // Scroll up if selected index goes out of view
                                if index <= list_offset {
                                    list_offset = list_offset.saturating_sub(1);
                                }
                            } else {
                                selected_song_index = Some(filtered_songs.len() - 1);
                                list_offset =
                                    filtered_songs.len().saturating_sub(visible_song_count);
                            }
                        } else if !filtered_songs.is_empty() {
                            selected_song_index = Some(filtered_songs.len() - 1);
                            list_offset = filtered_songs.len().saturating_sub(visible_song_count);
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char(' '),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        if let Some(index) = selected_song_index {
                            if let Some(selected_song) = filtered_songs.get(index) {
                                if currently_playing_index.is_none()
                                    || Some(index) != currently_playing_index
                                {
                                    sink.lock().unwrap().clear();
                                    selected_song.play(&sink);
                                    song_time = Some(Instant::now());
                                    currently_playing_index = Some(index);
                                    // Set is_playing field to true
                                    songs[index].is_playing = true;
                                } else {
                                    sink.lock().unwrap().clear();
                                    song_time = None;
                                    currently_playing_index = None;

                                    // Set is_playing field to false
                                    if let Some(index) = currently_playing_index {
                                        if songs[index].is_playing {
                                            if let Some(start_time) = song_time {
                                                let elapsed_time = start_time
                                                    .elapsed()
                                                    .as_secs_f64()
                                                    .min(songs[index].duration);
                                                let adjusted_time = if let Some(paused_at) =
                                                    paused_time
                                                {
                                                    elapsed_time + paused_at.elapsed().as_secs_f64()
                                                } else {
                                                    elapsed_time
                                                };
                                                song_time = Some(
                                                    start_time
                                                        + Duration::from_secs_f64(adjusted_time),
                                                );
                                            } else {
                                                song_time = Some(Instant::now());
                                            }
                                        }
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
                            if let Some(current_index) = currently_playing_index {
                                sink.lock().unwrap().play();
                                songs[current_index].is_playing = true;
                            }
                            // Calculate elapsed time during the pause
                            if let Some(paused_at) = paused_time {
                                let elapsed_during_pause = paused_at.elapsed();
                                song_time = song_time.map(|t| t + elapsed_during_pause);
                                paused_time = None;
                            }
                        } else {
                            if let Some(index) = currently_playing_index {
                                sink.lock().unwrap().pause();
                                songs[index].is_playing = false;
                                // Record the time when playback was paused
                                paused_time = Some(Instant::now());
                            }
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char('h'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        if currently_playing_index.is_some_and(|idx| idx > 0) {
                            let mut idx = currently_playing_index.unwrap();
                            idx -= 1;
                            sink.lock().unwrap().clear();
                            songs[idx].play(&sink);
                            currently_playing_index = Some(idx);
                            selected_song_index = Some(idx);
                            song_time = Some(Instant::now());
                            paused_time = None; // Reset paused time when starting a new song
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        if currently_playing_index.is_some_and(|idx| idx < songs.len() - 1) {
                            let mut idx = currently_playing_index.unwrap();
                            idx += 1;
                            sink.lock().unwrap().clear();
                            songs[idx].play(&sink);
                            selected_song_index = Some(idx);
                            currently_playing_index = Some(idx);
                            song_time = Some(Instant::now());
                            paused_time = None; // Reset paused time when starting a new song
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
                            previous_volume = sink.volume(); // Save current volume
                            sink.set_volume(0.0);
                        } else {
                            // Unmute music
                            sink.set_volume(previous_volume); // Restore previous volume
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        search_text.push(c);
                    }
                    KeyEvent {
                        code: KeyCode::Char(c),
                        modifiers: KeyModifiers::SHIFT,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        search_text.push(c.to_uppercase().last().unwrap());
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        search_text.pop();
                    }
                    KeyEvent {
                        code: KeyCode::Char('s'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        search_criteria = match search_criteria {
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
                        sort_criteria = sort_criteria.next();
                        sort_songs(&mut songs, &sort_criteria);
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        // !!!!!!!!!!!!!!!!!!!!!!!! problem during search
                        if let Some(index) = currently_playing_index {
                            let file = fs::File::open(&songs[index].path).unwrap();
                            let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();
                            let time = song_time.unwrap().sub(Duration::from_secs(5));
                            song_time = Some(time);
                            let source = source.skip_duration(time.elapsed());
                            sink.lock().unwrap().clear();
                            sink.lock().unwrap().append(source);
                            sink.lock().unwrap().play();
                            if !&songs[index].is_playing {
                                sink.lock().unwrap().pause();
                            }
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Left,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        if let Some(index) = currently_playing_index {
                            let file = fs::File::open(&songs[index].path).unwrap();
                            let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();
                            let time = song_time.unwrap().add(Duration::from_secs(5));
                            song_time = Some(time);
                            let source = source.skip_duration(time.elapsed());
                            sink.lock().unwrap().clear();
                            sink.lock().unwrap().append(source);
                            sink.lock().unwrap().play();
                            if !&songs[index].is_playing {
                                sink.lock().unwrap().pause();
                            }
                        }
                    }
                    KeyEvent {
                        code: KeyCode::F(1),
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        popup_state.toggle();
                    }
                    KeyEvent {
                        code: KeyCode::Esc,
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        // Close the popup if it's open
                        popup_state.visible = false;
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
    stdout().execute(LeaveAlternateScreen)?;
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

fn draw_popup(f: &mut tui::Frame<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    let size = f.size();
    let popup_width = size.width / 3;
    let popup_height = size.height / 3 + 1;
    let popup_area = Rect::new(
        (size.width - popup_width) / 2,
        (size.height - popup_height) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(Block::default().borders(Borders::ALL), popup_area);

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
- F1: Toggle Controls Popup
- Esc or F1: Close Popup",
    )
    .block(Block::default().borders(Borders::NONE))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::White));
    f.render_widget(popup_text, popup_area);

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
            PathBuf::from("/path/to/song1.mp3"),
            "Album X".to_string(),
            200.0,
        );
        let song2 = Song::new(
            "Song Two".to_string(),
            "Artist B".to_string(),
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
