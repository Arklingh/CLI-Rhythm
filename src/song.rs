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

use image::{DynamicImage, ImageFormat};
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
    pub cover_data: Option<Vec<u8>>, // Raw cover bytes, loaded on demand
    pub cover_mime_type: Option<String>, // Mime type for decoding
    pub path: PathBuf,
    pub album: String,
    pub duration: f64,
    pub is_playing: bool,
}

impl Song {
    /// Load cover image on demand (only when needed for display)
    pub fn load_cover(&self) -> Option<Arc<DynamicImage>> {
        let data = self.cover_data.as_ref()?;
        let format = match self.cover_mime_type.as_deref() {
            Some("image/jpeg") | Some("image/jpg") => ImageFormat::Jpeg,
            Some("image/png") => ImageFormat::Png,
            Some("image/gif") => ImageFormat::Gif,
            Some("image/bmp") => ImageFormat::Bmp,
            Some("image/tiff") => ImageFormat::Tiff,
            _ => ImageFormat::Jpeg, // Default
        };
        image::load_from_memory_with_format(data, format).ok().map(Arc::new)
    }

    pub fn new(
        title: String,
        artist: String,
        cover_data: Option<Vec<u8>>,
        cover_mime_type: Option<String>,
        path: PathBuf,
        album: String,
        duration: f64,
    ) -> Result<Self, String> {
        let path_str = path
            .to_str()
            .ok_or_else(|| format!("Invalid UTF-8 path: {:?}", path))?;

        Ok(Song {
            id: Uuid::new_v5(&Uuid::NAMESPACE_DNS, path_str.as_bytes()),
            title,
            artist,
            cover_data,
            cover_mime_type,
            path,
            album,
            duration,
            is_playing: false,
        })
    }

    pub fn play(&self, sink: &Arc<Mutex<Sink>>) -> Result<(), Box<dyn std::error::Error>> {
        let file = fs::File::open(&self.path)?;
        let source = rodio::Decoder::new(io::BufReader::new(file))?;
        {
            let sink_guard = sink
                .lock()
                .map_err(|_| "Failed to acquire audio sink lock")?;
            sink_guard.clear();
            sink_guard.append(source);
            sink_guard.play();
        }
        Ok(())
    }
}
