//! Turn-based action-point combat with damage threshold and resistance.
//!
//! [`Combat`] is one fight between two sides. It reads each fighter's numbers
//! through the small [`Combatant`] trait and keeps only what changes during the
//! fight (hit points, the active fighter's action points, the round), so the
//! player's character and inventory stay owned by `game`, and tests can fight with
//! plain structs.
//!
//! The fight owns these laws and nothing else:
//!
//! - fighters act in the order they were given; the active fighter starts its turn
//!   with all its action points, and an attack spends the weapon's cost;
//! - a move is checked as a whole before anything changes: a refused attack costs
//!   no action points, and a refused move leaves the fight as it was;
//! - damage runs the curated formulas in order: weapon damage, then resistance,
//!   then threshold, and a hit that gets through at all costs at least 1 hit point;
//! - the fight is over as soon as one side has no fighter standing, and nothing
//!   can be done after;
//! - a shot needs the rounds in the weapon, and a refused shot costs neither rounds
//!   nor action points.
//!
//! [`enemy_turns`] plays the enemy side with one plain tactic. Hit chance is behind [`HitModel`] because the wiki gives no formula for it;
//! [`FlatHitChance`] is a labelled stand-in until the real model arrives. Chance is
//! rolled through [`Dice`], so a fight is deterministic for a given sequence of
//! rolls. Ammunition is modelled: a weapon with a [`Magazine`] loses rounds with
//! every shot, hit or miss, and [`Combat::reload`] refills it from the reserve.
//! Weapon condition, body-part targeting and weapon spread are not modelled.

mod ai;
mod damage;
mod fight;
mod hit;
mod stats;

pub use ai::enemy_turns;
pub use damage::{Rules, damage_after_armor, damage_before_armor};
pub use fight::{
    AttackProfile, Combat, CombatError, Combatant, FighterId, Magazine, Reload, Resolution, Rounds,
    Side, Status, Strike,
};
pub use hit::{Dice, FlatHitChance, HitModel};
pub use stats::{Multiplier, Percent, Points, Power, StatError};
