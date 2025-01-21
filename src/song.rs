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
}

impl Song {
    pub fn new(title: String, artist: String, cover: Option<DynamicImage>, path: PathBuf, album: String, duration: f64) -> Self {
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

    pub fn play(&self, sink: &Arc<Mutex<Sink>>) {
        let file = fs::File::open(&self.path).unwrap();
        let source = rodio::Decoder::new(io::BufReader::new(file)).unwrap();
        sink.lock().unwrap().append(source);
        sink.lock().unwrap().play();
    }
}
