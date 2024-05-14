extern crate crossterm;
extern crate tui;

use std::env;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fs, io};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear};
use crossterm::ExecutableCommand;
use rodio::{OutputStream, Sink};
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::Text;
use tui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use tui::Terminal;

use dirs;
use textwrap::wrap;

use audiotags::{types::Album, Tag};
use mp3_metadata::read_from_file;

const MUSIC_FORMATS: [&str; 10] = [
    "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "alac", "ape", "opus",
];

#[derive(Debug, PartialEq, PartialOrd, Default)]
struct Song {
    title: String,
    artist: String,
    path: PathBuf,
    album: String,
    duration: f64,
    elapsed: f64,
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
            elapsed: 0.0,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    let mut songs = scan_folder_for_music();
    let mut filtered_songs: Vec<&Song> = Vec::new();

    let mut previous_volume = 1.0;

    let mut selected_song_index: Option<usize> = None;

    let mut search_text = String::new();
    let mut search_criteria = SearchCriteria::Title;

    // Sort songs alphabetically by title
    songs.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    let mut currently_playing_index: Option<usize> = None;

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap()));

    // Run event loop
    loop {
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
            f.render_widget(search_bar, vertical_layout[0]);

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

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(vertical_layout[1]);

            let song_items: Vec<ListItem> = filtered_songs
                .iter()
                .enumerate()
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
                .block(Block::default().borders(Borders::ALL).title("Songs"))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(song_list, chunks[0]);

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
            f.render_widget(selected_song_info, chunks[1]);

            let footer = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(vertical_layout[2]);

            let song_progress = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(Style::default().fg(Color::White))
                .label("");

            f.render_widget(song_progress, footer[0]);

            // Volume bar
            let volume_bar = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Volume"))
                .gauge_style(Style::default().fg(Color::LightBlue))
                .label(format!("{:.0}%", sink.lock().unwrap().volume() * 100.0))
                .ratio(sink.lock().unwrap().volume() as f64);

            f.render_widget(volume_bar, footer[1]);
        })?;

        // Handle input events
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
                        } else {
                            selected_song_index = Some(0);
                        }
                    } else if !filtered_songs.is_empty() {
                        selected_song_index = Some(0);
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
                        } else {
                            selected_song_index = Some(filtered_songs.len() - 1);
                        }
                    } else if !filtered_songs.is_empty() {
                        selected_song_index = Some(filtered_songs.len() - 1);
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
                                currently_playing_index = Some(index);
                            } else {
                                sink.lock().unwrap().clear();
                                currently_playing_index = None;
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
                        sink.lock().unwrap().play();
                    } else {
                        sink.lock().unwrap().pause();
                    }
                }
                KeyEvent {
                    code: KeyCode::Char('h'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::NONE,
                } => {
                    if selected_song_index.is_some_and(|idx| idx > 0) {
                        let mut idx = selected_song_index.unwrap();
                        idx -= 1;
                        sink.lock().unwrap().clear();
                        songs[idx].play(&sink);
                        currently_playing_index = Some(idx);
                    }
                }
                KeyEvent {
                    code: KeyCode::Char('l'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::NONE,
                } => {
                    if selected_song_index.is_some_and(|idx| idx < songs.len() - 1) {
                        let mut idx = selected_song_index.unwrap();
                        idx += 1;
                        sink.lock().unwrap().clear();
                        songs[idx].play(&sink);
                        currently_playing_index = Some(idx);
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

                _ => {}
            }
        }
    }

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    // Cleanup
    disable_raw_mode()?;
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
            "No songs in current directory!".to_string(),
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
