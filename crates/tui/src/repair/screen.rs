//! Having a vendor mend a weapon: the worn weapons the player carries, with what
//! the vendor would leave them at and what it costs.

use std::sync::Arc;

use engine::game::Game;
use engine::shared::{InventoryView, ItemId, VendorId};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{self, Menu};
use crate::shared::names::Names;
use crate::shared::theme;

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// The screen stays open.
    Stay,
    /// The player is done.
    Leave,
}

/// An open repair counter. It holds only the cursor: the weapons, their condition
/// and the quotes are read from the game whenever they are needed.
#[derive(Debug)]
pub struct Repair {
    vendor: VendorId,
    name: String,
    names: Arc<Names>,
    list: Menu,
}

impl Repair {
    /// Opens the counter of `vendor`, called `name`.
    #[must_use]
    pub fn open(vendor: VendorId, name: String, game: &Game, names: Arc<Names>) -> Self {
        let list = Menu::new(worn(game).len());
        Self {
            vendor,
            name,
            names,
            list,
        }
    }

    /// The keys that work right now.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        "Up/Down move | Enter pay and repair | Esc leave | q quit"
    }

    /// Applies a command. The repair goes through the game, so a refusal comes back
    /// in the engine's words.
    pub fn update(&mut self, command: Command, game: &mut Game, log: &mut Log) -> Next {
        if command == Command::Back {
            return Next::Leave;
        }
        let Some(index) = self.list.handle(command) else {
            return Next::Stay;
        };
        let Some(weapon) = worn(game).into_iter().nth(index) else {
            return Next::Stay;
        };
        let name = self.names.name(&weapon);
        let note = match game.repair_at(&self.vendor, &weapon) {
            Ok(quote) => format!(
                "{} mends your {name} to {}% for {} caps.",
                self.name,
                quote.condition.get(),
                quote.price
            ),
            Err(error) => error.to_string(),
        };
        log.push(Entry::Note(note));
        self.list = self.list.resized(worn(game).len());
        Next::Stay
    }

    /// Draws the weapons with their quotes.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let lines: Vec<Line<'static>> = worn(game)
            .iter()
            .enumerate()
            .map(|(index, weapon)| {
                let now = game
                    .weapon_condition(weapon)
                    .map_or(0, |condition| condition.get());
                let text = match game.repair_quote(&self.vendor, weapon) {
                    Ok(quote) => format!(
                        "{} - {now}% to {}% for {} caps",
                        self.names.name(weapon),
                        quote.condition.get(),
                        quote.price
                    ),
                    Err(error) => format!("{} - {now}%: {error}", self.names.name(weapon)),
                };
                let selected = self.list.selected() == Some(index);
                let style = if selected {
                    theme::selected()
                } else {
                    theme::text()
                };
                let marker = if selected { "> " } else { "  " };
                Line::styled(format!("{marker}{text}"), style)
            })
            .collect();
        let height = usize::from(area.height.saturating_sub(2));
        let lines = if lines.is_empty() {
            vec![Line::styled("You carry nothing worn.", theme::dim())]
        } else {
            menu::window(lines, self.list.selected(), height)
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::text())
            .title(format!(" {} - you have {} caps ", self.name, game.caps()));
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }
}

/// The carried weapons that are not as good as new, in the order the pack lists them.
fn worn(game: &Game) -> Vec<ItemId> {
    game.worn_weapons()
        .into_iter()
        .filter(|(_, left)| left.get() < 100)
        .map(|(item, _)| item)
        .collect()
}
