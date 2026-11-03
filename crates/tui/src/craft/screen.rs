//! Making things at a station: the recipes on the left, what the chosen one needs
//! on the right.

use std::sync::Arc;

use engine::craft::Formula;
use engine::game::Game;
use engine::shared::{InventoryView, RecipeId, StationId};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

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
    /// The player is done crafting.
    Leave,
}

/// An open station. It holds only the cursor: the recipes, the pack and the reasons
/// a recipe is out of reach are read from the game whenever they are needed, so the
/// screen cannot show a number the engine has since changed.
#[derive(Debug)]
pub struct Craft {
    station: StationId,
    name: String,
    names: Arc<Names>,
    list: Menu,
}

impl Craft {
    /// Opens the station `station`, called `name`.
    #[must_use]
    pub fn open(station: StationId, name: String, game: &Game, names: Arc<Names>) -> Self {
        let list = Menu::new(game.recipes_at(&station).len());
        Self {
            station,
            name,
            names,
            list,
        }
    }

    /// The keys that work right now.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        "Up/Down move | Enter make one | Esc leave | q quit"
    }

    /// Applies a command. Making a recipe goes through the game, so a refusal comes
    /// back in the engine's words.
    pub fn update(&mut self, command: Command, game: &mut Game, log: &mut Log) -> Next {
        if command == Command::Back {
            return Next::Leave;
        }
        let Some(index) = self.list.handle(command) else {
            return Next::Stay;
        };
        let Some(recipe) = self.ordered(game).get(index).map(|(id, _)| (*id).clone()) else {
            return Next::Stay;
        };
        let note = match game.craft(&self.station, &recipe) {
            Ok(made) => format!("You make {} x{}.", self.names.name(&made.item), made.count),
            Err(error) => error.to_string(),
        };
        log.push(Entry::Note(note));
        Next::Stay
    }

    /// The station's recipes in the order the list shows them: the ones the player
    /// can make now first, then the rest, each group by the name of what it makes.
    fn ordered<'a>(&self, game: &'a Game) -> Vec<(&'a RecipeId, &'a Formula)> {
        let mut recipes = game.recipes_at(&self.station);
        recipes.sort_by_cached_key(|(id, formula)| {
            (
                game.can_craft(&self.station, id).is_err(),
                self.names.name(formula.output().0),
            )
        });
        recipes
    }

    /// Draws the recipes and the chosen recipe's needs.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);
        self.render_recipes(frame, left, game);
        self.render_needs(frame, right, game);
    }

    fn render_recipes(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let height = usize::from(area.height.saturating_sub(2));
        let lines: Vec<Line<'static>> = self
            .ordered(game)
            .iter()
            .enumerate()
            .map(|(index, (id, formula))| {
                let (output, count) = formula.output();
                let label = if count.get() > 1 {
                    format!("{} x{count}", self.names.name(output))
                } else {
                    self.names.name(output)
                };
                let tag = if game.can_craft(&self.station, id).is_ok() {
                    "[ready]"
                } else {
                    ""
                };
                row(&label, tag, usize::from(area.width.saturating_sub(2)), {
                    self.list.selected() == Some(index)
                })
            })
            .collect();
        let lines = if lines.is_empty() {
            vec![Line::styled("Nothing can be made here.", theme::dim())]
        } else {
            menu::window(lines, self.list.selected(), height)
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::text())
            .title(format!(" {} ", capitalize(&self.name)));
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_needs(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let chosen: Option<(RecipeId, _)> = self
            .list
            .selected()
            .and_then(|index| self.ordered(game).into_iter().nth(index))
            .map(|(id, formula)| (id.clone(), formula));
        let mut lines = Vec::new();
        if let Some((id, formula)) = chosen {
            let (output, count) = formula.output();
            lines.push(Line::styled(
                format!("{} x{count}", self.names.name(output)),
                theme::heading(),
            ));
            if let Some((skill, level)) = formula.skill() {
                lines.push(Line::styled(
                    format!("Needs {} {}.", skill.name(), level.get()),
                    theme::text(),
                ));
            }
            lines.push(Line::default());
            lines.push(Line::styled("Uses up:", theme::heading()));
            for (item, need) in formula.inputs() {
                let have = game.inventory().item_count(item);
                let style = if have >= need.get() {
                    theme::text()
                } else {
                    theme::warning()
                };
                lines.push(Line::styled(
                    format!("{} - {have} of {need}", self.names.name(item)),
                    style,
                ));
            }
            if let Err(reason) = game.can_craft(&self.station, &id) {
                lines.push(Line::default());
                lines.push(Line::styled(reason.to_string(), theme::warning()));
            }
        }
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" What it needs ");
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

/// One row: the label on the left and a tag on the right, cut to fit `width`.
fn row(label: &str, tag: &str, width: usize, selected: bool) -> Line<'static> {
    let marker = if selected { "> " } else { "  " };
    let tag_len = tag.chars().count();
    let room = width.saturating_sub(marker.len() + tag_len + 1);
    let label: String = label.chars().take(room).collect();
    let gap = width.saturating_sub(marker.len() + label.chars().count() + tag_len);
    let style = if selected {
        theme::selected()
    } else {
        theme::text()
    };
    Line::styled(format!("{marker}{label}{}{tag}", " ".repeat(gap)), style)
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}
