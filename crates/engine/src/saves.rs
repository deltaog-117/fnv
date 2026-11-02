//! Versioned save games of the complete game state.
//!
//! A [`Save`] is a plain record of everything that changes as the player plays:
//! quests, the world, standing, the character, the pack, the shops, the clock, the
//! place, wounds, boosts and weapon wear. What the data files already say (items,
//! dialogue, maps) is not saved; loading starts from a game built from the data and
//! fills these facts back in, through the same checks play uses, so a file that
//! disagrees with the data is refused with a message naming the part at fault.
//!
//! This module holds only the record, its JSON form and the file handling. It does
//! not know the features: `game` converts between a [`Save`] and a running game.
//!
//! A save carries a version number. A file from a newer game is refused, and a file
//! from an older one is upgraded one version at a time before it is read; to change
//! the record, raise [`VERSION`] and add the step in `upgrade`.

mod error;
mod file;
mod record;

pub use error::SaveError;
pub use record::{
    Save, SavedCharacter, SavedDeferred, SavedEffect, SavedEnding, SavedFate, SavedFired, SavedNpc,
    SavedPack, SavedPhase, SavedQuest, SavedScore, SavedShop, SavedStanding, SavedWear, SavedWorld,
    VERSION,
};
