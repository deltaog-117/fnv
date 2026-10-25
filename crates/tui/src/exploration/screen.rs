//! Where the player stands and who they can talk to.

use engine::shared::{DialogueId, NpcId};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::menu::{Item, Menu};
use crate::shared::theme;

/// Someone in the current place with a conversation the player can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// Who they are in the world state.
    pub npc: NpcId,
    /// Display name.
    pub name: String,
    /// The conversation that opens when the player talks to them.
    pub dialogue: DialogueId,
}

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Nothing changed screen.
    Stay,
    /// Talk to the person at this position in [`Exploration::people`].
    Talk(usize),
    /// Leave the game.
    Quit,
}

/// The place and the actions it offers: one "talk to" per person, then quit.
#[derive(Debug)]
pub struct Exploration {
    place: String,
    people: Vec<Person>,
    menu: Menu,
}

impl Exploration {
    /// The scene for `place` with `people` to talk to.
    #[must_use]
    pub fn new(place: String, people: Vec<Person>) -> Self {
        // The extra row is "quit", so the menu is never empty.
        let menu = Menu::new(people.len() + 1);
        Self {
            place,
            people,
            menu,
        }
    }

    /// Applies a command: moves the cursor, or chooses the selected action.
    pub fn update(&mut self, command: Command) -> Next {
        match self.menu.handle(command) {
            Some(index) if index < self.people.len() => Next::Talk(index),
            Some(_) => Next::Quit,
            None => Next::Stay,
        }
    }

    /// Draws the scene and the numbered actions.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<Item> = self
            .people
            .iter()
            .map(|person| Item::new(format!("Talk to {}", person.name)))
            .chain([Item::new("Quit game")])
            .collect();
        let mut lines = vec![
            Line::styled(format!("You are in {}.", self.place), theme::text()),
            Line::default(),
            Line::styled("What do you do?", theme::heading()),
        ];
        lines.extend(self.menu.lines(&items));
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(format!(" {} ", self.place));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str) -> Person {
        let id = name.to_lowercase();
        Person {
            npc: NpcId::try_new(id.clone()).expect("valid id"),
            name: name.to_owned(),
            dialogue: DialogueId::try_new(id).expect("valid id"),
        }
    }

    #[test]
    fn choosing_a_person_asks_to_talk_to_them() {
        let mut scene = Exploration::new(
            "Goodsprings".to_owned(),
            vec![person("Doc"), person("Sunny")],
        );
        assert_eq!(scene.update(Command::Pick(1)), Next::Talk(1));
    }

    #[test]
    fn the_last_row_quits() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")]);
        assert_eq!(scene.update(Command::Pick(1)), Next::Quit);
    }

    #[test]
    fn moving_the_cursor_changes_nothing_until_a_choice() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")]);
        assert_eq!(scene.update(Command::Down), Next::Stay);
        assert_eq!(scene.update(Command::Select), Next::Quit);
    }

    #[test]
    fn a_place_with_nobody_still_offers_quit() {
        let mut scene = Exploration::new("Nowhere".to_owned(), Vec::new());
        assert_eq!(scene.update(Command::Select), Next::Quit);
    }
}
