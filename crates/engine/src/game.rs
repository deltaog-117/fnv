//! Composition root: assembles each feature's state and implements the view traits
//! the shared evaluator reads. The only module allowed to know every feature.
//!
//! [`Game`] holds the state of each feature built so far: quests, the world, the
//! character, the inventory, the merchants and the wounds the player carries between fights. Features that do not
//! exist yet answer reads with the shared defaults and fail writes with
//! `NotBuilt`, so an effect aimed at them is never silently dropped. Quest stage
//! effects and consequence rules run here, because they need the whole state.

mod battle;
mod state;

pub use battle::{Arsenal, Battle, BattleError, Fighter, Outcome};
pub use state::{Deferred, Game};
