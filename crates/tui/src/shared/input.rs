//! Keys translated into the few commands every screen understands.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What the player asked for, independent of the key that asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Move the selection up.
    Up,
    /// Move the selection down.
    Down,
    /// Move left: to the previous panel, or lower a value.
    Left,
    /// Move right: to the next panel, or raise a value.
    Right,
    /// Choose the selected entry.
    Select,
    /// Go back, or leave the current screen.
    Back,
    /// Quit the game.
    Quit,
    /// Choose the entry at this zero-based position (keys 1 to 9).
    Pick(usize),
}

/// The command a key press stands for, if any.
///
/// Key releases are ignored: some terminals report both edges of a press and the
/// menu would move twice.
#[must_use]
pub fn command(key: KeyEvent) -> Option<Command> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Command::Quit),
        KeyCode::Up | KeyCode::Char('k') => Some(Command::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Command::Down),
        KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => Some(Command::Left),
        KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => Some(Command::Right),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Command::Select),
        KeyCode::Esc => Some(Command::Back),
        KeyCode::Char('q') => Some(Command::Quit),
        KeyCode::Char(digit @ '1'..='9') => digit
            .to_digit(10)
            .and_then(|n| usize::try_from(n - 1).ok())
            .map(Command::Pick),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn arrows_and_vi_keys_move_the_selection() {
        assert_eq!(command(press(KeyCode::Up)), Some(Command::Up));
        assert_eq!(command(press(KeyCode::Char('j'))), Some(Command::Down));
    }

    #[test]
    fn left_and_right_keys_and_tab_move_sideways() {
        assert_eq!(command(press(KeyCode::Left)), Some(Command::Left));
        assert_eq!(command(press(KeyCode::Char('l'))), Some(Command::Right));
        assert_eq!(command(press(KeyCode::Tab)), Some(Command::Right));
        assert_eq!(command(press(KeyCode::BackTab)), Some(Command::Left));
    }

    #[test]
    fn digits_pick_a_zero_based_entry() {
        assert_eq!(command(press(KeyCode::Char('1'))), Some(Command::Pick(0)));
        assert_eq!(command(press(KeyCode::Char('9'))), Some(Command::Pick(8)));
        assert_eq!(command(press(KeyCode::Char('0'))), None);
    }

    #[test]
    fn control_c_quits_even_though_plain_c_does_nothing() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(command(ctrl_c), Some(Command::Quit));
        assert_eq!(command(press(KeyCode::Char('c'))), None);
    }

    #[test]
    fn a_key_release_is_not_a_command() {
        let mut release = press(KeyCode::Enter);
        release.kind = KeyEventKind::Release;
        assert_eq!(command(release), None);
    }
}
