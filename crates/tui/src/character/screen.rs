//! Spending the skill points and perk picks a level-up pays out.

use engine::game::Game;
use engine::shared::{CharacterView, PerkId, Skill};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{self, Item, Menu};
use crate::shared::theme;

/// Rows of the top menu, in order.
const TOP_SKILLS: usize = 0;
const TOP_PERKS: usize = 1;

/// Rows kept for the perk description under the list.
const DETAIL_HEIGHT: u16 = 4;

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// The screen stays open.
    Stay,
    /// The player is done and goes back to the scene.
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Top,
    Skills,
    Perks,
}

/// The character screen. It holds only the cursor: every number it shows is read
/// from the game when it is drawn, so it cannot show a stale one.
#[derive(Debug)]
pub struct LevelUp {
    view: View,
    menu: Menu,
}

impl LevelUp {
    /// The top menu: skills, perks, done.
    #[must_use]
    pub fn new() -> Self {
        Self {
            view: View::Top,
            menu: Menu::new(3),
        }
    }

    /// Applies a command. Esc steps back one view and then leaves; choosing a skill
    /// or a perk asks the game, so a refusal comes back in the engine's own words.
    pub fn update(&mut self, command: Command, game: &mut Game, log: &mut Log) -> Next {
        if command == Command::Back {
            return self.back();
        }
        let Some(index) = self.menu.handle(command) else {
            return Next::Stay;
        };
        match self.view {
            View::Top => match index {
                TOP_SKILLS => self.open(View::Skills, game.character().skills().count()),
                TOP_PERKS => self.open(View::Perks, open_perks(game).len()),
                _ => return Next::Leave,
            },
            View::Skills => {
                let skill = game.character().skills().nth(index).map(|(skill, _)| skill);
                if let Some(skill) = skill {
                    spend(game, log, skill);
                }
            }
            View::Perks => {
                if let Some(id) = open_perks(game).into_iter().nth(index).map(|perk| perk.0) {
                    self.take(game, log, &id);
                }
            }
        }
        Next::Stay
    }

    fn back(&mut self) -> Next {
        match self.view {
            View::Top => Next::Leave,
            View::Skills | View::Perks => {
                *self = Self::new();
                Next::Stay
            }
        }
    }

    fn open(&mut self, view: View, rows: usize) {
        self.view = view;
        self.menu = Menu::new(rows);
    }

    fn take(&mut self, game: &mut Game, log: &mut Log, id: &PerkId) {
        let name = game
            .perks()
            .perk(id)
            .map_or_else(|| id.to_string(), |perk| perk.name.clone());
        match game.choose_perk(id) {
            Ok(()) => {
                log.push(Entry::Note(format!("You take {name}.")));
                // The list shrinks when a perk is fully taken, and the cursor
                // must not point past its end.
                self.open(View::Perks, open_perks(game).len());
            }
            Err(error) => log.push(Entry::Note(error.to_string())),
        }
    }

    /// Draws the counts waiting, the active list and, for perks, what the selected
    /// one does.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Character ");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let detail = if self.view == View::Perks {
            DETAIL_HEIGHT
        } else {
            0
        };
        let [head, list, foot] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(detail),
        ])
        .areas(inner);

        let character = game.character();
        let title = match self.view {
            View::Top => "What do you do?",
            View::Skills => "Spend a skill point on:",
            View::Perks => "Take a perk:",
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(
                    format!(
                        "Level {} | {} skill point(s) | {} perk pick(s)",
                        character.level().get(),
                        character.skill_points(),
                        character.perk_picks(),
                    ),
                    theme::text(),
                ),
                Line::default(),
                Line::styled(title, theme::heading()),
            ]),
            head,
        );

        let items = self.items(game);
        let lines = menu::window(
            self.menu.lines(&items),
            self.menu.selected(),
            usize::from(list.height),
        );
        frame.render_widget(Paragraph::new(lines), list);

        if self.view == View::Perks {
            frame.render_widget(
                Paragraph::new(self.perk_detail(game)).wrap(Wrap { trim: false }),
                foot,
            );
        }
    }

    fn items(&self, game: &Game) -> Vec<Item> {
        let character = game.character();
        match self.view {
            View::Top => vec![
                Item::new("Skills"),
                Item::new("Perks"),
                Item::new("Back to the scene"),
            ],
            View::Skills => character
                .skills()
                .map(|(skill, level)| {
                    let tag = if character.is_tagged(skill) {
                        " (tagged)"
                    } else {
                        ""
                    };
                    let label = format!("{:<15} {:>3}{tag}", skill.name(), level.get());
                    if character.skill_points() > 0 {
                        Item::new(label)
                    } else {
                        Item::locked(label)
                    }
                })
                .collect(),
            View::Perks => open_perks(game)
                .into_iter()
                .map(|(_, label)| {
                    if character.perk_picks() > 0 {
                        Item::new(label)
                    } else {
                        Item::locked(label)
                    }
                })
                .collect(),
        }
    }

    /// What the selected perk is, and whether the game models its effect yet.
    fn perk_detail(&self, game: &Game) -> Vec<Line<'static>> {
        let Some(perk) = self
            .menu
            .selected()
            .and_then(|index| open_perks(game).into_iter().nth(index))
            .and_then(|(id, _)| game.perks().perk(&id))
        else {
            return vec![Line::styled("No perk is open to you yet.", theme::dim())];
        };
        let text = perk
            .effects_text
            .as_deref()
            .or(perk.description.as_deref())
            .unwrap_or("No description.");
        // The warning goes first: a long description would push it out of the pane.
        let mut lines = Vec::new();
        if perk.modifiers.is_empty() {
            lines.push(Line::styled(
                "Not in the game yet; taking it changes nothing.",
                theme::warning(),
            ));
        }
        lines.push(Line::styled(text.to_owned(), theme::text()));
        lines
    }
}

impl Default for LevelUp {
    fn default() -> Self {
        Self::new()
    }
}

/// The perks the character could take now, as an id and the label to show. Read
/// fresh each time, so the list follows the character.
fn open_perks(game: &Game) -> Vec<(PerkId, String)> {
    let character = game.character();
    game.perks()
        .open_to(character)
        .into_iter()
        .map(|perk| {
            let max = engine::character::PerkBook::max_rank(perk);
            let label = if max > 1 {
                format!(
                    "{} (rank {}/{max})",
                    perk.name,
                    character.perk_rank(&perk.id) + 1
                )
            } else {
                perk.name.clone()
            };
            (perk.id.clone(), label)
        })
        .collect()
}

fn spend(game: &mut Game, log: &mut Log, skill: Skill) {
    match game.spend_skill_point(skill) {
        Ok(()) => log.push(Entry::Note(format!(
            "{} rises to {}.",
            skill.name(),
            game.character().skill(skill).get()
        ))),
        Err(error) => log.push(Entry::Note(error.to_string())),
    }
}
