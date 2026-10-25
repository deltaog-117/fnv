//! The opponents' turn: a deliberately plain tactic, so the fight is fair to read.

use super::fight::{Combat, CombatError, Combatant, Resolution, Side};
use super::hit::{Dice, HitModel};
use super::stats::Power;

/// Plays turns for every fighter on the enemy side until it is a player-side
/// fighter's turn again, or the fight ends. Each enemy shoots the first
/// player-side fighter still standing while it can pay for the weapon, then ends
/// its turn. Returns what happened, in order.
///
/// It stops after at most one pass over the order: every attack spends at least one
/// action point and every pass hands the turn on, so it cannot loop forever.
pub fn enemy_turns<C, H>(combat: &mut Combat<C, H>, dice: &mut impl Dice) -> Vec<Resolution>
where
    C: Combatant,
    H: HitModel<C>,
{
    let mut log = Vec::new();
    while combat.status() == super::fight::Status::Ongoing
        && combat.side(combat.active()) == Some(Side::Enemy)
    {
        let target = combat
            .ids()
            .find(|&id| combat.side(id) == Some(Side::Player) && combat.is_standing(id));
        let mut attacked = target.map(|id| combat.attack(id, Power::Normal, dice));
        if let (Some(Err(CombatError::OutOfAmmo { .. })), Some(id)) = (&attacked, target)
            && combat.reload().is_ok()
        {
            // The reload shows in the log only as the action points it took.
            attacked = Some(combat.attack(id, Power::Normal, dice));
        }
        let attacked = attacked.and_then(Result::ok);
        match attacked {
            Some(resolution) => log.push(resolution),
            // Out of action points (or nobody to shoot): the turn passes. A refused
            // end_turn means the fight is over, which the loop condition then sees.
            None if combat.end_turn().is_err() => break,
            None => {}
        }
    }
    log
}
