//! Headless game engine for fnv-terminal-ai-game.
//!
//! Everything the game decides lives here and nothing here prints, so the
//! terminal front end, tests and tooling can all drive the same code.
//!
//! Modules are named for game capabilities. Each feature owns its own state and
//! never imports another feature; `shared` holds the contracts they have in
//! common and `game` is the one place that wires them together.

pub mod character;
pub mod combat;
pub mod companions;
pub mod dialogue;
pub mod game;
pub mod inventory;
pub mod modding;
pub mod quests;
pub mod reputation;
pub mod saves;
pub mod shared;
pub mod world;
