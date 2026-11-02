//! Where the player stands and who they can talk to.

use std::num::NonZeroU32;

use engine::clock::Span;
use engine::shared::{DialogueId, NpcId};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::menu::{self, Item, Menu};
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

/// The ways to pass time from the scene: the label and the hours.
const WAITS: [(&str, u32); 2] = [("Wait an hour", 1), ("Wait 8 hours", 8)];

/// The time a wait row passes.
fn wait_span(hours: u32) -> Option<Span> {
    NonZeroU32::new(hours).map(Span::hours)
}

/// The rows that are always there: "character", "pack", the journal, reputation,
/// travel, the waits and "quit".
const EXTRA_ROWS: usize = 8 + WAITS.len();

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Nothing changed screen.
    Stay,
    /// Talk to the person at this position in [`Exploration::people`].
    Talk(usize),
    /// Open the shop at this position in the offered shops.
    Shop(usize),
    /// Start the fight at this position in the offered fights.
    Fight(usize),
    /// Sleep in the bed at this position in the offered beds.
    Sleep(usize),
    /// Open the character screen.
    Character,
    /// Open the pack screen.
    Pack,
    /// Open the quest journal.
    Journal,
    /// Open the reputation screen.
    Reputation,
    /// Open the travel screen.
    Travel,
    /// Let this much time pass, and say what it was in `label`.
    Wait {
        /// How long.
        span: Span,
        /// What the row said, such as "Wait an hour".
        label: &'static str,
    },
    /// Write the game to the save file.
    Save,
    /// Replace the game with the one in the save file.
    Load,
    /// Leave the game.
    Quit,
}

/// The place and the actions it offers: one "talk to" per person, one per shop, one
/// per fight on offer, one per bed, the character screen, the pack, the journal,
/// reputation, travel, the waits, save, load, then quit.
#[derive(Debug)]
pub struct Exploration {
    place: String,
    people: Vec<Person>,
    shops: Vec<String>,
    fights: Vec<String>,
    beds: Vec<String>,
    character: String,
    menu: Menu,
}

impl Exploration {
    /// The scene for `place` with `people` to talk to.
    #[must_use]
    pub fn new(place: String, people: Vec<Person>) -> Self {
        // The extra rows are "character", "pack", the journal, reputation, travel, the
        // waits and "quit", so the menu is never empty.
        let menu = Menu::new(people.len() + EXTRA_ROWS);
        Self {
            place,
            people,
            shops: Vec::new(),
            fights: Vec::new(),
            beds: Vec::new(),
            character: "Character".to_owned(),
            menu,
        }
    }

    /// The same scene with shops to offer, by name, after the people.
    #[must_use]
    pub fn with_shops(mut self, shops: Vec<String>) -> Self {
        self.shops = shops;
        self.menu = Menu::new(self.rows());
        self
    }

    /// The same scene with fights to offer, by name, after the shops.
    #[must_use]
    pub fn with_fights(mut self, fights: Vec<String>) -> Self {
        self.fights = fights;
        self.menu = Menu::new(self.rows());
        self
    }

    /// The same scene with beds to offer, by name, after the fights.
    #[must_use]
    pub fn with_beds(mut self, beds: Vec<String>) -> Self {
        self.beds = beds;
        self.menu = Menu::new(self.rows());
        self
    }

    /// Everything the menu offers: the listed actions plus "character", "pack", the
    /// journal, reputation, travel, the waits and "quit".
    fn rows(&self) -> usize {
        self.people.len() + self.shops.len() + self.fights.len() + self.beds.len() + EXTRA_ROWS
    }

    /// The same scene with another label on the character row, so it can say that
    /// points are waiting.
    #[must_use]
    pub fn with_character_label(mut self, label: String) -> Self {
        self.character = label;
        self
    }

    /// Applies a command: moves the cursor, or chooses the selected action.
    pub fn update(&mut self, command: Command) -> Next {
        let shops_end = self.people.len() + self.shops.len();
        let fights_end = shops_end + self.fights.len();
        let beds_end = fights_end + self.beds.len();
        match self.menu.handle(command) {
            Some(index) if index < self.people.len() => Next::Talk(index),
            Some(index) if index < shops_end => Next::Shop(index - self.people.len()),
            Some(index) if index < fights_end => Next::Fight(index - shops_end),
            Some(index) if index < beds_end => Next::Sleep(index - fights_end),
            Some(index) if index == beds_end => Next::Character,
            Some(index) if index == beds_end + 1 => Next::Pack,
            Some(index) if index == beds_end + 2 => Next::Journal,
            Some(index) if index == beds_end + 3 => Next::Reputation,
            Some(index) if index == beds_end + 4 => Next::Travel,
            Some(index) if index + 1 == self.rows() => Next::Quit,
            Some(index) if index + 2 == self.rows() => Next::Load,
            Some(index) if index + 3 == self.rows() => Next::Save,
            // A row no arm names cannot be reached; staying is the harmless answer.
            Some(index) => index
                .checked_sub(beds_end + 5)
                .and_then(|n| WAITS.get(n))
                .and_then(|&(label, hours)| wait_span(hours).map(|span| Next::Wait { span, label }))
                .unwrap_or(Next::Stay),
            None => Next::Stay,
        }
    }

    /// Draws the scene and the numbered actions.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<Item> = self
            .people
            .iter()
            .map(|person| Item::new(format!("Talk to {}", person.name)))
            .chain(
                self.shops
                    .iter()
                    .map(|name| Item::new(format!("Trade at {name}"))),
            )
            .chain(self.fights.iter().map(|name| Item::new(name.clone())))
            .chain(
                self.beds
                    .iter()
                    .map(|name| Item::new(format!("Sleep in {name}"))),
            )
            .chain([
                Item::new(self.character.clone()),
                Item::new("Pack"),
                Item::new("Journal"),
                Item::new("Reputation"),
                Item::new("Travel"),
            ])
            .chain(WAITS.iter().map(|&(label, _)| Item::new(label)))
            .chain([
                Item::new("Save game"),
                Item::new("Load game"),
                Item::new("Quit game"),
            ])
            .collect();
        let mut lines = vec![
            Line::styled(format!("You are in {}.", self.place), theme::text()),
            Line::default(),
            Line::styled("What do you do?", theme::heading()),
        ];
        // The town can list more actions than the scene is tall, so the rows scroll
        // with the cursor instead of hiding whatever is below the last visible one.
        let menu_height = usize::from(area.height.saturating_sub(2)).saturating_sub(lines.len());
        lines.extend(menu::window(
            self.menu.lines(&items),
            self.menu.selected(),
            menu_height,
        ));
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
    fn the_journal_reputation_and_travel_follow_the_pack_then_the_waits_then_quit() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")]);
        assert_eq!(scene.update(Command::Pick(2)), Next::Pack);
        assert_eq!(scene.update(Command::Pick(3)), Next::Journal);
        assert_eq!(scene.update(Command::Pick(4)), Next::Reputation);
        assert_eq!(scene.update(Command::Pick(5)), Next::Travel);
        assert_eq!(
            scene.update(Command::Pick(6)),
            Next::Wait {
                span: Span::hours(NonZeroU32::MIN),
                label: "Wait an hour"
            }
        );
        assert!(matches!(
            scene.update(Command::Pick(7)),
            Next::Wait {
                label: "Wait 8 hours",
                ..
            }
        ));
        assert_eq!(scene.update(Command::Pick(8)), Next::Save);
        assert_eq!(scene.update(Command::Pick(9)), Next::Load);
        assert_eq!(scene.update(Command::Pick(10)), Next::Quit);
    }

    #[test]
    fn every_wait_passes_at_least_an_hour() {
        for (label, hours) in WAITS {
            assert!(wait_span(hours).is_some(), "{label} passes no time");
        }
    }

    #[test]
    fn moving_the_cursor_changes_nothing_until_a_choice() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")]);
        assert_eq!(scene.update(Command::Down), Next::Stay);
        assert_eq!(scene.update(Command::Select), Next::Character);
    }

    #[test]
    fn fights_come_after_the_people_and_before_quit() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")])
            .with_fights(vec!["Fight the gang".to_owned()]);
        assert_eq!(scene.update(Command::Pick(1)), Next::Fight(0));
        assert_eq!(scene.update(Command::Pick(2)), Next::Character);
        assert_eq!(scene.update(Command::Pick(3)), Next::Pack);
        assert_eq!(scene.update(Command::Pick(11)), Next::Quit);
    }

    #[test]
    fn shops_come_after_the_people_and_before_the_fights() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")])
            .with_shops(vec!["Chet's store".to_owned()])
            .with_fights(vec!["Fight the gang".to_owned()]);
        assert_eq!(scene.update(Command::Pick(1)), Next::Shop(0));
        assert_eq!(scene.update(Command::Pick(2)), Next::Fight(0));
        assert_eq!(scene.update(Command::Pick(3)), Next::Character);
        assert_eq!(scene.update(Command::Pick(4)), Next::Pack);
        assert_eq!(scene.update(Command::Pick(12)), Next::Quit);
    }

    #[test]
    fn beds_come_after_the_fights_and_before_the_character_row() {
        let mut scene = Exploration::new("Goodsprings".to_owned(), vec![person("Doc")])
            .with_fights(vec!["Fight the gang".to_owned()])
            .with_beds(vec!["a cot".to_owned()]);
        assert_eq!(scene.update(Command::Pick(1)), Next::Fight(0));
        assert_eq!(scene.update(Command::Pick(2)), Next::Sleep(0));
        assert_eq!(scene.update(Command::Pick(3)), Next::Character);
        assert_eq!(scene.update(Command::Pick(4)), Next::Pack);
        assert!(matches!(
            scene.update(Command::Pick(8)),
            Next::Wait {
                label: "Wait an hour",
                ..
            }
        ));
        assert_eq!(scene.update(Command::Pick(12)), Next::Quit);
    }

    #[test]
    fn a_place_with_nobody_still_offers_the_character_screen_the_pack_and_quit() {
        let mut scene = Exploration::new("Nowhere".to_owned(), Vec::new());
        assert_eq!(scene.update(Command::Select), Next::Character);
        assert_eq!(scene.update(Command::Pick(1)), Next::Pack);
        assert_eq!(scene.update(Command::Pick(9)), Next::Quit);
    }
}
