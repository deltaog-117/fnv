//! Terminal front end for fnv-terminal-ai-game.
//!
//! Kept thin on purpose: it parses arguments, loads configuration and hands
//! control to the screens. Every game rule lives in the `engine` crate.

mod app;
mod character;
mod combat;
mod dialogue;
mod exploration;
mod inventory;
mod quests;
mod shared;
mod trade;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use engine::shared::{GameData, LocationId, Scope};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use crate::app::App;
use crate::shared::input;

/// Where the engine data lives when no path is given (run from the project root).
const DEFAULT_DATA_DIR: &str = "data/engine";
/// The only playable area for now.
const START_PLACE: &str = "goodsprings";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    let dir = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from(DEFAULT_DATA_DIR), PathBuf::from);
    let place = LocationId::try_new(START_PLACE).context("starting place id")?;
    let data = GameData::load(&dir, &Scope::Around(place.clone()))
        .with_context(|| format!("loading the game data from {}", dir.display()))?;
    let mut app = App::new(&data, &place)?.with_creation();

    // `init` also installs a panic hook that restores the terminal, so a crash does
    // not leave the player's shell in raw mode.
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> anyhow::Result<()> {
    while app.is_running() {
        terminal.draw(|frame| app.render(frame))?;
        // A resize arrives as an event too, so the next iteration redraws it.
        if let Event::Key(key) = event::read()?
            && let Some(command) = input::command(key)
        {
            app.update(command);
        }
    }
    Ok(())
}
