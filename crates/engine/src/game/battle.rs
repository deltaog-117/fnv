//! Fights as the game sees them: fighters built from the character, the gear and
//! the NPC records, and the wounds carried back out.
//!
//! [`Arsenal`] is built once from the loaded data and checks everything a fight
//! needs (weapon and armor ids, the foes' NPC records, every number's range), so
//! a bad encounter fails at load and never in the middle of a fight. A [`Battle`]
//! is one fight over a [`Combat`]; it is started and settled by [`Game`](super::Game).

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;

use thiserror::Error;

use crate::character::{Character, CharacterRules, Level, SpecialSet};
use crate::combat::{
    AttackProfile, Combat, CombatError, Combatant, Dice, FighterId, FlatHitChance, Magazine,
    Multiplier, Percent, Points, Power, Reload, Resolution, Rules, StatError, enemy_turns,
};
use crate::shared::{
    AmmoFamilyId, Armor, BatchError, CharacterView, EffectError, Encounter, EncounterId, Foe,
    GameData, InventoryView, ItemId, ModelError, Npc, NpcSpecial, Skill, Special, SpecialLevel,
    Weapon, WeaponClass,
};

/// The chance every attack has to hit. A stand-in: the wiki gives no hit-chance
/// formula, so no weapon spread, skill or range changes it yet.
const STAND_IN_HIT_CHANCE: u32 = 65;
/// The most the Luck-based critical chance can reach, from `formulas.json`.
const CRIT_LUCK_CAP: u32 = 10;
/// Action points a reload costs. A stand-in: the data gives the reload time in
/// seconds and no conversion to action points is known, so every weapon pays the same.
const RELOAD_AP: u32 = 2;
/// A missing attribute on an NPC record reads as the middle of the scale.
const AVERAGE_SPECIAL: u32 = 5;

/// Why a fight could not be prepared, started or settled.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BattleError {
    /// An item's combat numbers are out of range.
    #[error("{item}: {source}")]
    BadGear {
        /// The item.
        item: ItemId,
        /// What was out of range.
        source: StatError,
    },
    /// An attribute or level on an NPC record is out of range.
    #[error("{npc}: {source}")]
    BadNpc {
        /// The NPC.
        npc: crate::shared::NpcId,
        /// What was out of range.
        source: ModelError,
    },
    /// A foe's NPC is not in the loaded data.
    #[error("encounter {encounter}: unknown npc {npc}")]
    UnknownNpc {
        /// The encounter.
        encounter: EncounterId,
        /// The missing NPC.
        npc: crate::shared::NpcId,
    },
    /// A foe's weapon is not a weapon that can be used in a fight.
    #[error("encounter {encounter}: {item} is not a usable weapon")]
    NotAWeapon {
        /// The encounter.
        encounter: EncounterId,
        /// The item.
        item: ItemId,
    },
    /// A foe's armor is not armor.
    #[error("encounter {encounter}: {item} is not armor")]
    NotArmor {
        /// The encounter.
        encounter: EncounterId,
        /// The item.
        item: ItemId,
    },
    /// The encounter does not exist.
    #[error("unknown encounter {0}")]
    UnknownEncounter(EncounterId),
    /// The encounter exists but is not on offer right now.
    #[error("{0} is not on offer")]
    NotOffered(EncounterId),
    /// The player has no hit points left.
    #[error("you are down")]
    PlayerDown,
    /// The player has nothing in hand.
    #[error("nothing is equipped to fight with")]
    Unarmed,
    /// What the player wields or wears has no combat numbers.
    #[error("{0} cannot be used in a fight")]
    UnusableGear(ItemId),
    /// The fight has not ended yet.
    #[error("the fight is not over")]
    Unfinished,
    /// The fight refused a move.
    #[error(transparent)]
    Combat(#[from] CombatError),
    /// A combat number was out of range.
    #[error(transparent)]
    Stat(#[from] StatError),
    /// The rounds fired could not be taken from the pack; nothing changed.
    #[error("the rounds fired could not be taken from the pack: {0}")]
    Ammo(#[from] EffectError),
    /// The winning effects could not be applied; nothing changed.
    #[error(transparent)]
    Effects(#[from] BatchError),
}

/// How a settled fight ended for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every enemy is down; the encounter's winning effects were applied.
    Victory,
    /// The player is down.
    Defeat,
}

/// A weapon's combat numbers, checked once.
#[derive(Debug, Clone, PartialEq)]
struct WeaponStats {
    class: WeaponClass,
    damage: Points,
    crit_damage: Points,
    crit_chance_mult: f64,
    ap_cost: NonZeroU32,
    ammo: Option<AmmoStats>,
}

/// What a weapon fires and how many rounds it holds, checked once.
#[derive(Debug, Clone, PartialEq)]
struct AmmoStats {
    family: AmmoFamilyId,
    clip: NonZeroU32,
    per_shot: NonZeroU32,
}

/// An armor piece's combat numbers, checked once.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ArmorStats {
    dt: Points,
    dr: Points,
}

/// Someone in a fight: the numbers [`Combat`] reads, fixed when the fight starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Fighter {
    name: String,
    max_hp: NonZeroU32,
    max_ap: u32,
    dt: Points,
    dr: Points,
    crit_chance: Percent,
    attack: AttackProfile,
    magazine: Option<Magazine>,
    ammo_item: Option<ItemId>,
}

impl Fighter {
    /// The name to show.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The ammunition item the weapon fires, when the fighter's rounds come from the
    /// pack; `None` for foes, whose supply the game does not track, and for weapons
    /// that need none.
    #[must_use]
    pub fn ammo_item(&self) -> Option<&ItemId> {
        self.ammo_item.as_ref()
    }

    /// Rounds the pack held when the fight began; `None` when the pack is not tracked.
    #[must_use]
    pub fn starting_stock(&self) -> Option<u32> {
        self.magazine.and_then(|magazine| magazine.stock)
    }

    /// Builds a fighter from a character, a weapon and the armor worn.
    ///
    /// Attribute-driven numbers come from the character, so the player and the
    /// foes read the same formulas: action points, critical chance (Luck, capped,
    /// times the weapon's multiplier) and the melee or unarmed damage bonus.
    fn assemble(
        name: String,
        max_hp: NonZeroU32,
        character: &Character,
        weapon: &WeaponStats,
        armor: Points,
        resistance: Points,
        rounds: Rounds,
    ) -> Result<Self, StatError> {
        let level_of = |stat| u32::from(character.special(stat).get());
        let crit = f64::from(level_of(Special::Luck).min(CRIT_LUCK_CAP)) * weapon.crit_chance_mult;
        let bonus = match weapon.class {
            WeaponClass::Melee => 0.5 * f64::from(level_of(Special::Strength)),
            WeaponClass::Unarmed => {
                (f64::from(character.skill(Skill::Unarmed).get()) / 20.0 + 0.5).ceil()
            }
            _ => 0.0,
        };
        Ok(Self {
            name,
            max_hp,
            max_ap: character.action_points(),
            dt: armor,
            dr: resistance,
            crit_chance: Percent::try_new(crit.round().clamp(0.0, 100.0) as u32)?,
            attack: AttackProfile {
                ap_cost: weapon.ap_cost,
                base_damage: weapon.damage,
                crit_damage: weapon.crit_damage,
                skill: Multiplier::FULL,
                condition: Multiplier::FULL,
                bonus: Points::try_new(bonus)?,
            },
            magazine: weapon.ammo.as_ref().map(|ammo| Magazine {
                clip: ammo.clip,
                per_shot: ammo.per_shot,
                reload_ap: NonZeroU32::new(RELOAD_AP).unwrap_or(NonZeroU32::MIN),
                stock: rounds.stock,
            }),
            ammo_item: rounds.item,
        })
    }
}

impl Combatant for Fighter {
    fn max_hp(&self) -> NonZeroU32 {
        self.max_hp
    }

    fn max_ap(&self) -> u32 {
        self.max_ap
    }

    fn dt(&self) -> Points {
        self.dt
    }

    fn dr(&self) -> Points {
        self.dr
    }

    fn crit_chance(&self) -> Percent {
        self.crit_chance
    }

    fn attack(&self) -> AttackProfile {
        self.attack
    }

    fn magazine(&self) -> Option<Magazine> {
        self.magazine
    }
}

/// Where a fighter's rounds come from.
#[derive(Debug, Clone, Default)]
struct Rounds {
    /// The ammunition item taken from the pack; `None` for an endless supply.
    item: Option<ItemId>,
    /// How many rounds the pack holds; `None` for an endless supply.
    stock: Option<u32>,
}

#[derive(Debug, Clone)]
struct Prepared {
    encounter: Encounter,
    foes: Vec<Fighter>,
}

/// Everything a fight needs from the data, checked once and shared.
#[derive(Debug, Default)]
pub struct Arsenal {
    weapons: HashMap<ItemId, WeaponStats>,
    armor: HashMap<ItemId, ArmorStats>,
    /// The ammunition items of each family, standard loads first.
    ammo: HashMap<AmmoFamilyId, Vec<ItemId>>,
    encounters: BTreeMap<EncounterId, Prepared>,
}

impl Arsenal {
    /// Prepares every weapon, every piece of armor and every encounter.
    ///
    /// A weapon with no action point cost (a thrown or placed one) is left out, so
    /// it cannot be wielded in a fight; an encounter that names one is refused.
    ///
    /// # Errors
    ///
    /// [`BattleError`] naming the item, NPC or encounter at fault.
    pub fn from_data(data: &GameData) -> Result<Self, BattleError> {
        let weapons = data
            .weapons
            .iter()
            .filter_map(|weapon| weapon_stats(weapon).transpose())
            .collect::<Result<HashMap<_, _>, _>>()?;
        let armor = data
            .armor
            .iter()
            .map(|piece| Ok((piece.id.clone(), armor_stats(piece)?)))
            .collect::<Result<HashMap<_, _>, BattleError>>()?;
        let mut arsenal = Self {
            weapons,
            armor,
            ammo: ammo_by_family(data),
            encounters: BTreeMap::new(),
        };
        for encounter in data.encounters.iter() {
            let foes = encounter
                .foes
                .iter()
                .map(|foe| arsenal.foe(data, &encounter.id, foe))
                .collect::<Result<Vec<_>, _>>()?;
            arsenal.encounters.insert(
                encounter.id.clone(),
                Prepared {
                    encounter: encounter.clone(),
                    foes,
                },
            );
        }
        Ok(arsenal)
    }

    /// The encounters, ordered by id.
    pub fn encounters(&self) -> impl Iterator<Item = &Encounter> {
        self.encounters.values().map(|prepared| &prepared.encounter)
    }

    /// The encounter with this id.
    #[must_use]
    pub fn encounter(&self, id: &EncounterId) -> Option<&Encounter> {
        self.encounters.get(id).map(|prepared| &prepared.encounter)
    }

    fn foe(
        &self,
        data: &GameData,
        encounter: &EncounterId,
        foe: &Foe,
    ) -> Result<Fighter, BattleError> {
        let npc = data
            .npcs
            .get(&foe.npc)
            .ok_or_else(|| BattleError::UnknownNpc {
                encounter: encounter.clone(),
                npc: foe.npc.clone(),
            })?;
        let weapon = self
            .weapons
            .get(&foe.weapon)
            .ok_or_else(|| BattleError::NotAWeapon {
                encounter: encounter.clone(),
                item: foe.weapon.clone(),
            })?;
        let armor = foe
            .armor
            .iter()
            .map(|item| {
                self.armor.get(item).ok_or_else(|| BattleError::NotArmor {
                    encounter: encounter.clone(),
                    item: item.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let character = npc_character(npc)?;
        // The record's own hit points win over the formula, which is the player's.
        let max_hp = foe
            .hp
            .or_else(|| npc.hp.and_then(whole_hit_points))
            .or_else(|| NonZeroU32::new(character.max_hp()))
            .unwrap_or(NonZeroU32::MIN);
        let (dt, dr) = worn(armor.into_iter().copied())?;
        Ok(Fighter::assemble(
            npc.name.clone(),
            max_hp,
            &character,
            weapon,
            dt,
            dr,
            Rounds::default(),
        )?)
    }
}

/// A weapon's numbers, or `None` for one that costs no action points to fire.
fn weapon_stats(weapon: &Weapon) -> Result<Option<(ItemId, WeaponStats)>, BattleError> {
    let bad = |source| BattleError::BadGear {
        item: weapon.id.clone(),
        source,
    };
    let Some(ap_cost) = weapon
        .ap_cost
        .map(f64::round)
        .filter(|cost| *cost >= 1.0)
        .and_then(|cost| NonZeroU32::new(cost as u32))
    else {
        return Ok(None);
    };
    let crit_chance_mult = weapon.crit_chance_mult.unwrap_or(1.0);
    if !crit_chance_mult.is_finite() || crit_chance_mult < 0.0 {
        return Err(bad(StatError::OutOfRange {
            what: "critical chance multiplier",
            min: 0.0,
            max: f64::MAX,
            got: crit_chance_mult,
        }));
    }
    let stats = WeaponStats {
        ammo: ammo_stats(weapon),
        class: weapon.weapon_class,
        damage: Points::try_new(weapon.damage).map_err(bad)?,
        crit_damage: Points::try_new(weapon.crit_damage.unwrap_or(0.0)).map_err(bad)?,
        crit_chance_mult,
        ap_cost,
    };
    Ok(Some((weapon.id.clone(), stats)))
}

/// The magazine of a weapon that fires ammunition. A weapon whose ammunition the
/// data does not resolve to a family (the hologram beam) is treated as needing none,
/// rather than as one that can never be fired. A shot never uses more rounds than
/// the weapon holds.
fn ammo_stats(weapon: &Weapon) -> Option<AmmoStats> {
    let family = weapon.ammo_family.clone()?;
    let clip = weapon.clip_size.and_then(NonZeroU32::new)?;
    let per_shot = weapon
        .ammo_per_shot
        .filter(|rounds| rounds.is_finite())
        .map_or(1, |rounds| rounds.round().max(1.0) as u32)
        .min(clip.get());
    Some(AmmoStats {
        family,
        clip,
        per_shot: NonZeroU32::new(per_shot).unwrap_or(NonZeroU32::MIN),
    })
}

/// Ammunition items grouped by family, the plain load first so it is the one a
/// weapon uses unless the pack holds only another.
fn ammo_by_family(data: &GameData) -> HashMap<AmmoFamilyId, Vec<ItemId>> {
    let mut by_family: HashMap<AmmoFamilyId, Vec<(bool, ItemId)>> = HashMap::new();
    for ammo in data.ammo.iter() {
        let plain = ammo.variant.as_deref() == Some("standard") || ammo.variant.is_none();
        by_family
            .entry(ammo.family.clone())
            .or_default()
            .push((!plain, ammo.id.clone()));
    }
    by_family
        .into_iter()
        .map(|(family, mut items)| {
            items.sort();
            (family, items.into_iter().map(|(_, id)| id).collect())
        })
        .collect()
}

fn armor_stats(piece: &Armor) -> Result<ArmorStats, BattleError> {
    let bad = |source| BattleError::BadGear {
        item: piece.id.clone(),
        source,
    };
    Ok(ArmorStats {
        dt: Points::try_new(piece.dt).map_err(bad)?,
        dr: Points::try_new(piece.dr.unwrap_or(0.0)).map_err(bad)?,
    })
}

/// Thresholds add up across the pieces worn; so do resistances.
fn worn(pieces: impl Iterator<Item = ArmorStats>) -> Result<(Points, Points), StatError> {
    let (dt, dr) = pieces.fold((0.0, 0.0), |(dt, dr), piece| {
        (dt + piece.dt.get(), dr + piece.dr.get())
    });
    Ok((Points::try_new(dt)?, Points::try_new(dr)?))
}

/// Hit points as a whole number, if the record gives a positive one.
fn whole_hit_points(hp: f64) -> Option<NonZeroU32> {
    (hp.is_finite() && hp >= 1.0)
        .then(|| hp.round() as u32)
        .and_then(NonZeroU32::new)
}

/// An NPC's attributes and level as a character, so a foe's action points and
/// critical chance come from the formulas the player's do. A missing attribute
/// reads as average; a stat outside the scale is clamped, because the wiki lists
/// some creatures above it.
fn npc_character(npc: &Npc) -> Result<Character, BattleError> {
    let bad = |source| BattleError::BadNpc {
        npc: npc.id.clone(),
        source,
    };
    let given = npc.special.unwrap_or(NpcSpecial {
        strength: None,
        perception: None,
        endurance: None,
        charisma: None,
        intelligence: None,
        agility: None,
        luck: None,
    });
    let mut special = SpecialSet::default();
    for (stat, value) in [
        (Special::Strength, given.strength),
        (Special::Perception, given.perception),
        (Special::Endurance, given.endurance),
        (Special::Charisma, given.charisma),
        (Special::Intelligence, given.intelligence),
        (Special::Agility, given.agility),
        (Special::Luck, given.luck),
    ] {
        let clamped = value
            .unwrap_or(AVERAGE_SPECIAL)
            .clamp(u32::from(SpecialLevel::MIN), u32::from(SpecialLevel::MAX));
        special = special.with(stat, SpecialLevel::try_new(clamped).map_err(bad)?);
    }
    let level = Level::try_new(
        npc.level
            .unwrap_or(1)
            .clamp(u32::from(Level::MIN), u32::from(Level::MAX)),
    )
    .map_err(bad)?;
    Ok(Character::at_level(
        special,
        CharacterRules::standard(),
        level,
    ))
}

impl Arsenal {
    /// The player as a fighter: their numbers, the weapon in hand and the armor
    /// worn (`worn` lists the equipped armor ids).
    pub(super) fn player(
        &self,
        character: &Character,
        weapon: Option<&ItemId>,
        worn_armor: &[&ItemId],
        pack: &impl InventoryView,
    ) -> Result<Fighter, BattleError> {
        let weapon_id = weapon.ok_or(BattleError::Unarmed)?;
        let stats = self
            .weapons
            .get(weapon_id)
            .ok_or_else(|| BattleError::UnusableGear(weapon_id.clone()))?;
        let pieces = worn_armor
            .iter()
            .map(|id| {
                self.armor
                    .get(*id)
                    .copied()
                    .ok_or_else(|| BattleError::UnusableGear((*id).clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (dt, dr) = worn(pieces.into_iter())?;
        let max_hp = NonZeroU32::new(character.max_hp()).unwrap_or(NonZeroU32::MIN);
        // The first load of the family the pack holds; with none, the plain load,
        // so the fight shows what is missing.
        let item = stats.ammo.as_ref().and_then(|ammo| {
            let loads = self.ammo.get(&ammo.family)?;
            loads
                .iter()
                .find(|load| pack.item_count(load) > 0)
                .or_else(|| loads.first())
                .cloned()
        });
        let rounds = Rounds {
            stock: item.as_ref().map(|load| pack.item_count(load)),
            item,
        };
        Ok(Fighter::assemble(
            "You".to_owned(),
            max_hp,
            character,
            stats,
            dt,
            dr,
            rounds,
        )?)
    }

    pub(super) fn foes(&self, id: &EncounterId) -> Option<&[Fighter]> {
        self.encounters
            .get(id)
            .map(|prepared| prepared.foes.as_slice())
    }
}

/// One fight in progress. The player acts first; each [`Battle::end_turn`] plays
/// the enemies' turns. When it is over, hand it to [`Game::finish_battle`](super::Game::finish_battle).
#[derive(Debug, Clone)]
pub struct Battle {
    encounter: EncounterId,
    combat: Combat<Fighter, FlatHitChance>,
    player: FighterId,
}

impl Battle {
    /// Starts the fight with the player at `hp`, ahead of the foes.
    pub(super) fn start(
        encounter: EncounterId,
        player: Fighter,
        hp: NonZeroU32,
        foes: &[Fighter],
    ) -> Result<Self, BattleError> {
        let order = std::iter::once((crate::combat::Side::Player, player)).chain(
            foes.iter()
                .map(|foe| (crate::combat::Side::Enemy, foe.clone())),
        );
        let hit = FlatHitChance(Percent::try_new(STAND_IN_HIT_CHANCE)?);
        let combat = Combat::new(order, Rules::standard(), hit)?;
        let player = combat.active();
        Ok(Self {
            encounter,
            combat: combat.with_hp(player, hp)?,
            player,
        })
    }

    /// Which encounter this is.
    #[must_use]
    pub fn encounter(&self) -> &EncounterId {
        &self.encounter
    }

    /// The fight, to read hit points, action points, status and the fighters.
    #[must_use]
    pub fn combat(&self) -> &Combat<Fighter, FlatHitChance> {
        &self.combat
    }

    /// The player's place in the fight.
    #[must_use]
    pub fn player(&self) -> FighterId {
        self.player
    }

    /// The player attacks `target`.
    ///
    /// # Errors
    ///
    /// [`CombatError`] when the move is refused (the fight is unchanged).
    pub fn attack(
        &mut self,
        target: FighterId,
        power: Power,
        dice: &mut impl Dice,
    ) -> Result<Resolution, CombatError> {
        self.combat.attack(target, power, dice)
    }

    /// The player reloads the weapon from the rounds carried.
    ///
    /// # Errors
    ///
    /// [`CombatError`] when the move is refused (the fight is unchanged).
    pub fn reload(&mut self) -> Result<Reload, CombatError> {
        self.combat.reload()
    }

    /// The player passes; the enemies then play until it is the player's turn
    /// again or the fight is over. Returns what the enemies did.
    ///
    /// # Errors
    ///
    /// [`CombatError::Over`] when the fight is already over.
    pub fn end_turn(&mut self, dice: &mut impl Dice) -> Result<Vec<Resolution>, CombatError> {
        self.combat.end_turn()?;
        Ok(enemy_turns(&mut self.combat, dice))
    }
}
