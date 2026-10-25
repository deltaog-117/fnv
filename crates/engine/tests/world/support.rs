//! Builders shared by the world tests.

use std::sync::Arc;

use engine::shared::{EndingId, FlagId, NpcId};
use engine::world::{WorldBook, WorldState};

pub fn npc(text: &str) -> NpcId {
    NpcId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn flag(text: &str) -> FlagId {
    FlagId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn ending(text: &str) -> EndingId {
    EndingId::try_new(text.to_owned()).expect("literal is a valid id")
}

/// The roster the tests share: `doc` and `sunny` exist, `ghost` does not.
pub fn book() -> Arc<WorldBook> {
    Arc::new(WorldBook::new([&npc("doc"), &npc("sunny")]))
}

pub fn world() -> WorldState {
    WorldState::new(book())
}
