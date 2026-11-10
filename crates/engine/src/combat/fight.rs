//! One fight: who is standing, whose turn it is, and what an attack does.

use std::num::NonZeroU32;

use thiserror::Error;

use super::damage::{Against, Rules, damage_after_armor, damage_before_armor};
use super::hit::{Aim, Dice, HitModel};
use super::stats::{Multiplier, Percent, Points, Power};
use crate::shared::FoeKind;

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

/// A weapon's magazine and the rounds that go with it, already validated.
///
/// Only weapons that fire ammunition have one; a fighter without one never runs dry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Magazine {
    /// Rounds the weapon holds when loaded.
    pub clip: NonZeroU32,
    /// Rounds one attack uses; at least 1, so shooting always empties the weapon.
    pub per_shot: NonZeroU32,
    /// Action points a reload costs; at least 1, like an attack.
    pub reload_ap: NonZeroU32,
    /// Every round carried for this weapon, loaded ones included. `None` is an
    /// endless supply, for foes whose pack the game does not track.
    pub stock: Option<u32>,
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
    /// The weapon's magazine, or `None` for a weapon that needs no ammunition.
    fn magazine(&self) -> Option<Magazine> {
        None
    }
    /// How well the fighter places a shot; only a model that reads it cares.
    fn aim(&self) -> Aim {
        Aim::UNTRAINED
    }
    /// Decides who acts first: the higher number goes earlier, and equal numbers
    /// keep the order the fighters were given in.
    fn sequence(&self) -> u32 {
        0
    }
    /// The chance that running away works. A fighter that cannot run says never.
    fn flee_chance(&self) -> Percent {
        Percent::NEVER
    }
    /// What sort of creature this is, for attackers that hit one sort harder.
    fn kind(&self) -> FoeKind {
        FoeKind::Human
    }
    /// What this fighter's attacks gain against a foe of `kind`.
    fn against(&self, _kind: FoeKind) -> Against {
        Against::NONE
    }
    /// Points of the target's damage threshold this fighter's attacks ignore.
    fn ignored_threshold(&self) -> Points {
        Points::ZERO
    }
    /// Percent points added to the chance that a hit on this fighter is critical,
    /// whoever strikes; -100 makes them immune. The sum is kept between 0 and 100.
    fn crit_chance_against(&self) -> i32 {
        0
    }
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
    /// The player's side got away. Nobody won: the foes are still up.
    Fled,
}

/// What an attempt to run away came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flight {
    /// The player got away and the fight is over.
    Escaped,
    /// The foes blocked the way; the rest of the turn is lost.
    Blocked,
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

/// The rounds a fighter has: the ones in the weapon and the ones still carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rounds {
    /// Rounds in the weapon now.
    pub loaded: u32,
    /// Rounds carried besides those; `None` is an endless supply.
    pub reserve: Option<u32>,
}

impl Rounds {
    /// Every round the fighter still has; `None` for an endless supply.
    #[must_use]
    pub fn total(self) -> Option<u32> {
        self.reserve.map(|reserve| self.loaded + reserve)
    }
}

/// What a reload did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reload {
    /// Rounds moved from the reserve into the weapon.
    pub added: u32,
    /// Rounds in the weapon afterwards.
    pub loaded: u32,
    /// The reloader's action points afterwards.
    pub ap_left: u32,
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
    /// Only the player's side can run away.
    #[error("only the player's side can run away")]
    OnlyThePlayerFlees,
    /// The target is on the attacker's side.
    #[error("fighter {0} is on the attacker's side")]
    FriendlyFire(usize),
    /// Only a fighter on the player's side can lead it.
    #[error("fighter {0} is on the enemy side and cannot lead the player's")]
    LeadOnTheEnemySide(usize),
    /// A fighter cannot start with more hit points than its maximum.
    #[error("fighter {id} cannot start with {hp} hit points; its maximum is {max}")]
    HpAboveMax {
        /// The fighter.
        id: usize,
        /// The hit points asked for.
        hp: u32,
        /// The fighter's maximum.
        max: u32,
    },
    /// A fighter's replacement must keep the maximum hit points, or the hit points
    /// it has left would mean something else.
    #[error("fighter {id} has {was} hit points at most and the replacement {now}")]
    MaxHpChanged {
        /// The fighter.
        id: usize,
        /// The maximum it had.
        was: u32,
        /// The maximum the replacement has.
        now: u32,
    },
    /// The weapon holds fewer rounds than one attack uses.
    #[error("the weapon holds {loaded} rounds and an attack uses {needed}")]
    OutOfAmmo {
        /// What one attack uses.
        needed: u32,
        /// What is in the weapon.
        loaded: u32,
    },
    /// The weapon has no magazine, is already full, or no rounds are left to load.
    #[error("there is nothing to reload")]
    NothingToReload,
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
    /// `None` for a weapon that needs no ammunition.
    rounds: Option<Rounds>,
}

/// A fight between two sides.
#[derive(Debug, Clone)]
pub struct Combat<C, H> {
    fighters: Vec<Slot<C>>,
    rules: Rules,
    hit_model: H,
    /// Fighter indexes in the order they act: the highest sequence first.
    turn_order: Vec<usize>,
    active: usize,
    ap_left: u32,
    round: u32,
    status: Status,
    /// The player-side fighter whose fall loses the fight for their side, even
    /// while allies stand. `None` means a side loses when all of it has fallen.
    lead: Option<usize>,
}

impl<C: Combatant, H: HitModel<C>> Combat<C, H> {
    /// Starts a fight. The fighter with the highest sequence acts first, then the
    /// rest by falling sequence; fighters with the same sequence keep the order they
    /// were given in. The round counter starts at 1.
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
                rounds: who.magazine().map(starting_rounds),
                who,
            })
            .collect();
        let has = |side| fighters.iter().any(|f| f.side == side);
        if !(has(Side::Player) && has(Side::Enemy)) {
            return Err(CombatError::NeedsBothSides);
        }
        let mut turn_order: Vec<usize> = (0..fighters.len()).collect();
        // A stable sort, so equal sequences keep the order given.
        turn_order.sort_by_key(|&i| std::cmp::Reverse(fighters[i].who.sequence()));
        let active = turn_order.first().copied().unwrap_or(0);
        let ap_left = fighters.get(active).map_or(0, |f| f.who.max_ap());
        Ok(Self {
            fighters,
            rules,
            hit_model,
            turn_order,
            active,
            ap_left,
            round: 1,
            status: Status::Ongoing,
            lead: None,
        })
    }

    /// Names the fighter whose fall loses the fight for the player's side, so allies
    /// who are still standing cannot go on without them. The same fighter is the one
    /// a person plays: everyone else on either side is played by the computer.
    ///
    /// # Errors
    ///
    /// [`CombatError::NoSuchFighter`] for an unknown id and
    /// [`CombatError::LeadOnTheEnemySide`] for a fighter on the enemy side.
    pub fn with_lead(mut self, id: FighterId) -> Result<Self, CombatError> {
        match self.fighters.get(id.0).map(|slot| slot.side) {
            None => Err(CombatError::NoSuchFighter(id.0)),
            Some(Side::Enemy) => Err(CombatError::LeadOnTheEnemySide(id.0)),
            Some(Side::Player) => {
                self.lead = Some(id.0);
                self.status = self.standing_status();
                Ok(self)
            }
        }
    }

    /// The fighter whose fall loses the fight for the player's side, if one is named.
    #[must_use]
    pub fn lead(&self) -> Option<FighterId> {
        self.lead.map(FighterId)
    }

    /// Starts `id` with `hp` hit points instead of its maximum, for a fighter who
    /// carries wounds into the fight. Zero is not allowed: a fighter cannot start
    /// down, which would let the fight be over before it begins.
    ///
    /// # Errors
    ///
    /// [`CombatError::NoSuchFighter`] for an unknown id, and
    /// [`CombatError::HpAboveMax`] when `hp` is more than the fighter's maximum.
    pub fn with_hp(mut self, id: FighterId, hp: NonZeroU32) -> Result<Self, CombatError> {
        let slot = self
            .fighters
            .get_mut(id.0)
            .ok_or(CombatError::NoSuchFighter(id.0))?;
        let max = slot.who.max_hp().get();
        if hp.get() > max {
            return Err(CombatError::HpAboveMax {
                id: id.0,
                hp: hp.get(),
                max,
            });
        }
        slot.hp = hp.get();
        Ok(self)
    }

    /// Swaps the numbers `id` fights with, for a fighter whose perks switch on or off
    /// as the fight goes (Nerd Rage below a fifth of their hit points).
    ///
    /// Hit points, rounds in the weapon and the turn in progress are kept: the
    /// action points left this turn are not topped up, the new maximum applies from
    /// the next turn, and the order of play does not change.
    ///
    /// # Errors
    ///
    /// [`CombatError::NoSuchFighter`] for an unknown id, and
    /// [`CombatError::MaxHpChanged`] when the replacement has a different maximum;
    /// the fight is unchanged in both cases.
    pub fn replace_combatant(&mut self, id: FighterId, who: C) -> Result<(), CombatError> {
        let slot = self
            .fighters
            .get_mut(id.0)
            .ok_or(CombatError::NoSuchFighter(id.0))?;
        let (was, now) = (slot.who.max_hp().get(), who.max_hp().get());
        if was != now {
            return Err(CombatError::MaxHpChanged { id: id.0, was, now });
        }
        slot.who = who;
        Ok(())
    }

    /// Swaps the numbers `id` fights with and sets its hit points, for a fighter whose
    /// maximum can change in the middle of a fight (a chem that raises Endurance, or
    /// a heal). Rounds and the turn in progress are kept.
    ///
    /// Unlike [`Combat::replace_combatant`] the maximum may differ, so the hit points
    /// are given with it: `hp` is never zero (a fighter is not raised from the dead
    /// or struck down this way) and never above the new maximum.
    ///
    /// # Errors
    ///
    /// [`CombatError::Over`] when the fight is over, [`CombatError::NoSuchFighter`]
    /// for an unknown id, [`CombatError::TargetDown`] for a fighter already down and
    /// [`CombatError::HpAboveMax`] when `hp` is more than the new maximum; the fight
    /// is unchanged in all of them.
    pub fn retune(&mut self, id: FighterId, who: C, hp: NonZeroU32) -> Result<(), CombatError> {
        if self.status != Status::Ongoing {
            return Err(CombatError::Over);
        }
        let slot = self
            .fighters
            .get_mut(id.0)
            .ok_or(CombatError::NoSuchFighter(id.0))?;
        if slot.hp == 0 {
            return Err(CombatError::TargetDown(id.0));
        }
        let max = who.max_hp().get();
        if hp.get() > max {
            return Err(CombatError::HpAboveMax {
                id: id.0,
                hp: hp.get(),
                max,
            });
        }
        slot.who = who;
        slot.hp = hp.get();
        Ok(())
    }

    /// The active fighter spends `cost` action points on something that is not an
    /// attack, such as using an item.
    ///
    /// # Errors
    ///
    /// [`CombatError::Over`] when the fight is over and
    /// [`CombatError::NotEnoughAp`] when the fighter has too few left; nothing
    /// changes in either case.
    pub fn spend_ap(&mut self, cost: u32) -> Result<(), CombatError> {
        if self.status != Status::Ongoing {
            return Err(CombatError::Over);
        }
        self.ap_left = self
            .ap_left
            .checked_sub(cost)
            .ok_or(CombatError::NotEnoughAp {
                needed: cost,
                left: self.ap_left,
            })?;
        Ok(())
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

    /// The fighter's rounds, if the weapon uses ammunition.
    #[must_use]
    pub fn rounds(&self, id: FighterId) -> Option<Rounds> {
        self.fighters.get(id.0).and_then(|f| f.rounds)
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
        let profile = self.fighters[attacker]
            .who
            .against(victim.who.kind())
            .applied(&profile);
        let needed = profile.ap_cost.get();
        if needed > self.ap_left {
            return Err(CombatError::NotEnoughAp {
                needed,
                left: self.ap_left,
            });
        }

        let per_shot = self.fighters[attacker]
            .who
            .magazine()
            .map(|m| m.per_shot.get());
        if let (Some(per_shot), Some(rounds)) = (per_shot, self.fighters[attacker].rounds)
            && rounds.loaded < per_shot
        {
            return Err(CombatError::OutOfAmmo {
                needed: per_shot,
                loaded: rounds.loaded,
            });
        }

        self.ap_left -= needed;
        let chance = self
            .hit_model
            .chance(&self.fighters[attacker].who, &victim.who);
        let strike = if roll(dice) <= chance.get() {
            let line = i32::from(self.fighters[attacker].who.crit_chance().get())
                .saturating_add(victim.who.crit_chance_against())
                .clamp(0, 100);
            let critical = i32::from(roll(dice)) <= line;
            let raw = damage_before_armor(&profile, power, critical);
            let dt = victim
                .who
                .dt()
                .less(self.fighters[attacker].who.ignored_threshold());
            let through = damage_after_armor(&self.rules, raw, dt, victim.who.dr());
            Strike::Hit {
                critical,
                damage: whole_hit_points(through),
            }
        } else {
            Strike::Missed
        };

        // The rounds go whether the shot hits or not; the check above guarantees
        // the weapon holds enough.
        if let (Some(per_shot), Some(rounds)) = (per_shot, self.fighters[attacker].rounds.as_mut())
        {
            rounds.loaded -= per_shot;
        }
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

    /// The active fighter fills the weapon from the rounds carried. A shot costs
    /// a round whether it hits or not, so the clip is what limits a turn.
    ///
    /// # Errors
    ///
    /// [`CombatError::Over`] when the fight is over, [`CombatError::NothingToReload`]
    /// when the weapon has no magazine, is full or there is nothing to load, and
    /// [`CombatError::NotEnoughAp`] when the reload cannot be paid. Nothing changes
    /// in those cases.
    pub fn reload(&mut self) -> Result<Reload, CombatError> {
        if self.status != Status::Ongoing {
            return Err(CombatError::Over);
        }
        let slot = &self.fighters[self.active];
        let (Some(magazine), Some(rounds)) = (slot.who.magazine(), slot.rounds) else {
            return Err(CombatError::NothingToReload);
        };
        let room = magazine.clip.get().saturating_sub(rounds.loaded);
        let added = rounds.reserve.map_or(room, |reserve| room.min(reserve));
        if added == 0 {
            return Err(CombatError::NothingToReload);
        }
        let needed = magazine.reload_ap.get();
        if needed > self.ap_left {
            return Err(CombatError::NotEnoughAp {
                needed,
                left: self.ap_left,
            });
        }
        self.ap_left -= needed;
        let slot = &mut self.fighters[self.active];
        let rounds = slot.rounds.get_or_insert(rounds);
        rounds.loaded += added;
        rounds.reserve = rounds.reserve.map(|reserve| reserve - added);
        Ok(Reload {
            added,
            loaded: rounds.loaded,
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
        let count = self.turn_order.len();
        let at = self
            .turn_order
            .iter()
            .position(|&i| i == self.active)
            .unwrap_or(0);
        // Both sides have someone standing while the fight goes on, so a next one exists.
        let (step, next) = (1..=count)
            .map(|step| (step, self.turn_order[(at + step) % count]))
            .find(|&(_, i)| self.fighters[i].hp > 0)
            .unwrap_or((count, self.active));
        // Passing the end of the order starts a new round.
        if at + step >= count {
            self.round += 1;
        }
        self.active = next;
        self.ap_left = self.fighters[next].who.max_ap();
        Ok(())
    }

    /// The active fighter tries to run away: one roll against the fighter's chance.
    /// Escaping ends the fight with [`Status::Fled`]; failing loses what is left of
    /// the turn, and the caller then ends it.
    ///
    /// # Errors
    ///
    /// [`CombatError::Over`] when the fight is over and
    /// [`CombatError::OnlyThePlayerFlees`] when the active fighter is an enemy.
    /// Nothing changes and no dice are rolled in those cases.
    pub fn flee(&mut self, dice: &mut impl Dice) -> Result<Flight, CombatError> {
        if self.status != Status::Ongoing {
            return Err(CombatError::Over);
        }
        let runner = &self.fighters[self.active];
        if runner.side != Side::Player {
            return Err(CombatError::OnlyThePlayerFlees);
        }
        if roll(dice) <= runner.who.flee_chance().get() {
            self.status = Status::Fled;
            Ok(Flight::Escaped)
        } else {
            self.ap_left = 0;
            Ok(Flight::Blocked)
        }
    }

    fn standing_status(&self) -> Status {
        let standing = |side| self.fighters.iter().any(|f| f.side == side && f.hp > 0);
        let player_side = match self.lead {
            Some(lead) => self.fighters[lead].hp > 0,
            None => standing(Side::Player),
        };
        match (player_side, standing(Side::Enemy)) {
            (true, false) => Status::Won(Side::Player),
            (false, true) => Status::Won(Side::Enemy),
            _ => Status::Ongoing,
        }
    }
}

/// The weapon starts loaded as far as the rounds allow; the rest is the reserve.
/// The game keeps no "loaded" state between fights, so a fight always begins with
/// a ready weapon and the unspent rounds go back to the pack.
fn starting_rounds(magazine: Magazine) -> Rounds {
    let loaded = magazine
        .stock
        .map_or(magazine.clip.get(), |stock| stock.min(magazine.clip.get()));
    Rounds {
        loaded,
        reserve: magazine.stock.map(|stock| stock - loaded),
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
