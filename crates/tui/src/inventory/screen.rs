//! The player's pack: every stack on the left, what is worn and what the chosen
//! item can do on the right.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::game::Game;
use engine::inventory::Slot;
use engine::shared::{InventoryView, ItemId, ItemUse};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{self, Item, Menu};
use crate::shared::names::Names;
use crate::shared::quantity::Quantity;
use crate::shared::theme;

/// How far Up and Down move the count in the "how many" question.
const BIG_STEP: u32 = 10;

/// What the player's command led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// The screen stays open.
    Stay,
    /// The player is done with the pack.
    Leave,
}

/// Something the player can do with a stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Use,
    Equip,
    Unequip(Slot),
    Drop,
    Back,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Use => "Use",
            Self::Equip => "Equip",
            Self::Unequip(_) => "Unequip",
            Self::Drop => "Drop",
            Self::Back => "Back",
        }
    }
}

/// What the screen is waiting for.
#[derive(Debug)]
enum Mode {
    /// Moving through the stacks.
    Browsing,
    /// Choosing what to do with `item`.
    Acting {
        item: ItemId,
        actions: Vec<Action>,
        menu: Menu,
    },
    /// Choosing how many of a stack to throw away.
    Dropping { item: ItemId, quantity: Quantity },
}

type Stack = (ItemId, NonZeroU32);

/// An open pack. It holds only the cursor and the pending question: stacks, worn
/// items and weight are read from the game whenever they are needed, so the
/// screen cannot show a number the engine has since changed.
#[derive(Debug)]
pub struct Pack {
    names: Arc<Names>,
    list: Menu,
    mode: Mode,
}

impl Pack {
    /// Opens the pack over what the game carries now.
    #[must_use]
    pub fn open(game: &Game, names: Arc<Names>) -> Self {
        Self {
            names,
            list: Menu::new(stacks(game).len()),
            mode: Mode::Browsing,
        }
    }

    /// The keys that work right now.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        match self.mode {
            Mode::Browsing => "Up/Down move | Enter actions | Esc leave | q quit",
            Mode::Acting { .. } => "Up/Down move | Enter choose | 1-9 pick | Esc back | q quit",
            Mode::Dropping { .. } => {
                "Left/Right +/-1 | Up/Down +/-10 | Enter confirm | Esc cancel | q quit"
            }
        }
    }

    /// Applies a command. Esc closes a question first and the pack second; every
    /// change goes through the game, so a refusal comes back in the engine's words.
    pub fn update(&mut self, command: Command, game: &mut Game, log: &mut Log) -> Next {
        match &mut self.mode {
            Mode::Browsing => {
                if command == Command::Back {
                    return Next::Leave;
                }
                if let Some(index) = self.list.handle(command) {
                    self.offer_actions(index, game);
                }
            }
            Mode::Acting {
                item,
                actions,
                menu,
            } => {
                if command == Command::Back {
                    self.mode = Mode::Browsing;
                } else if let Some(action) = menu.handle(command).and_then(|i| actions.get(i)) {
                    let (item, action) = (item.clone(), *action);
                    self.perform(action, &item, game, log);
                }
            }
            Mode::Dropping { item, quantity } => match command {
                Command::Select => {
                    let (item, count) = (item.clone(), quantity.get());
                    self.mode = Mode::Browsing;
                    self.throw_away(&item, count, game, log);
                }
                Command::Back => self.mode = Mode::Browsing,
                Command::Left => quantity.lower(1),
                Command::Right => quantity.raise(1),
                Command::Down => quantity.lower(BIG_STEP),
                Command::Up => quantity.raise(BIG_STEP),
                Command::Pick(_) | Command::Quit => {}
            },
        }
        Next::Stay
    }

    /// Opens the action menu of the stack at `index`.
    fn offer_actions(&mut self, index: usize, game: &Game) {
        let Some((item, _)) = stacks(game).into_iter().nth(index) else {
            return;
        };
        let actions = actions(game, &item);
        self.mode = Mode::Acting {
            menu: Menu::new(actions.len()),
            item,
            actions,
        };
    }

    fn perform(&mut self, action: Action, item: &ItemId, game: &mut Game, log: &mut Log) {
        let name = self.names.name(item);
        let note = match action {
            Action::Use => game
                .use_item(item)
                .map(|healed| format!("You use the {name} and recover {healed} hit points."))
                .map_err(|error| error.to_string()),
            Action::Equip => game
                .equip(item)
                .map(|_| format!("You equip the {name}."))
                .map_err(|error| error.to_string()),
            Action::Unequip(slot) => {
                game.unequip(slot);
                Ok(format!("You put the {name} away."))
            }
            Action::Drop => {
                let held = game.inventory().item_count(item);
                match NonZeroU32::new(held) {
                    Some(count) if count > NonZeroU32::MIN => {
                        self.mode = Mode::Dropping {
                            item: item.clone(),
                            quantity: Quantity::new(count),
                        };
                        return;
                    }
                    _ => {
                        self.mode = Mode::Browsing;
                        self.throw_away(item, NonZeroU32::MIN, game, log);
                        return;
                    }
                }
            }
            Action::Back => {
                self.mode = Mode::Browsing;
                return;
            }
        };
        log.push(Entry::Note(note.unwrap_or_else(|error| error)));
        self.mode = Mode::Browsing;
        self.list = self.list.resized(stacks(game).len());
    }

    fn throw_away(&mut self, item: &ItemId, count: NonZeroU32, game: &mut Game, log: &mut Log) {
        let name = self.names.name(item);
        let note = game
            .drop_item(item, count)
            .map(|()| format!("You drop {count} {name}."))
            .unwrap_or_else(|error| error.to_string());
        log.push(Entry::Note(note));
        self.list = self.list.resized(stacks(game).len());
    }

    /// Draws the stacks, the worn items, the chosen item and the question line.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let [panels, prompt] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
                .areas(panels);
        self.render_stacks(frame, left, game);
        self.render_side(frame, right, game);

        let carried = game.inventory().weight();
        let limit = game.character().carry_weight();
        let load = format!("Carrying {carried:.1} of {limit} lb.");
        let (text, style) = match &self.mode {
            Mode::Dropping { item, quantity } => (
                format!(
                    "Drop how many {}? < {} of {} >",
                    self.names.name(item),
                    quantity.get(),
                    quantity.max()
                ),
                theme::heading(),
            ),
            _ if game.is_encumbered() => {
                (format!("{load} You are overencumbered."), theme::warning())
            }
            _ => (load, theme::dim()),
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Pack ");
        frame.render_widget(
            Paragraph::new(Line::styled(text, style))
                .block(block)
                .wrap(Wrap { trim: false }),
            prompt,
        );
    }

    fn render_stacks(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let width = usize::from(area.width.saturating_sub(2));
        let height = usize::from(area.height.saturating_sub(2));
        let lines: Vec<Line<'static>> = stacks(game)
            .iter()
            .enumerate()
            .map(|(index, (item, count))| {
                let tag = worn_in(game, item).map_or("", |slot| match slot {
                    Slot::Weapon => "[wielded]",
                    Slot::Outfit => "[worn]",
                    Slot::Head => "[on head]",
                });
                row_line(
                    &format!("{} x{count}", self.names.name(item)),
                    tag,
                    width,
                    self.list.selected() == Some(index),
                )
            })
            .collect();
        let lines = if lines.is_empty() {
            vec![Line::styled("You carry nothing.", theme::dim())]
        } else {
            menu::window(lines, self.list.selected(), height)
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::text())
            .title(format!(" Your pack - {} caps ", game.caps()));
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    /// What is worn, then either the actions on offer or the chosen item.
    fn render_side(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let slot_line = |label: &str, slot: Slot| {
            let name = game
                .inventory()
                .equipped(slot)
                .map_or_else(|| "nothing".to_owned(), |item| self.names.name(item));
            Line::styled(format!("{label}: {name}"), theme::text())
        };
        let mut lines = vec![
            slot_line("Weapon", Slot::Weapon),
            slot_line("Outfit", Slot::Outfit),
            slot_line("Head", Slot::Head),
            Line::default(),
        ];
        match &self.mode {
            Mode::Acting {
                item,
                actions,
                menu,
            } => {
                lines.push(Line::styled(self.names.name(item), theme::heading()));
                let items: Vec<Item> = actions
                    .iter()
                    .map(|action| Item::new(action.label()))
                    .collect();
                lines.extend(menu.lines(&items));
            }
            Mode::Browsing | Mode::Dropping { .. } => {
                let chosen = self
                    .list
                    .selected()
                    .and_then(|index| stacks(game).into_iter().nth(index));
                if let Some((item, count)) = chosen {
                    lines.extend(self.describe(game, &item, count));
                }
            }
        }
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Worn and details ");
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn describe(&self, game: &Game, item: &ItemId, count: NonZeroU32) -> Vec<Line<'static>> {
        let mut lines = vec![Line::styled(
            format!("{} x{count}", self.names.name(item)),
            theme::heading(),
        )];
        if let Some(value) = game.inventory().value(item) {
            lines.push(Line::styled(
                format!("Worth {value:.0} caps."),
                theme::text(),
            ));
        }
        match game.inventory().item_use(item) {
            Some(ItemUse::Heal(hp)) => lines.push(Line::styled(
                format!("Restores {hp} hit points."),
                theme::text(),
            )),
            None => {}
        }
        lines
    }
}

/// The stacks the player carries, in the order the panel lists them.
fn stacks(game: &Game) -> Vec<Stack> {
    game.inventory()
        .stacks()
        .map(|(item, count)| (item.clone(), count))
        .collect()
}

/// The slot `item` is in, if it is equipped.
fn worn_in(game: &Game, item: &ItemId) -> Option<Slot> {
    [Slot::Weapon, Slot::Outfit, Slot::Head]
        .into_iter()
        .find(|slot| game.inventory().equipped(*slot) == Some(item))
}

/// What the player can do with `item` right now, ending with "Back".
fn actions(game: &Game, item: &ItemId) -> Vec<Action> {
    let inventory = game.inventory();
    let mut actions = Vec::new();
    if inventory.item_use(item).is_some() {
        actions.push(Action::Use);
    }
    if let Some(slot) = inventory.slot_for(item) {
        actions.push(match worn_in(game, item) {
            Some(worn) => Action::Unequip(worn),
            None => Action::Equip,
        });
        debug_assert!(worn_in(game, item).is_none_or(|worn| worn == slot));
    }
    actions.extend([Action::Drop, Action::Back]);
    actions
}

/// One row: the label on the left, a tag on the right, cut with an ellipsis to fit
/// `width` columns.
fn row_line(label: &str, tag: &str, width: usize, selected: bool) -> Line<'static> {
    let marker = if selected { "> " } else { "  " };
    let tag_len = tag.chars().count();
    let room = width.saturating_sub(marker.len() + tag_len + 1);
    let label = if label.chars().count() > room {
        let mut cut: String = label.chars().take(room.saturating_sub(1)).collect();
        if room > 0 {
            cut.push('…');
        }
        cut
    } else {
        label.to_owned()
    };
    let gap = width.saturating_sub(marker.len() + label.chars().count() + tag_len);
    let style = if selected {
        theme::selected()
    } else {
        theme::text()
    };
    Line::styled(format!("{marker}{label}{}{tag}", " ".repeat(gap)), style)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::OnceLock;

    use engine::inventory::ItemBook;
    use engine::quests::QuestBook;
    use engine::shared::{Effect, GameData, Scope, apply_all};
    use engine::world::WorldBook;
    use proptest::prelude::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    const ITEMS: [&str; 4] = ["stimpak", "9mm_pistol", "leather_armor", "9mm_round"];

    fn fixture() -> (Game, Arc<Names>) {
        static FIXTURE: OnceLock<(Game, Arc<Names>)> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
                let data = GameData::load(&dir, &Scope::Everywhere).expect("the data loads");
                let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("quests build"));
                let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
                let items = Arc::new(ItemBook::from_data(&data).expect("items build"));
                let game = Game::with_world(quests, world).with_items(items);
                (game, Arc::new(Names::from_data(&data)))
            })
            .clone()
    }

    fn id(raw: &str) -> ItemId {
        ItemId::try_new(raw).expect("valid id")
    }

    /// A game that carries `items`, wounded by `wounds`, and a pack open on it.
    fn carrying(wounds: u32, items: &[(&str, u32)]) -> (Game, Pack, Log) {
        let (game, names) = fixture();
        let mut game = game.with_wounds(wounds);
        let gifts: Vec<Effect> = items
            .iter()
            .map(|(item, count)| Effect::GiveItem {
                item: id(item),
                count: NonZeroU32::new(*count).expect("nonzero"),
            })
            .collect();
        apply_all(&gifts, &mut game).expect("the player is stocked");
        let pack = Pack::open(&game, names);
        (game, pack, Log::default())
    }

    /// Moves the cursor onto `item`.
    fn point_at(pack: &mut Pack, game: &mut Game, log: &mut Log, item: &str) {
        let target = stacks(game)
            .iter()
            .position(|(candidate, _)| candidate.as_str() == item)
            .expect("the item is listed");
        for _ in 0..stacks(game).len() {
            if pack.list.selected() == Some(target) {
                return;
            }
            pack.update(Command::Down, game, log);
        }
    }

    /// Opens the actions of `item` and chooses the one called `label`.
    fn choose(pack: &mut Pack, game: &mut Game, log: &mut Log, item: &str, label: &str) {
        point_at(pack, game, log, item);
        pack.update(Command::Select, game, log);
        let Mode::Acting { actions, .. } = &pack.mode else {
            panic!("the actions of {item} are open");
        };
        let index = actions
            .iter()
            .position(|action| action.label() == label)
            .unwrap_or_else(|| panic!("{item} offers {label}"));
        pack.update(Command::Pick(index), game, log);
    }

    fn said(log: &Log) -> String {
        log.tail(200, 8)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn using_a_stimpak_heals_spends_it_and_says_so() {
        let (mut game, mut pack, mut log) = carrying(40, &[("stimpak", 2)]);
        let before = game.hp();
        choose(&mut pack, &mut game, &mut log, "stimpak", "Use");
        assert_eq!(game.hp(), before + 30);
        assert_eq!(game.inventory().item_count(&id("stimpak")), 1);
        assert!(said(&log).contains("recover 30"), "{}", said(&log));
        assert!(matches!(pack.mode, Mode::Browsing));
    }

    #[test]
    fn a_stimpak_is_kept_when_there_is_nothing_to_heal() {
        let (mut game, mut pack, mut log) = carrying(0, &[("stimpak", 1)]);
        choose(&mut pack, &mut game, &mut log, "stimpak", "Use");
        assert_eq!(game.inventory().item_count(&id("stimpak")), 1);
        assert!(said(&log).contains("full health"), "{}", said(&log));
    }

    #[test]
    fn a_weapon_is_equipped_and_then_offered_for_unequipping() {
        let (mut game, mut pack, mut log) = carrying(0, &[("9mm_pistol", 1)]);
        choose(&mut pack, &mut game, &mut log, "9mm_pistol", "Equip");
        assert_eq!(
            game.inventory().equipped(Slot::Weapon),
            Some(&id("9mm_pistol"))
        );
        choose(&mut pack, &mut game, &mut log, "9mm_pistol", "Unequip");
        assert_eq!(game.inventory().equipped(Slot::Weapon), None);
    }

    #[test]
    fn an_item_with_no_use_does_not_offer_one() {
        let (mut game, mut pack, mut log) = carrying(0, &[("9mm_round", 5)]);
        point_at(&mut pack, &mut game, &mut log, "9mm_round");
        pack.update(Command::Select, &mut game, &mut log);
        let Mode::Acting { actions, .. } = &pack.mode else {
            panic!("the actions are open");
        };
        assert_eq!(actions, &[Action::Drop, Action::Back]);
    }

    #[test]
    fn dropping_a_bigger_stack_asks_how_many() {
        let (mut game, mut pack, mut log) = carrying(0, &[("9mm_round", 20)]);
        choose(&mut pack, &mut game, &mut log, "9mm_round", "Drop");
        assert!(matches!(pack.mode, Mode::Dropping { .. }));
        assert_eq!(game.inventory().item_count(&id("9mm_round")), 20);
        pack.update(Command::Up, &mut game, &mut log);
        pack.update(Command::Select, &mut game, &mut log);
        assert_eq!(game.inventory().item_count(&id("9mm_round")), 9);
        assert!(said(&log).contains("You drop 11"), "{}", said(&log));
    }

    #[test]
    fn dropping_a_single_item_needs_no_question() {
        let (mut game, mut pack, mut log) = carrying(0, &[("stimpak", 1)]);
        choose(&mut pack, &mut game, &mut log, "stimpak", "Drop");
        assert_eq!(game.inventory().item_count(&id("stimpak")), 0);
        assert!(stacks(&game).is_empty());
        assert_eq!(pack.list.selected(), None);
    }

    #[test]
    fn the_last_equipped_copy_cannot_be_dropped() {
        let (mut game, mut pack, mut log) = carrying(0, &[("leather_armor", 1)]);
        game.equip(&id("leather_armor")).expect("armor can be worn");
        choose(&mut pack, &mut game, &mut log, "leather_armor", "Drop");
        assert_eq!(game.inventory().item_count(&id("leather_armor")), 1);
        assert!(said(&log).contains("equipped"), "{}", said(&log));
    }

    #[test]
    fn escape_closes_the_question_then_the_menu_then_the_pack() {
        let (mut game, mut pack, mut log) = carrying(0, &[("9mm_round", 20)]);
        choose(&mut pack, &mut game, &mut log, "9mm_round", "Drop");
        assert_eq!(pack.update(Command::Back, &mut game, &mut log), Next::Stay);
        assert!(matches!(pack.mode, Mode::Browsing));
        pack.update(Command::Select, &mut game, &mut log);
        assert!(matches!(pack.mode, Mode::Acting { .. }));
        assert_eq!(pack.update(Command::Back, &mut game, &mut log), Next::Stay);
        assert!(matches!(pack.mode, Mode::Browsing));
        assert_eq!(pack.update(Command::Back, &mut game, &mut log), Next::Leave);
    }

    #[test]
    fn the_screen_shows_stacks_worn_items_and_the_load() {
        let (mut game, mut pack, mut log) = carrying(0, &[("stimpak", 2), ("leather_armor", 1)]);
        game.equip(&id("leather_armor")).expect("armor can be worn");
        point_at(&mut pack, &mut game, &mut log, "stimpak");
        let mut terminal = Terminal::new(TestBackend::new(100, 20)).expect("terminal");
        terminal
            .draw(|frame| pack.render(frame, frame.area(), &game))
            .expect("draws");
        let buffer = terminal.backend().buffer();
        let text: String = (0..20)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol())
            .collect();
        for expected in [
            "Stimpak x2",
            "[worn]",
            "Outfit: Leather armor",
            "Restores 30 hit points",
            "Carrying",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text}");
        }
    }

    fn any_command() -> impl Strategy<Value = Command> {
        prop_oneof![
            Just(Command::Up),
            Just(Command::Down),
            Just(Command::Left),
            Just(Command::Right),
            Just(Command::Select),
            Just(Command::Back),
            (0usize..10).prop_map(Command::Pick),
        ]
    }

    proptest! {
        #[test]
        fn no_key_sequence_adds_items_or_caps_or_overheals(
            commands in proptest::collection::vec(any_command(), 0..150),
        ) {
            let (mut game, mut pack, mut log) = carrying(
                60,
                &[("stimpak", 6), ("9mm_pistol", 1), ("leather_armor", 1), ("9mm_round", 40)],
            );
            let caps = game.caps();
            let max = game.character().max_hp();
            let mut previous: Vec<u32> =
                ITEMS.iter().map(|item| game.inventory().item_count(&id(item))).collect();
            for command in commands {
                pack.update(command, &mut game, &mut log);
                let now: Vec<u32> =
                    ITEMS.iter().map(|item| game.inventory().item_count(&id(item))).collect();
                prop_assert!(now.iter().zip(&previous).all(|(now, before)| now <= before));
                previous = now;
                prop_assert_eq!(game.caps(), caps);
                prop_assert!(game.hp() <= max);
                prop_assert!(pack.list.selected().is_none_or(|index| index < stacks(&game).len()));
            }
        }

        #[test]
        fn a_row_never_runs_past_the_panel(
            label in "[a-zA-Z0-9 ]{0,60}",
            tag in "(|\\[worn\\]|\\[wielded\\])",
            width in 12usize..80,
            selected in any::<bool>(),
        ) {
            let line = row_line(&label, &tag, width, selected);
            prop_assert!(line.to_string().chars().count() <= width);
        }
    }
}
