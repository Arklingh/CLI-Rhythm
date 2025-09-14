//! Song Struct and Playback Implementation for TUI Music App
//!
//! This module defines the `Song` struct which represents a music track in the application.
//! Each song holds metadata such as title, artist, album, cover image, duration, file path,
//! and a playback status flag.
//!
//! Key Features:
//! - Each `Song` instance is uniquely identified using a UUID (based on its file path).
//! - Optional support for cover art using the `image` crate's `DynamicImage`.
//! - Includes a method `play` to stream and play the song using `rodio` audio playback.
//!
//! Dependencies include:
//! - `rodio` for audio decoding and playback.
//! - `uuid` for unique song identification.
//! - `image` for optional album cover handling.
//! - `Arc<Mutex<Sink>>` for shared and safe control of audio playback across threads.

use image::DynamicImage;
use rodio::Sink;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct Song {
    pub id: Uuid,
    pub title: String,
    pub artist: String,
    pub cover: Option<DynamicImage>,
    pub path: PathBuf,
    pub album: String,
    pub duration: f64,
    pub is_playing: bool,
    // Cached lowercase versions for performance
    pub title_lower: String,
    pub artist_lower: String,
    pub album_lower: String,
}

impl Song {
    pub fn new(
        title: String,
        artist: String,
        cover: Option<DynamicImage>,
        path: PathBuf,
        album: String,
        duration: f64,
    ) -> Self {
        let title_lower = title.to_lowercase();
        let artist_lower = artist.to_lowercase();
        let album_lower = album.to_lowercase();
        
        Song {
            id: Uuid::new_v5(&Uuid::NAMESPACE_DNS, path.to_str().unwrap().as_bytes()),
            title,
            artist,
            cover,
            path,
            album,
            duration,
            is_playing: false,
            title_lower,
            artist_lower,
            album_lower,
        }
    }

    pub fn play(&self, sink: &Arc<Mutex<Sink>>) -> Result<(), Box<dyn std::error::Error>> {
        let file = fs::File::open(&self.path)?;
        let source = rodio::Decoder::new(io::BufReader::new(file))?;
        {
            let sink_guard = sink.lock().map_err(|_| "Failed to acquire audio sink lock")?;
            sink_guard.clear();
            sink_guard.append(source);
            sink_guard.play();
        }
        Ok(())
    }
}
