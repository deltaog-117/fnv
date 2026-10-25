//! One place for colors, so a theme (the Pip-Boy green is the default) can be
//! swapped later without touching a screen.

use ratatui::style::{Color, Modifier, Style};

const GREEN: Color = Color::Rgb(0x4a, 0xf6, 0x26);
const DIM_GREEN: Color = Color::Rgb(0x1f, 0x8a, 0x10);
const AMBER: Color = Color::Rgb(0xff, 0xb0, 0x00);

/// Ordinary text.
#[must_use]
pub fn text() -> Style {
    Style::new().fg(GREEN)
}

/// Borders and things that are not the point of the screen.
#[must_use]
pub fn dim() -> Style {
    Style::new().fg(DIM_GREEN)
}

/// The selected menu entry.
#[must_use]
pub fn selected() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(GREEN)
        .add_modifier(Modifier::BOLD)
}

/// Titles and the speaker's name.
#[must_use]
pub fn heading() -> Style {
    Style::new().fg(GREEN).add_modifier(Modifier::BOLD)
}

/// Something the player cannot do, or an error.
#[must_use]
pub fn warning() -> Style {
    Style::new().fg(AMBER)
}
