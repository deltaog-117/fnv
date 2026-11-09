//! Recruitable companions: who can join, and who is in the party.
//!
//! [`CompanionBook`] holds the roster read once from the data: each companion's NPC
//! and loadout, the perk they grant while with the player, and what makes them
//! refuse. [`Party`] is the people travelling with the player now.
//!
//! This feature owns these laws and nothing else:
//!
//! - the party never holds more than [`PARTY_LIMIT`] people, and never the same
//!   person twice;
//! - joining a full party, or one the person is already in, changes nothing;
//! - leaving puts the party back exactly as it was before the person joined.
//!
//! Whether a particular companion is willing (their reputation conditions, whether
//! their NPC is alive), the perk they grant and bringing them into a fight belong
//! to [`Game`](crate::game::Game). Loyalty, their own gear and their quests are not
//! built.

mod book;
mod party;

pub use book::{CompanionBook, CompanionError};
pub use party::{PARTY_LIMIT, Party, PartyError};
