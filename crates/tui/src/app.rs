//! Composition root of the front end: the one place that knows every screen.
//!
//! [`App`] owns the game and the persistent log, decides which screen is active,
//! and draws the persistent layout around it. It reads no terminal and writes none
//! (the caller feeds it commands and hands it a frame), so tests drive it headless.

use std::sync::Arc;

use anyhow::Context;
use engine::dialogue::{Conversation, DialogueBook};
use engine::game::Game;
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{Effect, GameData, InventoryView, LocationId, QuestId, QuestStage, apply_all};
use engine::world::WorldBook;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::dialogue::{self, Talk};
use crate::exploration::{self, Exploration, Person};
use crate::quests;
use crate::shared::input::Command;
use crate::shared::layout::{self, MIN_HEIGHT, MIN_WIDTH};
use crate::shared::log::{self, Entry, Log};
use crate::shared::theme;

/// The quest a new game starts in, and the stage that opens it.
const OPENING_QUEST: &str = "ain_t_that_a_kick_in_the_head";
const OPENING_STAGE: u32 = 10;

enum Screen {
    Exploring(Exploration),
    Talking(Talk),
}

/// The whole front end's state.
pub struct App {
    game: Game,
    dialogue: Arc<DialogueBook>,
    place: String,
    people: Vec<Person>,
    screen: Screen,
    log: Log,
    running: bool,
}

impl App {
    /// A new game in `place`, with the opening quest started.
    ///
    /// # Errors
    ///
    /// When the place is unknown or the loaded data does not build into a game.
    pub fn new(data: &GameData, place: &LocationId) -> anyhow::Result<Self> {
        let location = data
            .locations
            .get(place)
            .with_context(|| format!("unknown starting place {place}"))?;
        let quests = Arc::new(QuestBook::new(data.quests.iter()).context("building the quests")?);
        let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
        let dialogue =
            Arc::new(DialogueBook::new(data.dialogue.iter()).context("building the dialogue")?);

        let items = Arc::new(ItemBook::from_data(data).context("building the item book")?);
        let mut game = Game::with_world(quests, world).with_items(items);
        let opening = Effect::SetStage {
            quest: QuestId::try_new(OPENING_QUEST).context("opening quest id")?,
            stage: QuestStage::new(OPENING_STAGE),
        };
        apply_all(&[opening], &mut game).context("starting the opening quest")?;

        // Buildings sit inside the settlement, so their residents are "here" too.
        let here =
            |npc_place: &LocationId| npc_place == place || location.children.contains(npc_place);
        let mut people: Vec<Person> = data
            .npcs
            .iter()
            .filter(|npc| npc.location_ids.iter().any(here))
            .filter_map(|npc| {
                // Most dialogue files are transcripts without a graph; only the
                // authored ones open.
                let dialogue_id = npc
                    .dialogue
                    .iter()
                    .find(|id| Conversation::open(Arc::clone(&dialogue), id).is_ok())?;
                Some(Person {
                    npc: npc.id.clone(),
                    name: npc.name.clone(),
                    dialogue: dialogue_id.clone(),
                })
            })
            .collect();
        people.sort_by(|a, b| a.name.cmp(&b.name));

        let mut log = Log::default();
        log.push(Entry::Note(format!(
            "A new game begins in {}.",
            location.name
        )));
        Ok(Self {
            game,
            dialogue,
            screen: Screen::Exploring(Exploration::new(location.name.clone(), people.clone())),
            place: location.name.clone(),
            people,
            log,
            running: true,
        })
    }

    /// False once the player has quit.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Applies one command to the active screen.
    pub fn update(&mut self, command: Command) {
        if command == Command::Quit {
            self.running = false;
            return;
        }
        match &mut self.screen {
            Screen::Exploring(scene) => match scene.update(command) {
                exploration::Next::Stay => {}
                exploration::Next::Quit => self.running = false,
                exploration::Next::Talk(index) => self.start_talk(index),
            },
            Screen::Talking(talk) => {
                if talk.update(command, &mut self.game, &mut self.log) == dialogue::Next::Leave {
                    self.screen = self.exploring();
                }
            }
        }
    }

    fn exploring(&self) -> Screen {
        Screen::Exploring(Exploration::new(self.place.clone(), self.people.clone()))
    }

    fn start_talk(&mut self, index: usize) {
        let Some(person) = self.people.get(index) else {
            return;
        };
        if let Err(error) = self.game.meet(&person.npc) {
            self.log.push(Entry::Note(error.to_string()));
            return;
        }
        match Talk::open(
            Arc::clone(&self.dialogue),
            &person.dialogue,
            person.name.clone(),
            &self.game,
        ) {
            Ok(talk) => self.screen = Screen::Talking(talk),
            Err(error) => self.log.push(Entry::Note(error.to_string())),
        }
    }

    /// Draws the persistent layout and the active screen inside it.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let Some(regions) = layout::regions(area) else {
            render_too_small(frame, area);
            return;
        };
        let active = self.game.quests().active().count();
        frame.render_widget(
            Paragraph::new(format!(
                " {} | Lv {} | {} caps | {active} active quest(s)",
                self.place,
                self.game.character().level().get(),
                self.game.caps(),
            ))
            .style(theme::heading()),
            regions.status,
        );
        let keys = match &self.screen {
            Screen::Exploring(scene) => {
                scene.render(frame, regions.scene);
                "Up/Down move | Enter choose | 1-9 pick | q quit"
            }
            Screen::Talking(talk) => {
                talk.render(frame, regions.scene);
                "Up/Down move | Enter say | 1-9 pick | Esc walk away | q quit"
            }
        };
        log::render(frame, regions.log, &self.log);
        quests::render(frame, regions.quests, &self.game);
        frame.render_widget(
            Paragraph::new(format!(" {keys}")).style(theme::dim()),
            regions.footer,
        );
    }
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let message = format!(
        "Terminal too small: {}x{}. Need at least {MIN_WIDTH}x{MIN_HEIGHT}.",
        area.width, area.height
    );
    frame.render_widget(
        Paragraph::new(Line::styled(message, theme::warning())).alignment(Alignment::Center),
        area,
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use engine::shared::Scope;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn app() -> App {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
        let place = LocationId::try_new("goodsprings").expect("valid id");
        let data = GameData::load(&dir, &Scope::Around(place.clone())).expect("data loads");
        App::new(&data, &place).expect("the app builds")
    }

    fn screen(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
        terminal.draw(|frame| app.render(frame)).expect("draws");
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

    #[test]
    fn the_first_frame_shows_the_place_the_people_the_quest_and_the_keys() {
        let text = screen(&app(), 100, 30);
        for expected in [
            "You are in Goodsprings",
            "Talk to Doc Mitchell",
            "Talk to Sunny Smiles",
            "Quit game",
            "Ain't That a Kick in the Head",
            "Up/Down move",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn the_status_line_shows_the_level_and_the_caps() {
        let text = screen(&app(), 100, 30);
        assert!(text.contains("Lv 1 | 0 caps"), "{text}");
    }

    #[test]
    fn the_layout_fits_the_smallest_supported_terminal() {
        let text = screen(&app(), MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("Talk to Doc Mitchell"));
        assert!(text.contains("q quit"));
    }

    #[test]
    fn a_tiny_terminal_gets_a_message_instead_of_a_broken_layout() {
        assert!(screen(&app(), 40, 10).contains("Terminal too small"));
    }

    #[test]
    fn talking_to_doc_mitchell_shows_his_menu_and_goodbye_returns_to_the_scene() {
        let mut app = app();
        let people: Vec<_> = app.people.iter().map(|p| p.name.clone()).collect();
        let doc = people
            .iter()
            .position(|n| n == "Doc Mitchell")
            .expect("Doc is here");
        app.update(Command::Pick(doc));
        let text = screen(&app, 100, 30);
        assert!(text.contains("You are talking to Doc Mitchell"), "{text}");
        assert!(text.contains("What town is this?"), "{text}");

        // "Goodbye" is the last start topic.
        let Screen::Talking(talk) = &app.screen else {
            panic!("expected a conversation");
        };
        let last = talk.choice_count() - 1;
        app.update(Command::Pick(last.min(8)));
        if last > 8 {
            panic!("goodbye is beyond the pick keys");
        }
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert!(screen(&app, 100, 30).contains("Goodbye"));
    }

    #[test]
    fn a_topic_said_goes_to_the_log_with_the_speaker() {
        let mut app = app();
        let doc = app
            .people
            .iter()
            .position(|p| p.name == "Doc Mitchell")
            .expect("Doc");
        app.update(Command::Pick(doc));
        app.update(Command::Pick(0));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Doc Mitchell:"), "{text}");
    }

    #[test]
    fn quit_stops_the_app_from_any_screen() {
        let mut app = app();
        app.update(Command::Pick(0));
        assert!(app.is_running());
        app.update(Command::Quit);
        assert!(!app.is_running());
    }

    #[test]
    fn escape_walks_away_from_a_conversation_without_quitting() {
        let mut app = app();
        app.update(Command::Pick(0));
        app.update(Command::Back);
        assert!(app.is_running());
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }
}
