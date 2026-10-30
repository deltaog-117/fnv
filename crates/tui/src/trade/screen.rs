//! Buying and selling: the vendor's stock on the left, the player's pack on the
//! right, one cursor in each.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::game::Game;
use engine::shared::{InventoryView, ItemId, VendorId};
use engine::trade::Shop;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::log::{Entry, Log};
use crate::shared::menu::{self, Menu};
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
    /// The player is done trading.
    Leave,
}

/// Which side of the counter a panel is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// The vendor's stock: choosing a row buys it.
    Buy,
    /// The player's pack: choosing a row sells it.
    Sell,
}

/// A stack the player chose that holds more than one, and the count asked for.
#[derive(Debug, Clone)]
struct Ask {
    side: Side,
    item: ItemId,
    quantity: Quantity,
}

type Stack = (ItemId, NonZeroU32);

/// An open shop. It holds only the cursors and the pending question: stock, pack,
/// prices and caps are read from the game whenever they are needed, so the screen
/// cannot show a number the engine has since changed.
#[derive(Debug)]
pub struct Trade {
    vendor: VendorId,
    name: String,
    names: Arc<Names>,
    focus: Side,
    buy: Menu,
    sell: Menu,
    ask: Option<Ask>,
}

impl Trade {
    /// Opens the shop of `vendor`, or `None` when the game has no such shop.
    #[must_use]
    pub fn open(vendor: VendorId, name: String, game: &Game, names: Arc<Names>) -> Option<Self> {
        game.shop(&vendor)?;
        let buy = Menu::new(stock(game, &vendor).len());
        let sell = Menu::new(pack(game).len());
        Some(Self {
            vendor,
            name,
            names,
            focus: Side::Buy,
            buy,
            sell,
            ask: None,
        })
    }

    /// The keys that work right now.
    #[must_use]
    pub fn keys(&self) -> &'static str {
        if self.ask.is_some() {
            "Left/Right +/-1 | Up/Down +/-10 | Enter confirm | Esc cancel | q quit"
        } else {
            "Up/Down move | Left/Right switch panel | Enter trade | Esc leave | q quit"
        }
    }

    /// Applies a command. Esc closes a question first and the shop second; every
    /// trade goes through the game, so a refusal comes back in the engine's words.
    pub fn update(&mut self, command: Command, game: &mut Game, log: &mut Log) -> Next {
        if self.ask.is_some() {
            self.answer(command, game, log);
            return Next::Stay;
        }
        match command {
            Command::Back => return Next::Leave,
            Command::Left => self.focus = Side::Buy,
            Command::Right => self.focus = Side::Sell,
            Command::Up | Command::Down | Command::Select | Command::Pick(_) => {
                let menu = match self.focus {
                    Side::Buy => &mut self.buy,
                    Side::Sell => &mut self.sell,
                };
                if let Some(index) = menu.handle(command) {
                    self.choose(index, game, log);
                }
            }
            Command::Quit => {}
        }
        Next::Stay
    }

    /// Trades the stack at `index` of the focused panel: one unit at once, or ask
    /// how many when the stack holds more.
    fn choose(&mut self, index: usize, game: &mut Game, log: &mut Log) {
        let side = self.focus;
        let stacks = match side {
            Side::Buy => stock(game, &self.vendor),
            Side::Sell => pack(game),
        };
        let Some((item, count)) = stacks.into_iter().nth(index) else {
            return;
        };
        if count == NonZeroU32::MIN {
            self.trade(side, &item, count, game, log);
        } else {
            self.ask = Some(Ask {
                side,
                item,
                quantity: Quantity::new(count),
            });
        }
    }

    /// Handles a command while the "how many" question is open.
    fn answer(&mut self, command: Command, game: &mut Game, log: &mut Log) {
        match command {
            Command::Select => {
                if let Some(ask) = self.ask.take() {
                    self.trade(ask.side, &ask.item, ask.quantity.get(), game, log);
                }
            }
            Command::Back => self.ask = None,
            Command::Left | Command::Right | Command::Up | Command::Down => {
                if let Some(ask) = self.ask.as_mut() {
                    match command {
                        Command::Left => ask.quantity.lower(1),
                        Command::Right => ask.quantity.raise(1),
                        Command::Down => ask.quantity.lower(BIG_STEP),
                        _ => ask.quantity.raise(BIG_STEP),
                    }
                }
            }
            Command::Pick(_) | Command::Quit => {}
        }
    }

    fn trade(
        &mut self,
        side: Side,
        item: &ItemId,
        count: NonZeroU32,
        game: &mut Game,
        log: &mut Log,
    ) {
        let name = self.names.name(item);
        let note = match side {
            Side::Buy => game
                .buy(&self.vendor, item, count)
                .map(|price| format!("You buy {count} {name} for {price} caps.")),
            Side::Sell => game
                .sell(&self.vendor, item, count)
                .map(|price| format!("You sell {count} {name} for {price} caps.")),
        };
        log.push(Entry::Note(note.unwrap_or_else(|error| error.to_string())));
        self.buy = resized(&self.buy, stock(game, &self.vendor).len());
        self.sell = resized(&self.sell, pack(game).len());
    }

    /// Draws both panels and the question line under them.
    pub fn render(&self, frame: &mut Frame, area: Rect, game: &Game) {
        let [panels, prompt] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(panels);
        let shop_caps = game.shop(&self.vendor).map_or(0, Shop::caps);
        self.render_panel(
            frame,
            left,
            game,
            Side::Buy,
            format!(" {} - {shop_caps} caps ", self.name),
        );
        self.render_panel(
            frame,
            right,
            game,
            Side::Sell,
            format!(" Your pack - {} caps ", game.caps()),
        );

        let (text, style) = match &self.ask {
            Some(ask) => (self.question(ask, game), theme::heading()),
            None => (
                "Enter trades the selected stack. Prices are per unit.".to_owned(),
                theme::dim(),
            ),
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Trade ");
        frame.render_widget(
            Paragraph::new(Line::styled(text, style))
                .block(block)
                .wrap(Wrap { trim: false }),
            prompt,
        );
    }

    fn question(&self, ask: &Ask, game: &Game) -> String {
        let count = ask.quantity.get();
        let (verb, total) = match ask.side {
            Side::Buy => ("Buy", game.buy_price(&ask.item, count)),
            Side::Sell => ("Sell", game.sell_price(&ask.item, count)),
        };
        let name = self.names.name(&ask.item);
        match total {
            Ok(price) => format!(
                "{verb} how many {name}? < {count} of {} > for {price} caps",
                ask.quantity.max()
            ),
            Err(error) => error.to_string(),
        }
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, game: &Game, side: Side, title: String) {
        let (stacks, menu) = match side {
            Side::Buy => (stock(game, &self.vendor), &self.buy),
            Side::Sell => (pack(game), &self.sell),
        };
        let focused = self.focus == side;
        let width = usize::from(area.width.saturating_sub(2));
        let height = usize::from(area.height.saturating_sub(2));
        let caps = game.caps();
        let lines: Vec<Line<'static>> = stacks
            .iter()
            .enumerate()
            .map(|(index, (item, count))| {
                let price = match side {
                    Side::Buy => game.buy_price(item, NonZeroU32::MIN),
                    Side::Sell => game.sell_price(item, NonZeroU32::MIN),
                }
                .ok();
                let worn = side == Side::Sell && game.is_equipped(item);
                let usable = match side {
                    Side::Buy => price.is_some_and(|price| price <= caps),
                    Side::Sell => price.is_some() && !(worn && *count == NonZeroU32::MIN),
                };
                let mut label = format!("{} x{count}", self.names.name(item));
                if worn {
                    label.push_str(" [worn]");
                }
                let price = price.map_or_else(|| "-".to_owned(), |price| price.to_string());
                row_line(
                    &label,
                    &price,
                    width,
                    menu.selected() == Some(index),
                    focused,
                    usable,
                )
            })
            .collect();
        let lines = if lines.is_empty() {
            let empty = match side {
                Side::Buy => "Nothing for sale.",
                Side::Sell => "You carry nothing.",
            };
            vec![Line::styled(empty, theme::dim())]
        } else {
            menu::window(lines, menu.selected(), height)
        };
        let border = if focused { theme::text() } else { theme::dim() };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(border)
            .title(title);
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }
}

/// The vendor's stock, in the order the panel lists it.
fn stock(game: &Game, vendor: &VendorId) -> Vec<Stack> {
    game.shop(vendor)
        .map(|shop| {
            shop.stock()
                .map(|(item, count)| (item.clone(), count))
                .collect()
        })
        .unwrap_or_default()
}

/// The stacks the player carries, in the order the panel lists them.
fn pack(game: &Game) -> Vec<Stack> {
    game.inventory()
        .stacks()
        .map(|(item, count)| (item.clone(), count))
        .collect()
}

/// A menu over `len` rows that keeps the old selection where it still exists, so
/// the cursor stays put after a trade empties or shrinks the list.
fn resized(menu: &Menu, len: usize) -> Menu {
    let mut next = Menu::new(len);
    if let Some(selected) = menu.selected() {
        next.pick(selected.min(len.saturating_sub(1)));
    }
    next
}

/// One row: the label on the left, the price on the right, cut with an ellipsis to
/// fit `width` columns.
fn row_line(
    label: &str,
    price: &str,
    width: usize,
    selected: bool,
    focused: bool,
    usable: bool,
) -> Line<'static> {
    let marker = if selected { "> " } else { "  " };
    let price_len = price.chars().count();
    let room = width.saturating_sub(marker.len() + price_len + 1);
    let label = if label.chars().count() > room {
        let mut cut: String = label.chars().take(room.saturating_sub(1)).collect();
        if room > 0 {
            cut.push('…');
        }
        cut
    } else {
        label.to_owned()
    };
    let gap = width.saturating_sub(marker.len() + label.chars().count() + price_len);
    let style = match (selected, focused, usable) {
        (true, true, _) => theme::selected(),
        (true, false, _) => theme::heading(),
        (false, _, true) => theme::text(),
        (false, _, false) => theme::dim(),
    };
    Line::styled(format!("{marker}{label}{}{price}", " ".repeat(gap)), style)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::OnceLock;

    use engine::inventory::ItemBook;
    use engine::quests::QuestBook;
    use engine::shared::{Effect, GameData, Scope, apply_all};
    use engine::trade::VendorBook;
    use engine::world::WorldBook;
    use proptest::prelude::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    /// Every item Chet stocks or the test hands the player.
    const ITEMS: [&str; 8] = [
        "9mm_round",
        "357_magnum_round",
        "22lr_round",
        "12_gauge_buckshot",
        "9mm_pistol",
        "varmint_rifle",
        "leather_armor",
        "stimpak",
    ];

    fn fixture() -> (Game, Arc<Names>) {
        static FIXTURE: OnceLock<(Game, Arc<Names>)> = OnceLock::new();
        FIXTURE
            .get_or_init(|| {
                let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
                let data = GameData::load(&dir, &Scope::Everywhere).expect("the data loads");
                let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("quests build"));
                let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
                let items = Arc::new(ItemBook::from_data(&data).expect("items build"));
                let vendors = Arc::new(VendorBook::from_data(&data).expect("vendors build"));
                let game = Game::with_world(quests, world)
                    .with_items(items)
                    .with_vendors(vendors)
                    .expect("every stocked item has a value");
                (game, Arc::new(Names::from_data(&data)))
            })
            .clone()
    }

    fn id(raw: &str) -> ItemId {
        ItemId::try_new(raw).expect("valid id")
    }

    fn chet() -> VendorId {
        VendorId::try_new("chet").expect("valid id")
    }

    /// The game with the player holding `caps` and `items`, and a trade open at Chet.
    fn shop(caps: f64, items: &[(&str, u32)]) -> (Game, Trade, Log) {
        let (mut game, names) = fixture();
        let mut effects = vec![Effect::Caps { amount: caps }];
        effects.extend(items.iter().map(|(item, count)| Effect::GiveItem {
            item: id(item),
            count: NonZeroU32::new(*count).expect("nonzero"),
        }));
        apply_all(&effects, &mut game).expect("the player is stocked");
        let trade = Trade::open(chet(), "Chet's general store".to_owned(), &game, names)
            .expect("Chet trades");
        (game, trade, Log::default())
    }

    /// Moves the cursor of `side` onto `item`.
    fn point_at(trade: &mut Trade, game: &mut Game, log: &mut Log, side: Side, item: &str) {
        let (key, stacks) = match side {
            Side::Buy => (Command::Left, stock(game, &trade.vendor)),
            Side::Sell => (Command::Right, pack(game)),
        };
        trade.update(key, game, log);
        let target = stacks
            .iter()
            .position(|(candidate, _)| candidate.as_str() == item)
            .expect("the item is listed");
        for _ in 0..stacks.len() {
            let menu = match side {
                Side::Buy => &trade.buy,
                Side::Sell => &trade.sell,
            };
            if menu.selected() == Some(target) {
                return;
            }
            trade.update(Command::Down, game, log);
        }
    }

    fn said(log: &Log) -> String {
        log.tail(200, 8)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn a_vendor_the_game_does_not_have_cannot_be_opened() {
        let (game, names) = fixture();
        let nobody = VendorId::try_new("nobody").expect("valid id");
        assert!(Trade::open(nobody, "Nobody".to_owned(), &game, names).is_none());
    }

    #[test]
    fn a_stack_of_one_is_bought_at_once() {
        let (mut game, mut trade, mut log) = shop(1000.0, &[]);
        let armor = id("leather_armor");
        let price = game.buy_price(&armor, NonZeroU32::MIN).expect("priced");
        point_at(&mut trade, &mut game, &mut log, Side::Buy, "leather_armor");
        trade.update(Command::Select, &mut game, &mut log);
        assert_eq!(game.item_count(&armor), 1);
        assert_eq!(game.shop(&chet()).expect("shop").count(&armor), 0);
        assert_eq!(game.caps(), 1000 - price);
        assert!(said(&log).contains("You buy 1"), "{}", said(&log));
    }

    #[test]
    fn a_bigger_stack_asks_how_many_and_buys_that_many() {
        let (mut game, mut trade, mut log) = shop(1000.0, &[]);
        let rounds = id("9mm_round");
        point_at(&mut trade, &mut game, &mut log, Side::Buy, "9mm_round");
        trade.update(Command::Select, &mut game, &mut log);
        assert!(trade.ask.is_some());
        assert_eq!(game.item_count(&rounds), 0);
        trade.update(Command::Right, &mut game, &mut log);
        trade.update(Command::Right, &mut game, &mut log);
        trade.update(Command::Select, &mut game, &mut log);
        assert!(trade.ask.is_none());
        assert_eq!(game.item_count(&rounds), 3);
        assert_eq!(game.shop(&chet()).expect("shop").count(&rounds), 57);
    }

    #[test]
    fn escape_closes_the_question_and_then_the_shop() {
        let (mut game, mut trade, mut log) = shop(1000.0, &[]);
        point_at(&mut trade, &mut game, &mut log, Side::Buy, "9mm_round");
        trade.update(Command::Select, &mut game, &mut log);
        assert_eq!(trade.update(Command::Back, &mut game, &mut log), Next::Stay);
        assert!(trade.ask.is_none());
        assert_eq!(
            trade.update(Command::Back, &mut game, &mut log),
            Next::Leave
        );
    }

    #[test]
    fn a_purchase_the_player_cannot_afford_changes_nothing() {
        let (mut game, mut trade, mut log) = shop(0.0, &[]);
        let pistol = id("9mm_pistol");
        point_at(&mut trade, &mut game, &mut log, Side::Buy, "9mm_pistol");
        trade.update(Command::Select, &mut game, &mut log);
        assert_eq!(game.item_count(&pistol), 0);
        assert_eq!(game.shop(&chet()).expect("shop").count(&pistol), 1);
        assert_eq!(game.caps(), 0);
        assert!(!said(&log).is_empty());
    }

    #[test]
    fn selling_moves_the_item_and_the_caps_the_other_way() {
        let (mut game, mut trade, mut log) = shop(0.0, &[("stimpak", 4)]);
        let stimpak = id("stimpak");
        let paid = game.sell_price(&stimpak, NonZeroU32::MIN).expect("priced");
        let shop_caps = game.shop(&chet()).expect("shop").caps();
        point_at(&mut trade, &mut game, &mut log, Side::Sell, "stimpak");
        trade.update(Command::Select, &mut game, &mut log);
        trade.update(Command::Select, &mut game, &mut log);
        assert_eq!(game.item_count(&stimpak), 3);
        assert_eq!(game.caps(), paid);
        assert_eq!(game.shop(&chet()).expect("shop").caps(), shop_caps - paid);
        assert!(said(&log).contains("You sell 1"), "{}", said(&log));
    }

    #[test]
    fn the_last_worn_copy_is_refused() {
        let (mut game, mut trade, mut log) = shop(0.0, &[("leather_armor", 1)]);
        let armor = id("leather_armor");
        game.equip(&armor).expect("armor can be worn");
        point_at(&mut trade, &mut game, &mut log, Side::Sell, "leather_armor");
        trade.update(Command::Select, &mut game, &mut log);
        assert_eq!(game.item_count(&armor), 1);
        assert!(said(&log).contains("equipped"), "{}", said(&log));
    }

    #[test]
    fn both_panels_show_prices_caps_and_the_question() {
        let (mut game, mut trade, mut log) = shop(100.0, &[("stimpak", 2)]);
        point_at(&mut trade, &mut game, &mut log, Side::Buy, "9mm_round");
        trade.update(Command::Select, &mut game, &mut log);
        let mut terminal = Terminal::new(TestBackend::new(100, 20)).expect("terminal");
        terminal
            .draw(|frame| trade.render(frame, frame.area(), &game))
            .expect("draws");
        let buffer = terminal.backend().buffer();
        let text: String = (0..20)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol())
            .collect();
        for expected in ["Chet's general store", "500 caps", "Your pack", "100 caps"] {
            assert!(text.contains(expected), "missing {expected:?}");
        }
        assert!(text.contains("how many"));
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
        fn no_key_sequence_creates_or_destroys_caps_or_items(
            commands in proptest::collection::vec(any_command(), 0..120),
        ) {
            let (mut game, mut trade, mut log) =
                shop(400.0, &[("stimpak", 10), ("9mm_round", 100), ("leather_armor", 1)]);
            game.equip(&id("leather_armor")).expect("armor can be worn");
            let totals = |game: &Game| -> Vec<u32> {
                let shop = game.shop(&chet()).expect("shop");
                ITEMS
                    .iter()
                    .map(|item| game.item_count(&id(item)) + shop.count(&id(item)))
                    .chain([game.caps() + shop.caps()])
                    .collect()
            };
            let before = totals(&game);
            for command in commands {
                trade.update(command, &mut game, &mut log);
                prop_assert_eq!(totals(&game), before.clone());
                prop_assert!(trade
                    .buy
                    .selected()
                    .is_none_or(|index| index < stock(&game, &chet()).len()));
                prop_assert!(trade
                    .sell
                    .selected()
                    .is_none_or(|index| index < pack(&game).len()));
            }
        }

        #[test]
        fn a_row_never_runs_past_the_panel(
            label in "[a-zA-Z0-9 ]{0,60}",
            price in 0u32..99_999,
            width in 12usize..80,
            selected in any::<bool>(),
            focused in any::<bool>(),
        ) {
            let line = row_line(&label, &price.to_string(), width, selected, focused, true);
            prop_assert!(line.to_string().chars().count() <= width);
        }
    }
}
