use crate::song::Song;
use audiotags::{types::Album, Tag};
use dirs;
use image::{load_from_memory_with_format, ImageFormat};
use mp3_metadata::read_from_file;
use std::env;
use std::fs;
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
}

impl SortCriteria {
    /// Returns the next sorting criteria in the sequence.
    pub fn next(&self) -> SortCriteria {
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

pub fn scan_folder_for_music() -> Vec<Song> {
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
    }
}
