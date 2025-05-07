# CLI-Rhythm

cli-rhythm is a simple, functional, and lightweight Command-Line Interface (CLI) music player. It is designed to provide a seamless music-playing experience directly from your terminal. Built with Rust, it supports intuitive navigation, and a minimal user interface for distraction-free listening.

![зображення](https://github.com/user-attachments/assets/c93def08-cb3d-4dce-84f4-1c881c537b15)

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
git clone https://github.com/Arklingh/cli-rhythm.git
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

| Platform | Music folder example |
| -------- | ------- |
| Linux | /home/alice/Music |
| macOS | /Users/Alice/Music |
| Windows | C:\Users\Alice\Music |

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
- Ctrl + A: Select a song to be added
 to the new playlist
- Ctrl + C: New playlist name input popup
- Ctrl + K: Move playlist selection up
- Ctrl + J: Move playlist selection down
- Enter: Create a new playlist with given name
- Ctrl + X: Delete selected playlist
- F1: Toggle Controls Popup
- Esc or F1: Close Popup

## Planned Features

- [x] **Creating Playlists**: Allow users to group song by preference.
- [ ] **Shuffle & Repeat Modes**: Add options for shuffling songs in a playlist or repeating a song/playlist.
- [ ] **Cross-Platform Support**: Ensure the application runs smoothly on Windows, macOS, and Linux.
- [x] **Adaptivity to Different Screen Resolutions**: Ensure app's defined behaviour for different resolutions.

## Known Issues
- Fast consecutive song's volume and progress changes move the progress bar back.   
- If you encounter any problems, please [open an issue](https://github.com/Arklingh/CLI-Rhythm/issues).

## License

This project is licensed under the MIT License. 
