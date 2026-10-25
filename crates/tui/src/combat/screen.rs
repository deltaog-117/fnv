//! One fight, driven by the engine's `Battle`: the player picks a target or passes,
//! and the enemies answer in the log.

use engine::combat::{
    Combat, Combatant, Dice, FighterId, FlatHitChance, Power, Resolution, Side, Status, Strike,
};
use engine::game::{Battle, Fighter, Game, Outcome};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{Item, Menu};
use crate::shared::theme;

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// The fight goes on.
    Stay,
    /// The fight is over and settled; `None` when settling failed and the game was
    /// left as it was.
    Done(Option<Outcome>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Shoot(FighterId),
    Reload,
    EndTurn,
}

/// A fight in progress and the menu of what the player can do.
#[derive(Debug)]
pub struct Fight {
    battle: Battle,
    /// Display name of every fighter, in turn order; twins are numbered.
    names: Vec<String>,
    actions: Vec<Action>,
    menu: Menu,
}

impl Fight {
    /// Starts the screen on a fresh battle.
    #[must_use]
    pub fn new(battle: Battle) -> Self {
        let names = names(battle.combat());
        let mut fight = Self {
            battle,
            names,
            actions: Vec::new(),
            menu: Menu::new(0),
        };
        fight.refresh();
        fight
    }

    /// Applies a command: moves the cursor, or does the chosen action. What
    /// happened goes to the log; a finished fight is settled in `game`.
    pub fn update(
        &mut self,
        command: Command,
        game: &mut Game,
        log: &mut Log,
        dice: &mut impl Dice,
    ) -> Next {
        if command == Command::Back {
            log.push(Entry::Note(
                "There is no running from this fight.".to_owned(),
            ));
            return Next::Stay;
        }
        let Some(action) = self
            .menu
            .handle(command)
            .and_then(|index| self.actions.get(index))
            .copied()
        else {
            return Next::Stay;
        };
        match action {
            Action::Shoot(target) => match self.battle.attack(target, Power::Normal, dice) {
                Ok(resolution) => log.push(Entry::Note(self.describe(&resolution))),
                Err(error) => log.push(Entry::Note(error.to_string())),
            },
            Action::Reload => match self.battle.reload() {
                Ok(reload) => log.push(Entry::Note(format!(
                    "You reload: {} rounds in the weapon.",
                    reload.loaded
                ))),
                Err(error) => log.push(Entry::Note(error.to_string())),
            },
            Action::EndTurn => match self.battle.end_turn(dice) {
                Ok(resolutions) => resolutions
                    .iter()
                    .for_each(|resolution| log.push(Entry::Note(self.describe(resolution)))),
                Err(error) => log.push(Entry::Note(error.to_string())),
            },
        }
        if self.battle.combat().status() != Status::Ongoing {
            return self.settle(game, log);
        }
        self.refresh();
        Next::Stay
    }

    fn settle(&self, game: &mut Game, log: &mut Log) -> Next {
        match game.finish_battle(&self.battle) {
            Ok(outcome) => {
                let message = match outcome {
                    Outcome::Victory => "The fight is won.",
                    Outcome::Defeat => "You are down.",
                };
                log.push(Entry::Note(message.to_owned()));
                Next::Done(Some(outcome))
            }
            Err(error) => {
                log.push(Entry::Note(error.to_string()));
                Next::Done(None)
            }
        }
    }

    /// How many things the player can do right now; the last one is always passing.
    #[cfg(test)]
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Rebuilds the actions: a shot at each foe still standing, then pass.
    fn refresh(&mut self) {
        let combat = self.battle.combat();
        self.actions = combat
            .ids()
            .filter(|&id| combat.side(id) == Some(Side::Enemy) && combat.is_standing(id))
            .map(Action::Shoot)
            .chain(combat.rounds(self.battle.player()).map(|_| Action::Reload))
            .chain([Action::EndTurn])
            .collect();
        self.menu = Menu::new(self.actions.len());
    }

    fn name(&self, id: FighterId) -> &str {
        self.names.get(id.index()).map_or("?", String::as_str)
    }

    /// One line for an attack: who did what to whom, and what is left.
    fn describe(&self, resolution: &Resolution) -> String {
        let you = self.battle.player();
        let (attacker, target) = (resolution.attacker, resolution.target);
        let (subject, miss, hit) = if attacker == you {
            ("You".to_owned(), "miss", "hit")
        } else {
            (self.name(attacker).to_owned(), "misses", "hits")
        };
        let object = if target == you {
            "you"
        } else {
            self.name(target)
        };
        let mut text = match resolution.strike {
            Strike::Missed => format!("{subject} {miss} {object}."),
            Strike::Hit { critical, damage } => {
                let crit = if critical { " A critical hit!" } else { "" };
                format!("{subject} {hit} {object} for {damage}.{crit}")
            }
        };
        if resolution.defeated {
            let down = if target == you {
                "go down"
            } else {
                "goes down"
            };
            let who = if target == you {
                "You"
            } else {
                self.name(target)
            };
            text.push_str(&format!(" {who} {down}."));
        }
        text
    }

    /// Draws the fighters and what the player can do.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let combat = self.battle.combat();
        let your_turn = combat.active() == self.battle.player();
        let ap = combat.ap_left();
        let mut lines = vec![
            Line::styled(
                format!(
                    "Round {} | {} | {ap} AP left",
                    combat.round(),
                    if your_turn { "your turn" } else { "their turn" }
                ),
                theme::heading(),
            ),
            Line::default(),
        ];
        for id in combat.ids() {
            let hp = combat.hp(id).unwrap_or(0);
            let max = combat
                .combatant(id)
                .map_or(0, |c| engine::combat::Combatant::max_hp(c).get());
            let style = if hp == 0 { theme::dim() } else { theme::text() };
            // Only the player's rounds are counted; the foes' supply is endless.
            let ammo = (id == self.battle.player())
                .then(|| combat.rounds(id))
                .flatten()
                .map(|rounds| match rounds.reserve {
                    Some(reserve) => format!("  Ammo {}/{reserve}", rounds.loaded),
                    None => format!("  Ammo {}", rounds.loaded),
                })
                .unwrap_or_default();
            lines.push(Line::styled(
                format!("{:<20} HP {hp}/{max}{ammo}", self.name(id)),
                style,
            ));
        }
        lines.push(Line::default());
        lines.push(Line::styled("What do you do?", theme::heading()));
        let items: Vec<Item> = self
            .actions
            .iter()
            .map(|action| match *action {
                Action::Shoot(id) => {
                    let label = format!("Shoot {}", self.name(id));
                    match self.shot_refusal(combat) {
                        None => Item::new(label),
                        Some(reason) => Item::locked(format!("{label} ({reason})")),
                    }
                }
                Action::Reload => Item::new("Reload"),
                Action::EndTurn => Item::new("End turn"),
            })
            .collect();
        lines.extend(self.menu.lines(&items));
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Fight ");
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    /// Why the player cannot shoot right now, if they cannot.
    fn shot_refusal(&self, combat: &Combat<Fighter, FlatHitChance>) -> Option<&'static str> {
        let who = combat.combatant(combat.active())?;
        if who.attack().ap_cost.get() > combat.ap_left() {
            return Some("not enough AP");
        }
        let loaded = combat.rounds(combat.active())?.loaded;
        let needed = who.magazine()?.per_shot.get();
        (loaded < needed).then_some("out of ammo")
    }
}

/// Names in turn order, numbering fighters that share one ("Powder Ganger 1").
fn names(combat: &Combat<Fighter, FlatHitChance>) -> Vec<String> {
    let all: Vec<&str> = combat
        .ids()
        .filter_map(|id| combat.combatant(id))
        .map(Fighter::name)
        .collect();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    all.iter()
        .map(|&name| {
            if all.iter().filter(|&&other| other == name).count() > 1 {
                let count = seen.entry(name).or_insert(0);
                *count += 1;
                format!("{name} {count}")
            } else {
                name.to_owned()
            }
        })
        .collect()
}
