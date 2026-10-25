//! Write-side contracts: how an [`Effect`] changes the state that owns it.
//!
//! Each feature implements the sink for its own domain and nothing else. The
//! methods have no default body on purpose: an effect that nobody handles must be
//! a compile error, not a silent no-op.
//!
//! [`Effect`]: super::Effect

use std::num::NonZeroU32;

use thiserror::Error;

use super::ids::{CompanionId, EndingId, FactionId, FlagId, ItemId, NpcId, QuestId};
use super::view::StateView;
use super::vocabulary::{QuestStage, ReputationLevel};

/// An effect could not be applied.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum EffectError {
    /// The effect names something the state does not have.
    #[error("unknown {kind} {id:?}")]
    Unknown {
        /// What kind of thing was named (for example `"quest"`).
        kind: &'static str,
        /// The id that was not found.
        id: String,
    },
    /// The player holds fewer of an item than the effect takes.
    #[error("cannot take {need} of {item:?}: only {have} held")]
    NotEnoughItems {
        /// The item.
        item: ItemId,
        /// How many the player holds.
        have: u32,
        /// How many the effect takes.
        need: u32,
    },
    /// The player holds fewer caps than the effect removes.
    #[error("cannot change caps by {change}: only {have} held")]
    NotEnoughCaps {
        /// How many caps the player holds.
        have: u32,
        /// The requested change, negative when caps are removed.
        change: f64,
    },
    /// An amount was NaN or infinite.
    #[error("effect {effect} carries an amount that is not a finite number")]
    NotFinite {
        /// The `type` value of the effect.
        effect: &'static str,
    },
    /// The feature that owns the effect has not been built yet. Kept apart from
    /// [`EffectError::Refused`] so a caller can postpone these (a quest reward whose
    /// feature has not landed) while still failing on every real refusal.
    #[error("effect {effect} cannot run yet: the {feature} feature is not built")]
    NotBuilt {
        /// The `type` value of the effect.
        effect: &'static str,
        /// The missing feature.
        feature: &'static str,
    },
    /// The owning feature refused the effect for a reason of its own.
    #[error("effect {effect} refused: {reason}")]
    Refused {
        /// The `type` value of the effect.
        effect: &'static str,
        /// Why the owner refused.
        reason: String,
    },
}

/// Reads an effect amount (XP, caps) as a whole number.
///
/// Quest data writes amounts as floats, but caps and XP are counted in ones, so a
/// fraction is a data mistake to refuse rather than round away.
///
/// # Errors
///
/// [`EffectError::NotFinite`] for NaN or infinity, and [`EffectError::Refused`]
/// for a fraction or a number too large to count.
pub fn whole_amount(effect: &'static str, amount: f64) -> Result<i64, EffectError> {
    // 2^53: every whole number up to here is exact in an f64 and fits an i64.
    const LIMIT: f64 = 9_007_199_254_740_992.0;
    if !amount.is_finite() {
        return Err(EffectError::NotFinite { effect });
    }
    if amount.fract() != 0.0 || amount.abs() > LIMIT {
        return Err(EffectError::Refused {
            effect,
            reason: format!("{amount} is not a whole number that can be counted"),
        });
    }
    // Checked above: whole, and within the range an i64 holds exactly.
    Ok(amount as i64)
}

/// An effect in a batch failed. Effects before `index` were applied.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("effect {index} ({tag}) failed after {applied} applied: {source}")]
pub struct BatchError {
    /// Position of the failing effect in the batch.
    pub index: usize,
    /// The `type` value of the failing effect.
    pub tag: &'static str,
    /// How many effects were applied before the failure: zero when the pre-flight
    /// check rejected the batch.
    pub applied: usize,
    /// What went wrong.
    #[source]
    pub source: EffectError,
}

/// Quest effects.
pub trait QuestSink {
    /// Starts a quest.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the quest is unknown or cannot start.
    fn start_quest(&mut self, quest: &QuestId) -> Result<(), EffectError>;

    /// Completes a quest.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the quest is unknown or cannot complete.
    fn complete_quest(&mut self, quest: &QuestId) -> Result<(), EffectError>;

    /// Fails a quest.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the quest is unknown.
    fn fail_quest(&mut self, quest: &QuestId) -> Result<(), EffectError>;

    /// Locks a quest out so it can never start.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the quest is unknown.
    fn block_quest(&mut self, quest: &QuestId) -> Result<(), EffectError>;

    /// Moves a quest to a stage.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the quest or the stage is unknown.
    fn set_stage(&mut self, quest: &QuestId, stage: QuestStage) -> Result<(), EffectError>;
}

/// Reputation and karma effects. The amounts are finite; the dispatcher checks.
pub trait StandingSink {
    /// Adds fame and infamy points with a faction.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the faction is unknown.
    fn add_reputation(
        &mut self,
        faction: &FactionId,
        fame: f64,
        infamy: f64,
    ) -> Result<(), EffectError>;

    /// Forces the standing with a faction to a level.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the faction is unknown.
    fn set_reputation(
        &mut self,
        faction: &FactionId,
        level: ReputationLevel,
    ) -> Result<(), EffectError>;

    /// Changes karma; negative amounts subtract.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the owner refuses the change.
    fn add_karma(&mut self, amount: f64) -> Result<(), EffectError>;
}

/// Character progression effects.
pub trait CharacterSink {
    /// Grants experience points.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the owner refuses the grant.
    fn add_xp(&mut self, amount: f64) -> Result<(), EffectError>;
}

/// Inventory effects.
pub trait InventorySink {
    /// Changes the number of bottle caps; negative amounts subtract.
    ///
    /// # Errors
    ///
    /// [`EffectError::NotEnoughCaps`] when the player cannot pay.
    fn add_caps(&mut self, amount: f64) -> Result<(), EffectError>;

    /// Gives the player items.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the item is unknown.
    fn give_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError>;

    /// Takes items from the player.
    ///
    /// # Errors
    ///
    /// [`EffectError::NotEnoughItems`] when the player holds too few.
    fn take_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError>;
}

/// Companion effects.
pub trait CompanionSink {
    /// A companion joins the player.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the companion is unknown or cannot join.
    fn companion_join(&mut self, companion: &CompanionId) -> Result<(), EffectError>;

    /// A companion leaves the player.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the companion is unknown.
    fn companion_leave(&mut self, companion: &CompanionId) -> Result<(), EffectError>;
}

/// Story flag, NPC and ending effects.
pub trait WorldSink {
    /// Sets or clears a story flag.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the owner refuses the flag.
    fn set_flag(&mut self, flag: &FlagId, value: bool) -> Result<(), EffectError>;

    /// Kills a non-player character.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is unknown.
    fn kill_npc(&mut self, npc: &NpcId) -> Result<(), EffectError>;

    /// Makes a non-player character hostile.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is unknown.
    fn set_hostile(&mut self, npc: &NpcId) -> Result<(), EffectError>;

    /// Makes a non-player character friendly.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is unknown.
    fn set_friendly(&mut self, npc: &NpcId) -> Result<(), EffectError>;

    /// Removes a non-player character from the world.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is unknown.
    fn remove_npc(&mut self, npc: &NpcId) -> Result<(), EffectError>;

    /// Makes an ending reachable.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the ending is unknown.
    fn unlock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError>;

    /// Makes an ending unreachable.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the ending is unknown.
    fn lock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError>;
}

/// Everything an effect can change. Implemented for any type that implements the
/// six domain sinks.
pub trait StateSink:
    QuestSink + StandingSink + CharacterSink + InventorySink + CompanionSink + WorldSink
{
}

impl<T> StateSink for T where
    T: QuestSink
        + StandingSink
        + CharacterSink
        + InventorySink
        + CompanionSink
        + WorldSink
        + ?Sized
{
}

/// A state that can be both read and changed, as `apply_all` needs.
pub trait State: StateView + StateSink {}

impl<T: StateView + StateSink + ?Sized> State for T {}
