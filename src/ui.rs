use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::io;

pub fn draw_popup(f: &mut Frame) -> Result<(), io::Error> {
    let size = f.area();
    let popup_width = size.width / 3;
    let popup_height = size.height / 3 + 8;
    let popup_area = Rect::new(
        (size.width - popup_width) / 2,
        (size.height - popup_height) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded),
        popup_area,
    );

    let popup_text = Paragraph::new(
        "Controls
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
- Esc or F1: Close Popup",
    )
    .block(Block::default().borders(Borders::NONE))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::White));
    f.render_widget(popup_text, popup_area);

    Ok(())
}

pub fn draw_playlist_name_input_popup(f: &mut Frame, input: &str) -> Result<(), io::Error> {
    let size = f.area();
    let popup_width = size.width / 4;
    let popup_height = size.height / 8;
    let popup_area = Rect::new(
        (size.width - popup_width) / 2,
        (size.height - popup_height) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(
        Block::default()
            .title("Enter Playlist Name")
            .borders(Borders::ALL),
        popup_area,
    );

    let inner_area = Rect::new(
        popup_area.x,
        popup_area.y + 2,
        popup_area.width,
        popup_area.height - 4,
    );

    // Display the current input inside the popup
    let input_text = Paragraph::new(input)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(input_text, inner_area);

    Ok(())
}