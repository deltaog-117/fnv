//! The selection cursor of a list of options.

use ratatui::text::{Line, Span};

use super::input::Command;
use super::theme;

/// One row of a menu as the player sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// What the row says.
    pub label: String,
    /// False for an option that is shown but cannot be chosen yet.
    pub enabled: bool,
}

impl Item {
    /// A row the player can choose.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }

    /// A row that is shown but cannot be chosen yet.
    #[must_use]
    pub fn locked(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: false,
        }
    }
}

/// A cursor over `len` entries.
///
/// The selection is private and every method keeps it inside the list, so a screen
/// cannot select an entry that does not exist; an empty menu selects nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    len: usize,
    selected: usize,
}

impl Menu {
    /// A menu of `len` entries with the first one selected.
    #[must_use]
    pub fn new(len: usize) -> Self {
        Self { len, selected: 0 }
    }

    /// The selected position, or `None` when the menu is empty.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        (self.len > 0).then_some(self.selected)
    }

    /// Moves up one entry, wrapping from the first to the last.
    pub fn up(&mut self) {
        if self.len > 0 {
            self.selected = (self.selected + self.len - 1) % self.len;
        }
    }

    /// Moves down one entry, wrapping from the last to the first.
    pub fn down(&mut self) {
        if self.len > 0 {
            self.selected = (self.selected + 1) % self.len;
        }
    }

    /// Selects `index` and returns it, or `None` (leaving the selection alone) when
    /// there is no such entry.
    pub fn pick(&mut self, index: usize) -> Option<usize> {
        (index < self.len).then(|| {
            self.selected = index;
            index
        })
    }

    /// The rows as lines, numbered 1 to 9 to match the pick keys, with the
    /// selected one highlighted.
    #[must_use]
    pub fn lines(&self, items: &[Item]) -> Vec<Line<'static>> {
        items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let number = if index < 9 {
                    format!("{}. ", index + 1)
                } else {
                    String::new()
                };
                let marker = if self.selected() == Some(index) {
                    "> "
                } else {
                    "  "
                };
                let (text, style) = match (self.selected() == Some(index), item.enabled) {
                    (true, _) => (item.label.clone(), theme::selected()),
                    (false, true) => (item.label.clone(), theme::text()),
                    (false, false) => (item.label.clone(), theme::dim()),
                };
                let suffix = if item.enabled { "" } else { " (locked)" };
                Line::from(vec![
                    Span::styled(format!("{marker}{number}"), style),
                    Span::styled(format!("{text}{suffix}"), style),
                ])
            })
            .collect()
    }

    /// Applies a command and returns the entry it chose, if it chose one.
    /// Commands that are not about the list are left to the caller.
    pub fn handle(&mut self, command: Command) -> Option<usize> {
        match command {
            Command::Up => self.up(),
            Command::Down => self.down(),
            Command::Select => return self.selected(),
            Command::Pick(index) => return self.pick(index),
            Command::Back | Command::Quit => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn any_command() -> impl Strategy<Value = Command> {
        prop_oneof![
            Just(Command::Up),
            Just(Command::Down),
            Just(Command::Select),
            Just(Command::Back),
            Just(Command::Quit),
            (0usize..20).prop_map(Command::Pick),
        ]
    }

    proptest! {
        #[test]
        fn the_selection_never_leaves_the_list(
            len in 0usize..12,
            commands in proptest::collection::vec(any_command(), 0..60),
        ) {
            let mut menu = Menu::new(len);
            for command in commands {
                let chosen = menu.handle(command);
                prop_assert!(chosen.is_none_or(|index| index < len));
                prop_assert!(menu.selected().is_none_or(|index| index < len));
                prop_assert_eq!(menu.selected().is_some(), len > 0);
            }
        }

        #[test]
        fn moving_down_len_times_comes_back_to_the_start(len in 1usize..12, start in 0usize..12) {
            let mut menu = Menu::new(len);
            menu.pick(start % len);
            let before = menu.selected();
            (0..len).for_each(|_| menu.down());
            prop_assert_eq!(menu.selected(), before);
        }
    }

    #[test]
    fn up_from_the_first_entry_wraps_to_the_last() {
        let mut menu = Menu::new(3);
        menu.up();
        assert_eq!(menu.selected(), Some(2));
    }

    #[test]
    fn picking_past_the_end_keeps_the_selection() {
        let mut menu = Menu::new(2);
        menu.down();
        assert_eq!(menu.pick(5), None);
        assert_eq!(menu.selected(), Some(1));
    }
}
