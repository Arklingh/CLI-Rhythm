# CLI-Rhythm

cli-rhythm is a simple, functional, and lightweight Command-Line Interface (CLI) music player. It is designed to provide a seamless music-playing experience directly from your terminal. Built with Rust, it supports intuitive navigation, and a minimal user interface for distraction-free listening.

![зображення](https://github.com/user-attachments/assets/b313abe8-d93d-449f-8997-9886f74bf8f4)

## Features

- **Play music from your local library**: Easily scan and play music files from a directory.
- **Supported Formats**: MP3, WAV, FLAC, AAC.
- **Minimalistic interface**: Clean and simple UI for focusing on music.
- **Keyboard shortcuts**: Navigate and control the player entirely via keyboard.
- **Metadata extraction**: Automatically extracts song information such as artist, album, and track title.

# Installation

## Installing precompiled binary(Windows only)
Preompiled binary is availiable in [Releases](https://github.com/Arklingh/CLI-Rhythm/releases).

## Building from source
1. Ensure you have Rust installed.
2. Clone the repository:
```bash
git clone https://github.com/yourusername/cli-rhythm.git
cd cli-rhythm
```
3. Build the application:
```bash
cargo build --release
```
4. Run the player:
```bash
./target/release/cli-rhythm
```
## Usage

cli-rhythm scans your system's default music folder for music files. If it doesn't find any there, it will scan the folder cli-rhythm executable is in.

## Controls

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
- Esc or F1: Close Popup

## Planned Features

- [ ] **Creating Playlists**: Allow users to group song by preference.
- [ ] **Support for Additional File Formats**: Extend compatibility to more audio formats such as AAC, OGG, and AIFF.
- [ ] **Equalizer Support**: Implement an equalizer to adjust audio frequencies for a more customized listening experience.
- [ ] **Configurable Key Bindings**: Allow users to customize keyboard shortcuts according to their preferences.
- [ ] **Cross-Platform Support**: Ensure the application runs smoothly on Windows, macOS, and Linux.
- [ ] **Advanced Playlist Management**: Enhance playlist functionality with features like shuffle, repeat, and smart playlists.
- [ ] **Lyrics Display**: Add support for displaying lyrics if available in the metadata.
- [ ] **Streaming Support**: Implement support for streaming music from online sources or services.
- [ ] **Improved Metadata Handling**: Enhance metadata extraction and display, including album artwork.

## License

This project is licensed under the Apache License 2.0.
