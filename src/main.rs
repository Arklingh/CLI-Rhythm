//! # CLI-Rhythm
//!
//! CLI-Rhythm is a terminal-based music player written in Rust, designed for a minimalistic and efficient command-line interface. It allows users to manage and play their music collection directly from the terminal, offering features such as sorting, searching, and playback controls. Built with a focus on simplicity and performance, CLI-Rhythm provides an intuitive experience for music enthusiasts who prefer a text-based environment.
//!
//! ## Features
//! - Play music files from supported formats.
//! - Sort and search for songs by title, artist, or album.
//! - Navigate through a list of songs with ease.
//! - Minimal resource usage with a clean terminal interface.
//! - Control playback with keyboard shortcuts.
//! - Save and load playlists for quick access.

/// Problems
/// - No +/- 5 seconds on current song!!!
/// - No Mouse support

extern crate crossterm;
extern crate ratatui;

mod song;
mod app;
mod ui;
mod utils;
mod input_handler;

use app::MyApp;
use crossterm::event::{poll, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear};
use crossterm::ExecutableCommand;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use rodio::{OutputStream, Sink};
use std::io::stdout;
use std::ops::Sub;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, io};
use utils::sort_songs;
use ui::{draw_popup, draw_playlist_name_input_popup};
use utils::SearchCriteria;
use textwrap::wrap;
use image::{ImageBuffer, Rgba, DynamicImage};
use flume;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()?;
    let mut terminal = ratatui::init();
    let picker = Picker::from_fontsize((7, 14));
    let (clock_to_main_sender, clock_to_main_recv) = flume::unbounded();
    let stop_signal = Arc::new(AtomicBool::new(false));
    let mut exit_code = false;

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

    let mut time_thread: Option<std::thread::JoinHandle<()>> = None;
    let mut elapsed_time = Duration::default();
    // Run event loop
    loop {
        if myapp.currently_playing_song.is_some() && time_thread.is_none() && myapp.paused_time.is_none() {
            let clone_send = clock_to_main_sender.clone();
            let stop_signal_clone = stop_signal.clone();
            time_thread = Some(std::thread::spawn(move || {
                loop {
                    if stop_signal_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    clone_send.send(Some(Instant::now())).unwrap();
                    std::thread::sleep(Duration::from_millis(100));
                }
                clone_send.send(None).unwrap();
                stop_signal_clone.store(false, std::sync::atomic::Ordering::Relaxed);
            }));
        } else if myapp.currently_playing_song.is_none() && time_thread.is_some() {
            stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(handle) = time_thread.take() {
                handle.join().unwrap();
            }
            stop_signal.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        if let Ok(Some(a)) = clock_to_main_recv.try_recv() {
            if myapp.currently_playing_song.is_some() {
                //dbg!(a);
                elapsed_time += Duration::from_millis(100);
                myapp.song_time = Some(elapsed_time);
            }
        }

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
                    // If the song is finished, play the next one
                    if myapp.song_time.unwrap().as_secs_f64() >= song.duration {

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
                            elapsed_time = Duration::default();
                            myapp.song_time = Some(elapsed_time);
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
                    let elapsed_time = song_time.as_secs_f64().min(song.duration);
                    if elapsed_time >= song.duration {
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
                        let elapsed_time = song_time.as_secs_f64().min(song.duration);
                        ratio = (elapsed_time - paused_time.as_secs_f64()).max(0.0) / song.duration;
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
                    .unwrap_or(Duration::default())
                    .as_secs_f64()
                    .sub(paused_time.as_secs_f64())
                    .min(song.duration)
            } else {
                myapp
                    .song_time
                    .unwrap_or(Duration::default())
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

        let mut playlist_bounds = None;
        let mut song_list_bounds = None;
        let mut volume_bar_bounds = None;

        terminal.draw(|f| {
            let vertical_layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Fill(1),
                ])
                .split(f.area());

            let song_tab_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(7),
                    Constraint::Percentage(86),
                    Constraint::Percentage(7),
                ])
                .split(vertical_layout[0]);
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

            playlist_bounds = Some(chunks[0]);
            f.render_widget(playlist_list, chunks[0]);
            song_list_bounds = Some(chunks[1]);
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
            volume_bar_bounds = Some(footer[1]);
            f.render_widget(volume_bar, footer[1]);

            if myapp.hint_popup_state.visible {
                let _ = draw_popup(f);
            }

            if myapp.playlist_input_popup.visible {
                let _ = draw_playlist_name_input_popup(f, &myapp.playlist_name_input);
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
            match crossterm::event::read()? {
                Event::Key(key) => {
                    input_handler::handle_key_event(key, &mut myapp, &sink, visible_song_count, visible_playlist_count, &mut exit_code);
                    if exit_code {
                        break;
                    }
                    // Stop the time thread if the song is paused
                    if myapp.paused_time.is_some() && time_thread.is_some() {
                        stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
                        if let Some(handle) = time_thread.take() {
                            handle.join().unwrap();
                        }
                        stop_signal.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                _ => {}
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
    Ok(())
}
