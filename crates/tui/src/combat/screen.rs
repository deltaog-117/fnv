//! One fight, driven by the engine's `Battle`: the player picks a target or passes,
//! and the enemies answer in the log.

use engine::combat::{
    AimedHitChance, Combat, Combatant, Dice, FighterId, Flight, Power, Resolution, Side, Status,
    Strike,
};
use engine::game::{Battle, Fighter, Game, Outcome};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{self, Item, Menu};
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
    /// Try to run away.
    Flee,
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
    /// Starts the screen on a fresh battle. A foe that is quicker than the player
    /// acts first: what it did before the player's turn goes to the log.
    #[must_use]
    pub fn new(mut battle: Battle, log: &mut Log, dice: &mut impl Dice) -> Self {
        let names = names(battle.combat());
        let opening = battle.open(dice);
        let mut fight = Self {
            battle,
            names,
            actions: Vec::new(),
            menu: Menu::new(0),
        };
        if !opening.is_empty() {
            log.push(Entry::Note("They are quicker and shoot first.".to_owned()));
            opening
                .iter()
                .for_each(|resolution| log.push(Entry::Note(fight.describe(resolution))));
        }
        fight.refresh();
        fight
    }

    /// Whether the fight ended before the player could act, so it must be settled
    /// at once.
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.battle.combat().status() != Status::Ongoing
    }

    /// Settles a fight that is over, as [`Fight::update`] does after a move.
    pub fn finish(&self, game: &mut Game, log: &mut Log) -> Next {
        self.settle(game, log)
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
                "Choose \"Run away\" to try to get out of this fight.".to_owned(),
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
            Action::Flee => match self.battle.flee(dice) {
                Ok((Flight::Escaped, _)) => log.push(Entry::Note("You get away.".to_owned())),
                Ok((Flight::Blocked, resolutions)) => {
                    log.push(Entry::Note("You cannot get away!".to_owned()));
                    resolutions
                        .iter()
                        .for_each(|resolution| log.push(Entry::Note(self.describe(resolution))));
                }
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
                    Outcome::Fled => "You got away from the fight.",
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
            .chain([Action::Flee, Action::EndTurn])
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
        // The player first, then the allies together on one line (a fight with three
        // helpers would otherwise push the actions off a small terminal), then the foes.
        let ally =
            |id: &FighterId| *id != self.battle.player() && combat.side(*id) == Some(Side::Player);
        let (allies, others): (Vec<FighterId>, Vec<FighterId>) =
            combat.ids().partition(|id| ally(id));
        let mut shown: Vec<FighterId> = vec![self.battle.player()];
        shown.extend(
            others
                .iter()
                .copied()
                .filter(|id| *id != self.battle.player()),
        );
        for id in shown {
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
            if id == self.battle.player() && !allies.is_empty() {
                lines.push(self.allies_line(combat, &allies));
            }
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
                Action::Flee => Item::new(format!(
                    "Run away ({}% to get out)",
                    combat
                        .combatant(self.battle.player())
                        .map_or(0, |you| you.flee_chance().get())
                )),
                Action::EndTurn => Item::new("End turn"),
            })
            .collect();
        // A fight with allies lists more than a short terminal can hold, so the
        // actions scroll with the cursor, as the other menus do.
        let room = usize::from(area.height)
            .saturating_sub(2)
            .saturating_sub(lines.len())
            .max(1);
        lines.extend(menu::window(
            self.menu.lines(&items),
            self.menu.selected(),
            room,
        ));
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

    /// "On your side: Sunny Smiles 30/30, Trudy 0/35 (down)", one line, with a fallen
    /// ally dimmed.
    fn allies_line(
        &self,
        combat: &Combat<Fighter, AimedHitChance>,
        allies: &[FighterId],
    ) -> Line<'static> {
        let mut spans = vec![Span::styled("On your side: ", theme::heading())];
        for (index, &id) in allies.iter().enumerate() {
            let hp = combat.hp(id).unwrap_or(0);
            let max = combat
                .combatant(id)
                .map_or(0, |c| engine::combat::Combatant::max_hp(c).get());
            let (text, style) = if hp == 0 {
                (format!("{} {hp}/{max} (down)", self.name(id)), theme::dim())
            } else {
                (format!("{} {hp}/{max}", self.name(id)), theme::text())
            };
            if index > 0 {
                spans.push(Span::styled(", ", theme::dim()));
            }
            spans.push(Span::styled(text, style));
        }
        Line::from(spans)
    }

    /// Why the player cannot shoot right now, if they cannot.
    fn shot_refusal(&self, combat: &Combat<Fighter, AimedHitChance>) -> Option<&'static str> {
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
fn names(combat: &Combat<Fighter, AimedHitChance>) -> Vec<String> {
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
