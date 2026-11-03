//! Lists the saves in the folder. Saving offers "New save" (type a name) and every
//! named save to write over; loading offers every save, the automatic one included.
//! The screen holds the list it read when it opened, a cursor and the name being
//! typed; the files themselves are written and read by the app.

use engine::saves::{MAX_NAME_LEN, SaveEntry, SaveFolder, SaveName, SaveSummary, Slot};
use engine::shared::LocationId;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::menu::{self, Item, Menu};
use crate::shared::theme;

/// What the screen is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Writing the game to a slot.
    Save,
    /// Replacing the game with a saved one.
    Load,
}

/// What the player's input led to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The screen stays open.
    Stay,
    /// The player is done.
    Leave,
    /// Write the game under this name.
    Save(SaveName),
    /// Load the game in this slot.
    Load(Slot),
}

/// What the screen is waiting for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    /// Moving through the list.
    Browsing,
    /// Typing the name of a new save.
    Typing(String),
    /// Asking whether to write over a save that exists.
    Overwriting(SaveName),
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    /// "New save": type a name.
    New,
    /// The saved game at this position in the entries.
    Entry(usize),
}

/// An open save or load screen.
#[derive(Debug)]
pub struct Slots {
    purpose: Purpose,
    entries: Vec<SaveEntry>,
    /// Display name of the place of each entry, in the same order.
    places: Vec<String>,
    problem: Option<String>,
    mode: Mode,
    menu: Menu,
}

impl Slots {
    /// Opens the screen over what the folder holds now. `place_name` turns a place
    /// id into the name the player knows.
    #[must_use]
    pub fn open(
        purpose: Purpose,
        folder: &SaveFolder,
        place_name: impl Fn(&LocationId) -> String,
    ) -> Self {
        let (entries, problem) = match folder.list() {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(error.to_string())),
        };
        let places = entries
            .iter()
            .map(|entry| {
                entry
                    .summary
                    .as_ref()
                    .ok()
                    .and_then(|summary| summary.place.as_ref())
                    .map_or_else(String::new, &place_name)
            })
            .collect();
        let mut slots = Self {
            purpose,
            entries,
            places,
            problem,
            mode: Mode::Browsing,
            menu: Menu::new(0),
        };
        slots.menu = Menu::new(slots.rows().len());
        slots
    }

    /// Whether keys are being typed into a name, so they must not be read as
    /// commands.
    #[must_use]
    pub fn is_typing(&self) -> bool {
        matches!(self.mode, Mode::Typing(_))
    }

    /// The keys that work right now.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        match self.mode {
            Mode::Browsing => "Up/Down move | Enter choose | Esc back | q quit",
            Mode::Typing(_) => "Type a name | Backspace erase | Enter save | Esc cancel",
            Mode::Overwriting(_) => "Enter write over it | Esc keep it",
        }
    }

    fn rows(&self) -> Vec<Row> {
        let entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| self.purpose == Purpose::Load || entry.slot != Slot::Auto)
            .map(|(index, _)| Row::Entry(index));
        match self.purpose {
            Purpose::Save => std::iter::once(Row::New).chain(entries).collect(),
            Purpose::Load => entries.collect(),
        }
    }

    /// Applies a command while browsing or answering the overwrite question.
    pub fn update(&mut self, command: Command) -> Next {
        match &self.mode {
            Mode::Browsing => {
                if command == Command::Back {
                    return Next::Leave;
                }
                let Some(index) = self.menu.handle(command) else {
                    return Next::Stay;
                };
                match self.rows().get(index).cloned() {
                    Some(Row::New) => {
                        self.problem = None;
                        self.mode = Mode::Typing(String::new());
                        Next::Stay
                    }
                    Some(Row::Entry(entry)) => self.choose(entry),
                    None => Next::Stay,
                }
            }
            Mode::Overwriting(name) => match command {
                Command::Select => {
                    let name = name.clone();
                    self.mode = Mode::Browsing;
                    Next::Save(name)
                }
                Command::Back => {
                    self.mode = Mode::Browsing;
                    Next::Stay
                }
                _ => Next::Stay,
            },
            // Typing is fed by `type_key`; a command that arrives anyway does nothing.
            Mode::Typing(_) => Next::Stay,
        }
    }

    fn choose(&mut self, entry: usize) -> Next {
        let Some(entry) = self.entries.get(entry) else {
            return Next::Stay;
        };
        match (self.purpose, &entry.slot) {
            (Purpose::Load, slot) => Next::Load(slot.clone()),
            (Purpose::Save, Slot::Named(name)) => {
                self.mode = Mode::Overwriting(name.clone());
                Next::Stay
            }
            // The automatic save is the game's own; it is never a place to save to.
            (Purpose::Save, Slot::Auto) => Next::Stay,
        }
    }

    /// Applies a key typed into a name.
    pub fn type_key(&mut self, key: KeyCode) -> Next {
        let Mode::Typing(text) = &mut self.mode else {
            return Next::Stay;
        };
        match key {
            KeyCode::Char(c) if SaveName::allows(c) => {
                if text.chars().count() < MAX_NAME_LEN {
                    text.push(c);
                    self.problem = None;
                }
            }
            KeyCode::Backspace => {
                text.pop();
                self.problem = None;
            }
            KeyCode::Esc => {
                self.mode = Mode::Browsing;
                self.problem = None;
            }
            KeyCode::Enter => match SaveName::try_new(text) {
                Err(error) => self.problem = Some(error.to_string()),
                Ok(name) => {
                    let taken = self
                        .entries
                        .iter()
                        .any(|entry| entry.slot == Slot::Named(name.clone()));
                    if taken {
                        self.mode = Mode::Overwriting(name);
                    } else {
                        self.mode = Mode::Browsing;
                        return Next::Save(name);
                    }
                }
            },
            _ => {}
        }
        Next::Stay
    }

    /// Draws the list and the line under it.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let [list, prompt] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);
        let rows = self.rows();
        let items: Vec<Item> = rows
            .iter()
            .map(|row| match row {
                Row::New => Item::new("New save..."),
                Row::Entry(index) => Item::new(self.describe(*index)),
            })
            .collect();
        let lines = self.menu.lines(&items);
        let height = usize::from(list.height.saturating_sub(2));
        let lines = if lines.is_empty() {
            vec![Line::styled("There are no saves yet.", theme::dim())]
        } else {
            menu::window(lines, self.menu.selected(), height)
        };
        let title = match self.purpose {
            Purpose::Save => " Save game ",
            Purpose::Load => " Load game ",
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::text())
            .title(title);
        frame.render_widget(Paragraph::new(lines).block(block), list);

        let (text, style) = match (&self.mode, &self.problem) {
            (_, Some(problem)) => (problem.clone(), theme::warning()),
            (Mode::Typing(name), None) => (format!("Name: {name}_"), theme::heading()),
            (Mode::Overwriting(name), None) => (
                format!("Write over \"{name}\"? Enter yes, Esc no."),
                theme::heading(),
            ),
            (Mode::Browsing, None) => (
                match self.purpose {
                    Purpose::Save => "Choose \"New save...\" or a save to write over.".to_owned(),
                    Purpose::Load => "Choose a save to load.".to_owned(),
                },
                theme::dim(),
            ),
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Saves ");
        frame.render_widget(
            Paragraph::new(Line::styled(text, style))
                .block(block)
                .wrap(Wrap { trim: false }),
            prompt,
        );
    }

    /// One entry as a row: its name, then when and where it was left.
    fn describe(&self, index: usize) -> String {
        let Some(entry) = self.entries.get(index) else {
            return String::new();
        };
        let name = match entry.slot {
            Slot::Named(_) => entry.slot.label(),
            Slot::Auto => "Autosave (automatic)".to_owned(),
        };
        match &entry.summary {
            Ok(summary) => format!("{name} - {}", when_and_where(summary, &self.places[index])),
            Err(reason) => format!("{name} - cannot be read: {reason}"),
        }
    }
}

/// "Day 3, 14:05, Goodsprings, 1200 xp".
fn when_and_where(summary: &SaveSummary, place: &str) -> String {
    let place = if place.is_empty() {
        String::new()
    } else {
        format!(", {place}")
    };
    format!(
        "Day {}, {:02}:{:02}{place}, {} xp",
        summary.day, summary.hour, summary.minute, summary.xp
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use engine::saves::Save;
    use proptest::prelude::*;

    use super::*;

    fn folder_with(names: &[&str], tag: &str) -> (SaveFolder, PathBuf) {
        let dir = std::env::temp_dir().join(format!("fnv-slots-{tag}-{}", std::process::id()));
        let folder = SaveFolder::new(&dir);
        let save = blank_save();
        for name in names {
            let slot = if *name == "autosave" {
                Slot::Auto
            } else {
                Slot::Named(SaveName::try_new(name).expect("valid"))
            };
            folder.write(&slot, &save).expect("written");
        }
        (folder, dir)
    }

    /// The save of a new game with nothing in it.
    fn blank_save() -> Save {
        let quests = engine::quests::QuestBook::new(std::iter::empty::<&engine::shared::Quest>())
            .expect("no quests is a valid book");
        engine::game::Game::new(std::sync::Arc::new(quests)).save()
    }

    fn open(purpose: Purpose, folder: &SaveFolder) -> Slots {
        Slots::open(purpose, folder, |place| place.as_str().to_owned())
    }

    #[test]
    fn saving_lists_new_then_the_named_saves_and_never_the_autosave() {
        let (folder, dir) = folder_with(&["one", "two", "autosave"], "save");
        let slots = open(Purpose::Save, &folder);
        let rows = slots.rows();
        assert_eq!(rows.len(), 3, "new, one, two");
        assert_eq!(rows[0], Row::New);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn loading_lists_every_save_including_the_autosave() {
        let (folder, dir) = folder_with(&["one", "autosave"], "load");
        let slots = open(Purpose::Load, &folder);
        assert_eq!(slots.rows().len(), 2);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn choosing_a_save_to_load_names_its_slot() {
        let (folder, dir) = folder_with(&["one"], "choose");
        let mut slots = open(Purpose::Load, &folder);
        assert_eq!(
            slots.update(Command::Select),
            Next::Load(Slot::Named(SaveName::try_new("one").expect("valid")))
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_new_name_is_returned_and_an_existing_one_asks_first() {
        let (folder, dir) = folder_with(&["one"], "name");
        let mut slots = open(Purpose::Save, &folder);
        slots.update(Command::Select);
        assert!(slots.is_typing());
        for c in "one".chars() {
            slots.type_key(KeyCode::Char(c));
        }
        assert_eq!(slots.type_key(KeyCode::Enter), Next::Stay);
        assert!(!slots.is_typing());
        assert_eq!(
            slots.mode,
            Mode::Overwriting(SaveName::try_new("one").expect("valid"))
        );
        assert_eq!(slots.update(Command::Back), Next::Stay);
        slots.update(Command::Select);
        for c in "fresh".chars() {
            slots.type_key(KeyCode::Char(c));
        }
        assert_eq!(
            slots.type_key(KeyCode::Enter),
            Next::Save(SaveName::try_new("fresh").expect("valid"))
        );
        std::fs::remove_dir_all(dir).ok();
    }

    proptest! {
        /// Whatever is typed, the name stays within its limit, only allowed
        /// characters get in, and a name that comes out is a valid one.
        #[test]
        fn typing_never_builds_a_name_the_engine_would_refuse(
            keys in proptest::collection::vec(
                prop_oneof![
                    any::<char>().prop_map(KeyCode::Char),
                    Just(KeyCode::Backspace),
                    Just(KeyCode::Enter),
                    Just(KeyCode::Esc),
                    Just(KeyCode::Left),
                ],
                0..120,
            )
        ) {
            let folder = SaveFolder::new(std::env::temp_dir().join("fnv-slots-prop-never-made"));
            let mut slots = open(Purpose::Save, &folder);
            slots.update(Command::Select);
            for key in keys {
                if let Next::Save(name) = slots.type_key(key) {
                    prop_assert!(SaveName::try_new(name.as_str()).is_ok());
                    slots.update(Command::Select);
                }
                if let Mode::Typing(text) = &slots.mode {
                    prop_assert!(text.chars().count() <= MAX_NAME_LEN);
                    prop_assert!(text.chars().all(SaveName::allows));
                }
                if !slots.is_typing() && slots.mode == Mode::Browsing {
                    slots.update(Command::Select);
                }
            }
        }
    }
}
