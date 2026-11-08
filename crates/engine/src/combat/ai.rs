//! The computer's turns: a deliberately plain tactic, so the fight is fair to read.

use super::fight::{Combat, CombatError, Combatant, FighterId, Resolution, Side, Status};
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
    play(combat, dice, |combat| {
        combat.side(combat.active()) != Some(Side::Enemy)
    })
}

/// Plays turns for everyone but the player's lead, enemies and allies alike, until
/// it is the lead's turn again or the fight ends. Each fighter shoots the first
/// opponent still standing, which is the same tactic the enemies use alone. With
/// no lead named, it plays the enemy side only, like [`enemy_turns`].
///
/// It stops after at most one pass over the order, for the same reason.
pub fn computer_turns<C, H>(combat: &mut Combat<C, H>, dice: &mut impl Dice) -> Vec<Resolution>
where
    C: Combatant,
    H: HitModel<C>,
{
    let Some(lead) = combat.lead() else {
        return enemy_turns(combat, dice);
    };
    play(combat, dice, |combat| combat.active() == lead)
}

fn play<C, H>(
    combat: &mut Combat<C, H>,
    dice: &mut impl Dice,
    hands_over: impl Fn(&Combat<C, H>) -> bool,
) -> Vec<Resolution>
where
    C: Combatant,
    H: HitModel<C>,
{
    let mut log = Vec::new();
    while combat.status() == Status::Ongoing && !hands_over(combat) {
        let Some(own) = combat.side(combat.active()) else {
            break;
        };
        let target = first_standing(combat, own);
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

/// The first fighter standing on the other side from `own`.
fn first_standing<C, H>(combat: &Combat<C, H>, own: Side) -> Option<FighterId>
where
    C: Combatant,
    H: HitModel<C>,
{
    combat
        .ids()
        .find(|&id| combat.side(id).is_some_and(|side| side != own) && combat.is_standing(id))
}
