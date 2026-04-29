//! Music Metadata Handling and Song Management
//!
//! This module provides core functionality for scanning the filesystem
//! for music files, extracting metadata, handling popup state, and
//! managing search and sorting criteria.
//!
//! Key Components:
//! - `scan_folder_for_music`: Scans the user's music or current directory
//!   for supported formats (`mp3`, `wav`, `flac`, `aac`), extracts tags
//!   using `audiotags` and `mp3_metadata`, and constructs `Song` instances.
//!
//! - `PopupState`: Stores visibility state for popups (like help or input dialogs).
//!
//! - `SearchCriteria` and `SortCriteria`: Enums for defining user-selectable
//!   filters and sorting logic.
//!
//! - `sort_songs`: Sorts a vector of `Song` instances by title, artist, or duration.
//!
//! Additional Notes:
//! - Song metadata includes album art decoding via `image` crate.
//! - The system gracefully handles cases where metadata or song files are missing or incomplete.

use crate::song::Song;
use audiotags::Tag;
use dirs;
use mp3_metadata::read_from_file;
use rand::{rng, seq::SliceRandom};
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub const MUSIC_FORMATS: [&str; 4] = ["mp3", "wav", "flac", "aac"];

pub struct PopupState {
    pub visible: bool,
}

impl PopupState {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

/// Enum representing the criteria for searching songs.
#[derive(PartialEq, Clone)]
pub enum SearchCriteria {
    Title,
    Artist,
    Album,
}

/// Enum representing the criteria for sorting songs.
#[derive(PartialEq, Eq, Debug)]
pub enum SortCriteria {
    Title,
    Artist,
    Duration,
    Shuffle,
}

impl SortCriteria {
    /// Returns the next sorting criteria in the sequence.
    pub fn next(&self) -> SortCriteria {
        match self {
            SortCriteria::Title => SortCriteria::Artist,
            SortCriteria::Artist => SortCriteria::Duration,
            SortCriteria::Duration => SortCriteria::Shuffle,
            SortCriteria::Shuffle => SortCriteria::Title,
        }
    }
}

impl fmt::Display for SortCriteria {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SortCriteria::Title => "Title",
                SortCriteria::Artist => "Artist",
                SortCriteria::Duration => "Duration",
                SortCriteria::Shuffle => "Shuffled",
            }
        )
    }
}

/// Additional func to safely parse metadata
/// Returns Option<Song> to allow filter_map to skip bad files
fn parse_song_metadata(path: &Path) -> Option<Song> {
    let ext = path.extension().and_then(|e| e.to_str())?;

    // Read safely through .ok()?. If file is bad - break and return None
    let meta = Tag::new().read_from_path(path).ok()?;

    // Store raw cover bytes instead of decoded image (saves ~90% memory)
    let (cover_data, cover_mime_type) = meta.album_cover().map(|cover| {
        let mime = match cover.mime_type {
            audiotags::MimeType::Jpeg => "image/jpeg",
            audiotags::MimeType::Png => "image/png",
            audiotags::MimeType::Gif => "image/gif",
            audiotags::MimeType::Bmp => "image/bmp",
            audiotags::MimeType::Tiff => "image/tiff",
        };
        (cover.data.to_vec(), mime.to_string())
    }).unzip();

    // Unified length logic(read_from_file for mp3, standart for else)
    let duration = if ext == "mp3" {
        read_from_file(path)
            .map(|mp3_meta| mp3_meta.duration.as_secs_f64())
            .unwrap_or_else(|_| meta.duration().unwrap_or(0.0))
    } else {
        meta.duration().unwrap_or(0.0)
    };

    Song::new(
        meta.title().unwrap_or("No Title").to_string(),
        meta.artist().unwrap_or("Unknown Artist").to_string(),
        cover_data,
        cover_mime_type,
        path.to_path_buf(),
        meta.album()
            .map(|a| a.title.to_string())
            .unwrap_or_else(|| "None".to_string()),
        duration,
    ).ok()
}

pub fn scan_folder_for_music() -> Vec<Song> {
    // Dir
    let current_folder = dirs::audio_dir().unwrap_or_else(|| env::current_dir().unwrap());

    // Scan and filter through iters
    let song_paths: Vec<PathBuf> = fs::read_dir(&current_folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok) // Ігноруємо помилки доступу до окремих файлів
                .map(|entry| entry.path())
                .filter(|path| path.is_file()) // Беремо тільки файли
                .filter(|path| {
                    // Checking extention
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext_str| MUSIC_FORMATS.contains(&ext_str))
                })
                .collect()
        })
        .unwrap_or_else(|e| {
            eprintln!("Error reading directory: {e}");
            eprintln!("Please ensure the directory exists and you have read permissions.");
            Vec::new()
        });

    let mut song_list: Vec<Song> = song_paths
        .into_iter()
        .filter_map(|path| parse_song_metadata(&path))
        .collect();

    if song_list.is_empty() {
        song_list.push(Song::new(
            "No songs in \"Music\" and current directory!".to_string(),
            "No Title".to_string(),
            None,
            None,
            PathBuf::new(),
            "None".to_string(),
            0.0,
        ).expect("Failed to create placeholder song; PathBuf::new() should always be valid for UUID generation."));
    }

    song_list
}

pub fn sort_songs(songs: &mut Vec<Song>, criteria: &SortCriteria) {
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
        SortCriteria::Shuffle => {
            let mut rand = rng();
            songs.shuffle(&mut rand);
        }
    }
}
