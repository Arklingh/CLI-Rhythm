extern crate crossterm;
extern crate tui;

use std::io;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

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

            let song_items: Vec<ListItem> = vec![0, 1, 2]
                .iter()
                .enumerate()
                .map(|(_index, song)| {
                    let style = Style::default();

                    ListItem::new(song.to_string().clone()).style(style)
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
                .label(format!("100%"))
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
                _ => {}
            }
        }
    }

    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;

    // Cleanup
    disable_raw_mode()?;
    Ok(())
}
