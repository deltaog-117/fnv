//! One fight: who is standing, whose turn it is, and what an attack does.

use std::num::NonZeroU32;

use thiserror::Error;

use super::damage::{Rules, damage_after_armor, damage_before_armor};
use super::hit::{Dice, HitModel};
use super::stats::{Multiplier, Percent, Points, Power};

/// What a fighter's weapon does, already validated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttackProfile {
    /// Action points one attack costs; at least 1, so a turn always runs out.
    pub ap_cost: NonZeroU32,
    /// The weapon's damage.
    pub base_damage: Points,
    /// Damage added to the base on a critical hit.
    pub crit_damage: Points,
    /// The skill multiplier, 0.5 to 1.
    pub skill: Multiplier,
    /// The weapon condition multiplier, 0.5 to 1.
    pub condition: Multiplier,
    /// The Strength or Unarmed bonus added after the multipliers.
    pub bonus: Points,
}

/// What the fight needs to know about a fighter. The numbers are read, never
/// changed: the fight tracks hit points and action points itself.
pub trait Combatant {
    /// Hit points at full health; a fighter cannot start dead.
    fn max_hp(&self) -> NonZeroU32;
    /// Action points at the start of each turn.
    fn max_ap(&self) -> u32;
    /// Damage threshold of the worn armor.
    fn dt(&self) -> Points;
    /// Damage resistance of the worn armor, in percent.
    fn dr(&self) -> Points;
    /// Chance that a hit is critical.
    fn crit_chance(&self) -> Percent;
    /// The weapon in hand.
    fn attack(&self) -> AttackProfile;
}

/// One of the two sides of a fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The player and their allies.
    Player,
    /// Whoever the player fights.
    Enemy,
}

/// A fighter's place in the fight; only [`Combat`] hands these out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FighterId(usize);

impl FighterId {
    /// The position the fighter was given at the start, from zero.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// Whether the fight goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Both sides still have a fighter standing.
    Ongoing,
    /// The other side has no fighter standing.
    Won(Side),
}

/// What one attack did to its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strike {
    /// The attack missed; the action points are spent anyway.
    Missed,
    /// The attack hit.
    Hit {
        /// Whether the hit was critical.
        critical: bool,
        /// Hit points taken off the target, after armor.
        damage: u32,
    },
}

/// The result of an attack that was carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    /// Who attacked.
    pub attacker: FighterId,
    /// Who was attacked.
    pub target: FighterId,
    /// What happened.
    pub strike: Strike,
    /// The target's hit points afterwards.
    pub target_hp: u32,
    /// Whether this attack took the target down.
    pub defeated: bool,
    /// The attacker's action points afterwards.
    pub ap_left: u32,
}

/// A move was refused; the fight is exactly as it was.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CombatError {
    /// A fight needs a fighter on each side to begin.
    #[error("a fight needs at least one fighter on each side")]
    NeedsBothSides,
    /// The fight is over.
    #[error("the fight is over")]
    Over,
    /// The id does not belong to this fight.
    #[error("no fighter {0}")]
    NoSuchFighter(usize),
    /// The target is already down.
    #[error("fighter {0} is already down")]
    TargetDown(usize),
    /// The target is on the attacker's side.
    #[error("fighter {0} is on the attacker's side")]
    FriendlyFire(usize),
    /// The active fighter cannot pay for the attack.
    #[error("the attack costs {needed} action points and only {left} are left")]
    NotEnoughAp {
        /// The weapon's cost.
        needed: u32,
        /// What the fighter has left this turn.
        left: u32,
    },
}

#[derive(Debug, Clone)]
struct Slot<C> {
    who: C,
    side: Side,
    hp: u32,
}

/// A fight between two sides.
#[derive(Debug, Clone)]
pub struct Combat<C, H> {
    fighters: Vec<Slot<C>>,
    rules: Rules,
    hit_model: H,
    active: usize,
    ap_left: u32,
    round: u32,
    status: Status,
}

impl<C: Combatant, H: HitModel<C>> Combat<C, H> {
    /// Starts a fight. The first fighter acts first, then the rest in the order
    /// given, and the round counter starts at 1.
    ///
    /// # Errors
    ///
    /// Returns [`CombatError::NeedsBothSides`] unless each side has a fighter.
    pub fn new(
        fighters: impl IntoIterator<Item = (Side, C)>,
        rules: Rules,
        hit_model: H,
    ) -> Result<Self, CombatError> {
        let fighters: Vec<Slot<C>> = fighters
            .into_iter()
            .map(|(side, who)| Slot {
                hp: who.max_hp().get(),
                side,
                who,
            })
            .collect();
        let has = |side| fighters.iter().any(|f| f.side == side);
        if !(has(Side::Player) && has(Side::Enemy)) {
            return Err(CombatError::NeedsBothSides);
        }
        let ap_left = fighters.first().map_or(0, |f| f.who.max_ap());
        Ok(Self {
            fighters,
            rules,
            hit_model,
            active: 0,
            ap_left,
            round: 1,
            status: Status::Ongoing,
        })
    }

    /// Whether the fight goes on or who won.
    #[must_use]
    pub fn status(&self) -> Status {
        self.status
    }

    /// The fighter whose turn it is.
    #[must_use]
    pub fn active(&self) -> FighterId {
        FighterId(self.active)
    }

    /// Action points the active fighter has left this turn.
    #[must_use]
    pub fn ap_left(&self) -> u32 {
        self.ap_left
    }

    /// The round, starting at 1; it counts up each time the order wraps around.
    #[must_use]
    pub fn round(&self) -> u32 {
        self.round
    }

    /// Every fighter's id, in turn order.
    pub fn ids(&self) -> impl Iterator<Item = FighterId> + use<C, H> {
        (0..self.fighters.len()).map(FighterId)
    }

    /// A fighter's side, if the id belongs to this fight.
    #[must_use]
    pub fn side(&self, id: FighterId) -> Option<Side> {
        self.fighters.get(id.0).map(|f| f.side)
    }

    /// A fighter's current hit points, if the id belongs to this fight.
    #[must_use]
    pub fn hp(&self, id: FighterId) -> Option<u32> {
        self.fighters.get(id.0).map(|f| f.hp)
    }

    /// The fighter's numbers, if the id belongs to this fight.
    #[must_use]
    pub fn combatant(&self, id: FighterId) -> Option<&C> {
        self.fighters.get(id.0).map(|f| &f.who)
    }

    /// Whether the fighter is still standing.
    #[must_use]
    pub fn is_standing(&self, id: FighterId) -> bool {
        self.hp(id).is_some_and(|hp| hp > 0)
    }

    /// The active fighter attacks `target`.
    ///
    /// The hit roll comes first; only a hit rolls for a critical, so a miss draws
    /// one roll and a hit draws two.
    ///
    /// # Errors
    ///
    /// Returns a [`CombatError`] when the fight is over, the target is unknown,
    /// down or on the attacker's side, or the attacker cannot pay the action
    /// points. Nothing changes in that case, and no dice are rolled.
    pub fn attack(
        &mut self,
        target: FighterId,
        power: Power,
        dice: &mut impl Dice,
    ) -> Result<Resolution, CombatError> {
        if self.status != Status::Ongoing {
            return Err(CombatError::Over);
        }
        let attacker = self.active;
        let victim = self
            .fighters
            .get(target.0)
            .ok_or(CombatError::NoSuchFighter(target.0))?;
        if victim.hp == 0 {
            return Err(CombatError::TargetDown(target.0));
        }
        if victim.side == self.fighters[attacker].side {
            return Err(CombatError::FriendlyFire(target.0));
        }
        let profile = self.fighters[attacker].who.attack();
        let needed = profile.ap_cost.get();
        if needed > self.ap_left {
            return Err(CombatError::NotEnoughAp {
                needed,
                left: self.ap_left,
            });
        }

        self.ap_left -= needed;
        let chance = self
            .hit_model
            .chance(&self.fighters[attacker].who, &victim.who);
        let strike = if roll(dice) <= chance.get() {
            let critical = roll(dice) <= self.fighters[attacker].who.crit_chance().get();
            let raw = damage_before_armor(&profile, power, critical);
            let through = damage_after_armor(&self.rules, raw, victim.who.dt(), victim.who.dr());
            Strike::Hit {
                critical,
                damage: whole_hit_points(through),
            }
        } else {
            Strike::Missed
        };

        let victim = &mut self.fighters[target.0];
        if let Strike::Hit { damage, .. } = strike {
            victim.hp = victim.hp.saturating_sub(damage);
        }
        let target_hp = victim.hp;
        self.status = self.standing_status();
        Ok(Resolution {
            attacker: FighterId(attacker),
            target,
            strike,
            target_hp,
            defeated: target_hp == 0,
            ap_left: self.ap_left,
        })
    }

    /// The active fighter passes. The next fighter still standing gets a full set of
    /// action points, and the round counts up when the order wraps around.
    ///
    /// # Errors
    ///
    /// Returns [`CombatError::Over`] when the fight is over.
    pub fn end_turn(&mut self) -> Result<(), CombatError> {
        if self.status != Status::Ongoing {
            return Err(CombatError::Over);
        }
        let count = self.fighters.len();
        // Both sides have someone standing while the fight goes on, so a next one exists.
        let next = (1..=count)
            .map(|step| (self.active + step) % count)
            .find(|&i| self.fighters[i].hp > 0)
            .unwrap_or(self.active);
        if next <= self.active {
            self.round += 1;
        }
        self.active = next;
        self.ap_left = self.fighters[next].who.max_ap();
        Ok(())
    }

    fn standing_status(&self) -> Status {
        let standing = |side| self.fighters.iter().any(|f| f.side == side && f.hp > 0);
        match (standing(Side::Player), standing(Side::Enemy)) {
            (true, false) => Status::Won(Side::Player),
            (false, true) => Status::Won(Side::Enemy),
            _ => Status::Ongoing,
        }
    }
}

/// A roll clamped into 1 to 100, so a faulty source cannot make an attack always
/// hit (0) or never miss.
fn roll(dice: &mut impl Dice) -> u8 {
    dice.percent().clamp(1, 100)
}

/// Damage that gets through at all costs at least one hit point, so a fight with
/// any hitting weapon ends. The cast saturates, and `through` is never negative.
fn whole_hit_points(through: f64) -> u32 {
    through.ceil() as u32
}
