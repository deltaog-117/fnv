//! Every quest the player has started: the active ones with their current entry,
//! then the completed and the failed. Read from the game on every draw.

use engine::game::Game;
use engine::shared::QuestStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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

/// One quest in the list.
struct Row {
    name: String,
    status: QuestStatus,
    /// The journal entry of the stage the quest is at; only active quests have one.
    entry: Option<String>,
}

impl Row {
    fn label(&self) -> String {
        let tag = match self.status {
            QuestStatus::Active => "active",
            QuestStatus::Completed => "done",
            QuestStatus::Failed => "failed",
            QuestStatus::NotStarted => "not started",
        };
        format!("[{tag}] {}", self.name)
    }
}

/// An open journal. It holds only the cursor.
#[derive(Debug)]
pub struct Journal {
    menu: Menu,
}

/// The rows: active quests, then completed, then failed, each group by name.
fn rows(game: &Game) -> Vec<Row> {
    let log = game.quests();
    let book = log.book();
    let name =
        |quest: &engine::shared::QuestId| book.name(quest).unwrap_or(quest.as_str()).to_owned();
    let mut active: Vec<Row> = log
        .active()
        .map(|(quest, stage)| Row {
            name: name(quest),
            status: QuestStatus::Active,
            entry: stage
                .and_then(|stage| book.stage(quest, stage))
                .map(|stage| stage.description.trim().to_owned())
                .filter(|text| !text.is_empty()),
        })
        .collect();
    active.sort_by(|a, b| a.name.cmp(&b.name));
    let mut ended: Vec<Row> = log
        .finished()
        .map(|(quest, status)| Row {
            name: name(quest),
            status,
            entry: None,
        })
        .collect();
    // Completed before failed, then by name, so the list never shuffles between draws.
    ended.sort_by(|a, b| {
        let rank = |row: &Row| u8::from(row.status == QuestStatus::Failed);
        rank(a).cmp(&rank(b)).then_with(|| a.name.cmp(&b.name))
    });
    active.extend(ended);
    active
}

impl Journal {
    /// Opens the journal over the quests the player has started.
    #[must_use]
    pub fn open(game: &Game) -> Self {
        Self {
            menu: Menu::new(rows(game).len()),
        }
    }

    /// Moves the cursor, or leaves on Back. Nothing here changes the game.
    pub fn update(&mut self, command: Command) -> Next {
        if command == Command::Back {
            return Next::Leave;
        }
        // Choosing a row does nothing: the journal is only read.
        let _ = self.menu.handle(command);
        Next::Stay
    }

    /// Draws the list and the entry of the quest under the cursor.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let rows = rows(game);
        let [list, detail] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(5)]).areas(area);
        let lines = if rows.is_empty() {
            vec![Line::styled(
                "You have not started any quest.",
                theme::dim(),
            )]
        } else {
            let items: Vec<Item> = rows.iter().map(|row| Item::new(row.label())).collect();
            let height = usize::from(list.height.saturating_sub(2));
            menu::window(self.menu.lines(&items), self.menu.selected(), height)
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Journal ");
        frame.render_widget(Paragraph::new(lines).block(block), list);

        let text = self
            .menu
            .selected()
            .and_then(|index| rows.get(index))
            .map_or_else(String::new, |row| match (&row.entry, row.status) {
                (Some(entry), _) => entry.clone(),
                (None, QuestStatus::Completed) => "You finished this quest.".to_owned(),
                (None, QuestStatus::Failed) => "You can no longer finish this quest.".to_owned(),
                (None, _) => "(no journal entry)".to_owned(),
            });
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Entry ");
        frame.render_widget(
            Paragraph::new(Line::styled(text, theme::text()))
                .block(block)
                .wrap(Wrap { trim: true }),
            detail,
        );
    }

    /// The key help for the footer.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        "Up/Down scroll | Esc back | q quit"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use engine::quests::QuestBook;
    use engine::shared::{Effect, GameData, LocationId, QuestId, QuestStage, Scope, apply_all};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn game() -> Game {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
        let place = LocationId::try_new("goodsprings").expect("valid id");
        let data = GameData::load(&dir, &Scope::Around(place)).expect("data loads");
        let items = engine::inventory::ItemBook::from_data(&data).expect("items build");
        Game::new(Arc::new(
            QuestBook::new(data.quests.iter()).expect("quests build"),
        ))
        .with_items(Arc::new(items))
    }

    fn stage(game: &mut Game, quest: &str, number: u32) {
        let effect = Effect::SetStage {
            quest: QuestId::try_new(quest).expect("valid id"),
            stage: QuestStage::new(number),
        };
        apply_all(&[effect], game).expect("the stage is reached");
    }

    fn text(screen: &Journal, game: &Game) -> String {
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
    fn a_new_game_has_an_empty_journal() {
        let game = game();
        let shown = text(&Journal::open(&game), &game);
        assert!(shown.contains("You have not started any quest."), "{shown}");
    }

    #[test]
    fn an_active_quest_shows_its_current_entry() {
        let mut game = game();
        stage(&mut game, "ain_t_that_a_kick_in_the_head", 10);
        let screen = Journal::open(&game);
        let shown = text(&screen, &game);
        assert!(
            shown.contains("[active] Ain't That a Kick in the Head"),
            "{shown}"
        );
        assert!(shown.contains("Entry"), "{shown}");
    }

    #[test]
    fn a_finished_quest_is_listed_after_the_active_ones() {
        let mut game = game();
        stage(&mut game, "ain_t_that_a_kick_in_the_head", 10);
        stage(&mut game, "back_in_the_saddle", 60);
        let shown = text(&Journal::open(&game), &game);
        let active = shown.find("[active]").expect("an active quest is listed");
        let done = shown.find("[done]").expect("a finished quest is listed");
        assert!(active < done, "{shown}");
    }

    #[test]
    fn back_leaves_and_every_other_key_stays() {
        let game = game();
        let mut screen = Journal::open(&game);
        for command in [
            Command::Up,
            Command::Down,
            Command::Select,
            Command::Pick(2),
        ] {
            assert_eq!(screen.update(command), Next::Stay);
        }
        assert_eq!(screen.update(Command::Back), Next::Leave);
    }
}
