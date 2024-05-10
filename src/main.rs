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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    let mut songs = scan_folder_for_music();
    let mut selected_song_index = 0;
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

            // Render search bar
            let search_bar = Paragraph::new(Text::raw(""))
                .block(Block::default().borders(Borders::ALL).title("Search"))
                .style(Style::default().fg(Color::White));
            f.render_widget(search_bar, vertical_layout[0]);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(vertical_layout[1]);

            let song_items: Vec<ListItem> = songs
                .iter()
                .enumerate()
                .map(|(index, song)| {
                    let mut style = Style::default();
                    if index == selected_song_index {
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

            let song_details = format!(
                "Artist: {}\nSong: {}\nAlbum: {}\nDuration: {:02}:{:02}",
                "None", "None", "None", 0, 0
            );
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
                .label(format!("100%"));

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
                    if selected_song_index < songs.len() - 1 {
                        selected_song_index += 1;
                    }
                }
                KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::NONE,
                } => {
                    // Move selection up
                    if selected_song_index > 0 {
                        selected_song_index -= 1;
                    }
                }
                KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::NONE,
                } => {
                    if let Some(selected_song) = songs.get(selected_song_index) {
                        if currently_playing_index.is_none()
                            || selected_song_index != currently_playing_index.unwrap()
                        {
                            selected_song.play(&sink);
                            currently_playing_index = Some(selected_song_index);
                        } else {
                            sink.lock().unwrap().clear();
                            currently_playing_index = None;
                        }
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
