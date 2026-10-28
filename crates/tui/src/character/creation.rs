//! Making the character before play: the S.P.E.C.I.A.L. spread, three tagged
//! skills and up to two traits, each in its own panel.

use engine::character::{
    Character, CharacterRules, MAX_TRAITS, PerkBook, START_SPECIAL_POINTS, SpecialSet, TAG_BONUS,
    TAG_COUNT,
};
use engine::shared::{CharacterView, Skill, Special, SpecialLevel, TraitId};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::shared::input::Command;
use crate::shared::menu::{self, Menu};
use crate::shared::theme;

/// The panels, in the order the cursor visits them.
const PANELS: [Panel; 4] = [Panel::Special, Panel::Skills, Panel::Traits, Panel::Begin];

/// The attributes in the order the panel lists them.
const STATS: [Special; 7] = [
    Special::Strength,
    Special::Perception,
    Special::Endurance,
    Special::Charisma,
    Special::Intelligence,
    Special::Agility,
    Special::Luck,
];

/// The skills in the engine's own order.
fn skills() -> Vec<Skill> {
    Character::default()
        .skills()
        .map(|(skill, _)| skill)
        .collect()
}

/// Rows kept under the panels for the derived numbers and the trait description.
const DETAIL_HEIGHT: u16 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Special,
    Skills,
    Traits,
    Begin,
}

/// A trait as the panel lists it, copied once so drawing needs no lookups.
#[derive(Debug, Clone)]
struct TraitRow {
    id: TraitId,
    name: String,
    text: String,
    /// False when the game does not carry the trait's effect yet.
    modeled: bool,
}

/// The creation screen. It holds the choices and the cursor; the number of points
/// left is derived from the spread, so it cannot disagree with it.
///
/// Every command keeps the spread at 40 points or fewer and the tags and traits
/// within their limits, so once the player may begin, the engine accepts the
/// character.
#[derive(Debug)]
pub struct Creation {
    special: SpecialSet,
    tags: Vec<Skill>,
    traits: Vec<TraitId>,
    rows: Vec<TraitRow>,
    /// Which panel the cursor is on.
    panels: Menu,
    /// The row inside the panel being edited; `None` while only browsing panels.
    editing: Option<Menu>,
    notice: Option<String>,
}

impl Creation {
    /// A new character: a 5 in everything, nothing tagged, no trait.
    #[must_use]
    pub fn new(book: &PerkBook) -> Self {
        let rows = book
            .traits()
            .map(|record| TraitRow {
                id: record.id.clone(),
                name: record.name.clone(),
                text: [
                    record.benefits.as_deref().map(|text| format!("+ {text}")),
                    record.penalties.as_deref().map(|text| format!("- {text}")),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("  ")
                .trim()
                .to_owned(),
                modeled: !record.modifiers.is_empty(),
            })
            .collect();
        Self {
            special: SpecialSet::default(),
            tags: Vec::new(),
            traits: Vec::new(),
            rows,
            panels: Menu::new(PANELS.len()),
            editing: None,
            notice: None,
        }
    }

    /// Applies a command. Browsing moves between panels and Enter opens one;
    /// inside a panel the arrows change the selected row and Esc closes it. The
    /// character is returned once the player begins with valid choices.
    pub fn update(&mut self, command: Command, book: &PerkBook) -> Option<Character> {
        self.notice = None;
        if self.editing.is_some() {
            self.edit(command);
            None
        } else {
            self.browse(command, book)
        }
    }

    fn browse(&mut self, command: Command, book: &PerkBook) -> Option<Character> {
        match command {
            Command::Left => self.panels.up(),
            Command::Right => self.panels.down(),
            _ => {
                let panel = self.panels.handle(command).map(|index| PANELS[index])?;
                match panel {
                    Panel::Special => self.open(STATS.len()),
                    Panel::Skills => self.open(skills().len()),
                    Panel::Traits => self.open(self.rows.len()),
                    Panel::Begin => return self.begin(book),
                }
            }
        }
        None
    }

    fn open(&mut self, rows: usize) {
        self.editing = Some(Menu::new(rows));
    }

    fn edit(&mut self, command: Command) {
        let panel = PANELS[self.panels.selected().unwrap_or(0)];
        match command {
            Command::Back => self.editing = None,
            Command::Left if panel == Panel::Special => self.adjust(false),
            Command::Right if panel == Panel::Special => self.adjust(true),
            Command::Left | Command::Right | Command::Quit => {}
            _ => {
                let chosen = self.editing.as_mut().and_then(|rows| rows.handle(command));
                if chosen.is_some() {
                    match panel {
                        Panel::Special => self.adjust(true),
                        Panel::Skills => self.toggle_tag(),
                        Panel::Traits => self.toggle_trait(),
                        Panel::Begin => {}
                    }
                }
            }
        }
    }

    /// The row the cursor is on inside the open panel.
    fn row(&self) -> Option<usize> {
        self.editing.as_ref().and_then(Menu::selected)
    }

    /// Points not yet placed. The spread never goes above the total, so the
    /// subtraction cannot underflow.
    fn points_left(&self) -> u32 {
        START_SPECIAL_POINTS.saturating_sub(self.special.total())
    }

    fn adjust(&mut self, up: bool) {
        let Some(stat) = self.row().and_then(|row| STATS.get(row).copied()) else {
            return;
        };
        let now = u32::from(self.special.get(stat).get());
        let next = if up {
            if self.points_left() == 0 {
                self.notice = Some("No points left; lower another attribute first.".to_owned());
                return;
            }
            now + 1
        } else {
            now.saturating_sub(1)
        };
        match SpecialLevel::try_new(next) {
            Ok(level) => self.special = self.special.with(stat, level),
            Err(_) => {
                self.notice = Some(format!(
                    "{} stays between {} and {}.",
                    stat.name(),
                    SpecialLevel::MIN,
                    SpecialLevel::MAX
                ));
            }
        }
    }

    fn toggle_tag(&mut self) {
        let Some(skill) = self.row().and_then(|row| skills().get(row).copied()) else {
            return;
        };
        if let Some(at) = self.tags.iter().position(|tag| *tag == skill) {
            self.tags.remove(at);
        } else if self.tags.len() < TAG_COUNT {
            self.tags.push(skill);
        } else {
            self.notice = Some(format!("Only {TAG_COUNT} skills can be tagged; untag one."));
        }
    }

    fn toggle_trait(&mut self) {
        let Some(id) = self
            .row()
            .and_then(|row| self.rows.get(row))
            .map(|row| row.id.clone())
        else {
            return;
        };
        if let Some(at) = self.traits.iter().position(|chosen| *chosen == id) {
            self.traits.remove(at);
        } else if self.traits.len() < MAX_TRAITS {
            self.traits.push(id);
        } else {
            self.notice = Some(format!("Only {MAX_TRAITS} traits can be chosen; drop one."));
        }
    }

    /// What is still missing before the player may begin, or `None` when nothing is.
    fn missing(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.points_left() > 0 {
            parts.push(format!("spend {} more point(s)", self.points_left()));
        }
        if self.tags.len() < TAG_COUNT {
            parts.push(format!("tag {} more skill(s)", TAG_COUNT - self.tags.len()));
        }
        (!parts.is_empty()).then(|| parts.join(" and "))
    }

    fn begin(&mut self, book: &PerkBook) -> Option<Character> {
        if let Some(missing) = self.missing() {
            self.notice = Some(format!("Not yet: {missing}."));
            return None;
        }
        match Character::create(
            self.special,
            &self.tags,
            &self.traits,
            CharacterRules::standard(),
            book,
        ) {
            Ok(character) => Some(character),
            Err(error) => {
                self.notice = Some(error.to_string());
                None
            }
        }
    }

    /// Draws the three panels, the derived numbers and the begin button over the
    /// whole of `area`, with the keys on the last line.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let [title, body, details, foot] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(DETAIL_HEIGHT),
            Constraint::Length(1),
        ])
        .areas(area);
        let heading = match &self.notice {
            Some(notice) => Line::styled(format!(" {notice}"), theme::warning()),
            None => Line::styled(" Create your character", theme::heading()),
        };
        frame.render_widget(Paragraph::new(heading), title);

        let [special, skills, traits] = Layout::horizontal([
            Constraint::Percentage(32),
            Constraint::Percentage(36),
            Constraint::Percentage(32),
        ])
        .areas(body);
        let base = Character::new(self.special, CharacterRules::standard());
        self.panel(
            frame,
            special,
            Panel::Special,
            &format!(" S.P.E.C.I.A.L. ({} left) ", self.points_left()),
            self.special_lines(),
        );
        self.panel(
            frame,
            skills,
            Panel::Skills,
            &format!(" Skills (tagged {}/{TAG_COUNT}) ", self.tags.len()),
            self.skill_lines(&base),
        );
        self.panel(
            frame,
            traits,
            Panel::Traits,
            &format!(" Traits ({}/{MAX_TRAITS}) ", self.traits.len()),
            self.trait_lines(),
        );

        let [derived, begin] =
            Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)])
                .areas(details);
        self.render_derived(frame, derived, &base);
        self.render_begin(frame, begin);
        frame.render_widget(
            Paragraph::new(
                " Left/Right panel | Enter open or toggle | Left/Right change a value \
                 | Esc close | q quit",
            )
            .style(theme::dim()),
            foot,
        );
    }

    fn focused(&self, panel: Panel) -> bool {
        self.panels.selected().map(|index| PANELS[index]) == Some(panel)
    }

    /// A bordered panel; the one under the cursor is bright, and the one being
    /// edited shows its selected row.
    fn panel(
        &self,
        frame: &mut Frame,
        area: Rect,
        panel: Panel,
        title: &str,
        lines: Vec<Line<'static>>,
    ) {
        let focused = self.focused(panel);
        let style = if focused {
            theme::heading()
        } else {
            theme::dim()
        };
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title.to_owned());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let selected = self
            .editing
            .as_ref()
            .filter(|_| focused)
            .map(Menu::selected);
        let lines = menu::window(lines, selected.flatten(), usize::from(inner.height));
        frame.render_widget(Paragraph::new(lines), inner);
    }

    /// Rows of a panel: a plain list while browsing, with the cursor while editing.
    fn rows_of(&self, panel: Panel, labels: Vec<String>) -> Vec<Line<'static>> {
        match self.editing.as_ref().filter(|_| self.focused(panel)) {
            Some(cursor) => {
                let items: Vec<_> = labels.into_iter().map(menu::Item::new).collect();
                cursor.lines(&items)
            }
            None => labels
                .into_iter()
                .map(|label| Line::styled(format!("  {label}"), theme::text()))
                .collect(),
        }
    }

    fn special_lines(&self) -> Vec<Line<'static>> {
        let labels = STATS
            .iter()
            .map(|stat| format!("{:<13} {:>2}", stat.name(), self.special.get(*stat).get()))
            .collect();
        self.rows_of(Panel::Special, labels)
    }

    /// Skill levels from the attributes alone, plus the tag bonus: the traits that
    /// change a skill are not counted until the character is made.
    fn skill_lines(&self, base: &Character) -> Vec<Line<'static>> {
        let labels = skills()
            .into_iter()
            .map(|skill| {
                let tagged = self.tags.contains(&skill);
                let bonus = if tagged { TAG_BONUS.unsigned_abs() } else { 0 };
                let level = (u32::from(base.skill(skill).get()) + bonus).min(100);
                format!(
                    "[{}] {:<14} {level:>3}",
                    if tagged { 'x' } else { ' ' },
                    skill.name()
                )
            })
            .collect();
        self.rows_of(Panel::Skills, labels)
    }

    fn trait_lines(&self) -> Vec<Line<'static>> {
        let labels = self
            .rows
            .iter()
            .map(|row| {
                let chosen = self.traits.contains(&row.id);
                format!("[{}] {}", if chosen { 'x' } else { ' ' }, row.name)
            })
            .collect();
        self.rows_of(Panel::Traits, labels)
    }

    fn render_derived(&self, frame: &mut Frame, area: Rect, base: &Character) {
        let mut lines = vec![
            Line::styled(
                format!(
                    "HP {} | AP {} | Carry {} lb",
                    base.max_hp(),
                    base.action_points(),
                    base.carry_weight()
                ),
                theme::text(),
            ),
            Line::default(),
        ];
        let selected = self
            .row()
            .filter(|_| self.focused(Panel::Traits))
            .and_then(|row| self.rows.get(row));
        if let Some(row) = selected {
            if !row.modeled {
                lines.push(Line::styled(
                    "Not in the game yet; choosing it changes nothing.",
                    theme::warning(),
                ));
            }
            lines.push(Line::styled(row.text.clone(), theme::text()));
        } else if self
            .traits
            .iter()
            .any(|id| self.rows.iter().any(|row| row.modeled && &row.id == id))
        {
            lines.push(Line::styled(
                "Your traits adjust these numbers once you begin.",
                theme::dim(),
            ));
        }
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(theme::dim())
            .title(" Derived ");
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
    }

    fn render_begin(&self, frame: &mut Frame, area: Rect) {
        let ready = self.missing().is_none();
        let focused = self.focused(Panel::Begin);
        let label = match (focused, ready) {
            (true, true) => Line::styled("> Begin <", theme::selected()),
            (false, true) => Line::styled("  Begin", theme::text()),
            (_, false) => Line::styled("  Begin (locked)", theme::dim()),
        };
        let hint = self.missing().map_or_else(
            || Line::styled("Ready.", theme::text()),
            |missing| Line::styled(format!("To do: {missing}."), theme::warning()),
        );
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(if focused {
                theme::heading()
            } else {
                theme::dim()
            })
            .title(" Done ");
        frame.render_widget(
            Paragraph::new(vec![label, Line::default(), hint])
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::OnceLock;

    use engine::shared::{GameData, LocationId, Scope};
    use proptest::prelude::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    /// Loaded once: the property test builds a screen for every case.
    fn book() -> &'static PerkBook {
        static BOOK: OnceLock<PerkBook> = OnceLock::new();
        BOOK.get_or_init(|| {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
            let place = LocationId::try_new("goodsprings").expect("valid id");
            let data = GameData::load(&dir, &Scope::Around(place)).expect("data loads");
            PerkBook::base_game(&data)
        })
    }

    fn text(creation: &Creation, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
        terminal
            .draw(|frame| creation.render(frame, frame.area()))
            .expect("draws");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Opens the panel at `index`, from the first one.
    fn open_panel(creation: &mut Creation, book: &PerkBook, index: usize) {
        for _ in 0..index {
            creation.update(Command::Right, book);
        }
        creation.update(Command::Select, book);
    }

    fn command_strategy() -> impl Strategy<Value = Command> {
        prop_oneof![
            Just(Command::Up),
            Just(Command::Down),
            Just(Command::Left),
            Just(Command::Right),
            Just(Command::Select),
            Just(Command::Back),
            (0usize..9).prop_map(Command::Pick),
        ]
    }

    #[test]
    fn a_new_character_has_five_points_to_spend_and_cannot_begin() {
        let book = book();
        let mut creation = Creation::new(book);
        assert_eq!(creation.points_left(), 5);
        let frame = text(&creation, 100, 30);
        assert!(frame.contains("S.P.E.C.I.A.L. (5 left)"), "{frame}");
        assert!(frame.contains("Begin (locked)"), "{frame}");
        for _ in 0..3 {
            creation.update(Command::Right, book);
        }
        assert!(creation.update(Command::Select, book).is_none());
        let frame = text(&creation, 100, 30);
        assert!(frame.contains("Not yet: spend 5 more point(s)"), "{frame}");
    }

    #[test]
    fn the_arrows_change_an_attribute_inside_its_panel() {
        let book = book();
        let mut creation = Creation::new(book);
        creation.update(Command::Select, book);
        creation.update(Command::Right, book);
        assert_eq!(creation.special.get(Special::Strength).get(), 6);
        assert_eq!(creation.points_left(), 4);
        creation.update(Command::Left, book);
        creation.update(Command::Left, book);
        assert_eq!(creation.special.get(Special::Strength).get(), 4);
        assert_eq!(creation.points_left(), 6);
        creation.update(Command::Back, book);
        creation.update(Command::Right, book);
        assert!(creation.panels.selected() == Some(1));
    }

    #[test]
    fn an_attribute_stops_at_ten_and_at_one_with_the_reason() {
        let book = book();
        let mut creation = Creation::new(book);
        creation.update(Command::Select, book);
        for _ in 0..5 {
            creation.update(Command::Right, book);
        }
        assert_eq!(creation.special.get(Special::Strength).get(), 10);
        creation.update(Command::Right, book);
        assert_eq!(creation.special.get(Special::Strength).get(), 10);
        assert!(text(&creation, 100, 30).contains("No points left"));
        for _ in 0..12 {
            creation.update(Command::Left, book);
        }
        assert_eq!(creation.special.get(Special::Strength).get(), 1);
        assert!(text(&creation, 100, 30).contains("stays between 1 and 10"));
    }

    #[test]
    fn a_fourth_tag_and_a_third_trait_are_refused() {
        let book = book();
        let mut creation = Creation::new(book);
        open_panel(&mut creation, book, 1);
        for _ in 0..3 {
            creation.update(Command::Select, book);
            creation.update(Command::Down, book);
        }
        creation.update(Command::Select, book);
        assert_eq!(creation.tags.len(), TAG_COUNT);
        assert!(text(&creation, 100, 30).contains("untag one"));
        creation.update(Command::Back, book);
        creation.update(Command::Right, book);
        creation.update(Command::Select, book);
        for _ in 0..2 {
            creation.update(Command::Select, book);
            creation.update(Command::Down, book);
        }
        creation.update(Command::Select, book);
        assert_eq!(creation.traits.len(), MAX_TRAITS);
        assert!(text(&creation, 100, 30).contains("drop one"));
    }

    #[test]
    fn a_finished_character_begins_with_exactly_what_was_chosen() {
        let book = book();
        let mut creation = Creation::new(book);
        creation.update(Command::Select, book);
        for _ in 0..5 {
            creation.update(Command::Right, book);
        }
        creation.update(Command::Back, book);
        open_panel_from_panel(&mut creation, book);
        creation.update(Command::Back, book);
        creation.update(Command::Right, book);
        creation.update(Command::Right, book);
        let character = creation
            .update(Command::Select, book)
            .expect("ready to begin");
        assert_eq!(character.attributes().total(), START_SPECIAL_POINTS);
        assert_eq!(character.attributes().get(Special::Strength).get(), 10);
        assert!(character.is_tagged(Skill::Barter));
        assert!(character.is_tagged(Skill::EnergyWeapons));
        assert!(character.is_tagged(Skill::Explosives));
        assert_eq!(character.traits().count(), 0);
    }

    /// From the SPECIAL panel, opens the skills panel and tags the first three.
    fn open_panel_from_panel(creation: &mut Creation, book: &PerkBook) {
        creation.update(Command::Right, book);
        creation.update(Command::Select, book);
        for _ in 0..TAG_COUNT {
            creation.update(Command::Select, book);
            creation.update(Command::Down, book);
        }
    }

    #[test]
    fn the_screen_fits_the_smallest_terminal_with_every_panel_visible() {
        let book = book();
        let creation = Creation::new(book);
        let frame = text(&creation, 70, 22);
        for expected in [
            "S.P.E.C.I.A.L.",
            "Skills (tagged 0/3)",
            "Traits (0/2)",
            "Derived",
        ] {
            assert!(frame.contains(expected), "missing {expected:?}:\n{frame}");
        }
    }

    proptest! {
        /// Whatever the player presses, the spread never exceeds 40 points and the
        /// limits hold, and whenever the screen lets the player begin, the engine
        /// accepts the character.
        #[test]
        fn no_key_sequence_breaks_a_limit(
            commands in proptest::collection::vec(command_strategy(), 0..200),
        ) {
            let book = book();
            let mut creation = Creation::new(book);
            for command in commands {
                let made = creation.update(command, book);
                prop_assert!(creation.special.total() <= START_SPECIAL_POINTS);
                prop_assert!(creation.tags.len() <= TAG_COUNT);
                prop_assert!(creation.traits.len() <= MAX_TRAITS);
                if let Some(character) = made {
                    prop_assert_eq!(character.attributes().total(), START_SPECIAL_POINTS);
                    prop_assert_eq!(creation.missing(), None);
                    break;
                }
            }
        }
    }
}
