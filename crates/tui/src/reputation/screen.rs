//! Reads the standing from the game on every draw, so the screen cannot show a
//! number the engine has since changed. It holds only a cursor for scrolling.

use engine::game::Game;
use engine::shared::{Alignment, Renown, StandingView};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::shared::input::Command;
use crate::shared::menu::{self, Item, Menu};
use crate::shared::theme;

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// The screen stays open.
    Stay,
    /// The player is done reading.
    Leave,
}

/// An open reputation screen.
#[derive(Debug)]
pub struct Reputation {
    menu: Menu,
}

/// The name karma goes by on screen.
fn alignment_name(alignment: Alignment) -> &'static str {
    match alignment {
        Alignment::Good => "good",
        Alignment::Neutral => "neutral",
        Alignment::Evil => "evil",
    }
}

impl Reputation {
    /// Opens the screen over the factions the game tracks.
    #[must_use]
    pub fn open(game: &Game) -> Self {
        Self {
            menu: Menu::new(game.standing().factions().count()),
        }
    }

    /// Moves the cursor, or leaves on Back. Nothing here changes the game.
    pub fn update(&mut self, command: Command) -> Next {
        if command == Command::Back {
            return Next::Leave;
        }
        // Choosing a row does nothing: the list is only read.
        let _ = self.menu.handle(command);
        Next::Stay
    }

    /// The row for each faction, by name: its title, then fame and infamy points.
    fn rows(game: &Game) -> Vec<String> {
        let standing = game.standing();
        let mut rows: Vec<(&str, String)> = standing
            .factions()
            .map(|(id, name)| {
                let title = standing.title(id).unwrap_or("Unknown");
                let fame = standing.renown_points(id, Renown::Fame);
                let infamy = standing.renown_points(id, Renown::Infamy);
                (
                    name,
                    format!("{name}: {title} (fame {fame:.0}, infamy {infamy:.0})"),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(b.0));
        rows.into_iter().map(|(_, row)| row).collect()
    }

    /// Draws karma, then the factions.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let karma = game.standing().karma();
        let items: Vec<Item> = Self::rows(game).into_iter().map(Item::new).collect();
        let mut lines = vec![
            Line::styled(
                format!(
                    "Karma: {:.0} ({})",
                    karma.get(),
                    alignment_name(karma.alignment())
                ),
                theme::heading(),
            ),
            Line::default(),
        ];
        let height = usize::from(area.height.saturating_sub(2)).saturating_sub(lines.len());
        lines.extend(menu::window(
            self.menu.lines(&items),
            self.menu.selected(),
            height,
        ));
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Reputation ");
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    /// The key help for the footer.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        "Up/Down scroll | Esc back | q quit"
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use engine::quests::QuestBook;
    use engine::reputation::ReputationBook;
    use engine::shared::{Effect, FactionId, GameData, LocationId, Scope, apply};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn game() -> Game {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
        let place = LocationId::try_new("goodsprings").expect("valid id");
        let data = GameData::load(&dir, &Scope::Around(place)).expect("data loads");
        let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("quests build"));
        let book = ReputationBook::from_data(&data).expect("reputation builds");
        Game::new(quests).with_reputation(Arc::new(book))
    }

    fn text(screen: &Reputation, game: &Game) -> String {
        let mut terminal = Terminal::new(TestBackend::new(90, 24)).expect("a test terminal");
        terminal
            .draw(|frame| screen.render(frame, frame.area(), game))
            .expect("the frame draws");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn it_shows_karma_and_the_goodsprings_title() {
        let game = game();
        let shown = text(&Reputation::open(&game), &game);
        for expected in ["Karma: 0 (neutral)", "Goodsprings: Neutral", "fame 0"] {
            assert!(shown.contains(expected), "missing {expected:?} in {shown}");
        }
    }

    #[test]
    fn a_standing_earned_in_the_game_shows_at_once() {
        let mut game = game();
        let screen = Reputation::open(&game);
        let level = Effect::SetReputation {
            faction: FactionId::try_new("goodsprings").expect("valid id"),
            level: engine::shared::ReputationLevel::Accepted,
        };
        apply(&level, &mut game).expect("the town tracks reputation");
        let shown = text(&screen, &game);
        assert!(shown.contains("Goodsprings: Accepted"), "{shown}");
    }

    #[test]
    fn back_leaves_and_every_other_key_stays() {
        let game = game();
        let mut screen = Reputation::open(&game);
        for command in [
            Command::Up,
            Command::Down,
            Command::Select,
            Command::Pick(3),
            Command::Left,
        ] {
            assert_eq!(screen.update(command), Next::Stay);
        }
        assert_eq!(screen.update(Command::Back), Next::Leave);
    }
}
