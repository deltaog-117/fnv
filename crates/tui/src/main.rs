//! Terminal front end for fnv-terminal-ai-game.
//!
//! Kept thin on purpose: it parses arguments, loads configuration and hands
//! control to the screens. Every game rule lives in the `engine` crate.

mod character;
mod combat;
mod dialogue;
mod exploration;
mod inventory;
mod quests;
mod shared;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Placeholder until the first screen exists; it only proves the binary builds.
    println!("fnv-terminal-ai-game {}", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}
