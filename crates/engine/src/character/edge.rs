//! What a character's perks and traits add to a fight and to healing, resolved for
//! the weapon in hand.

use crate::shared::{FoeKind, ItemId, Modifier, Skill};

/// What an attack against one kind of foe gains or loses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Versus {
    /// The kind of foe.
    pub kind: FoeKind,
    /// Percent added to the damage.
    pub damage_percent: i32,
    /// Percent added to the critical bonus.
    pub crit_damage_percent: i32,
}

/// The fight numbers perks and traits change, worked out for one weapon.
///
/// Every field is a sum, so two perks stack by addition and the order they were
/// taken in does not matter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edge {
    /// Points added to the armor's damage threshold.
    pub threshold: i32,
    /// Percent points added to the armor's damage resistance.
    pub resistance: i32,
    /// Points of a target's threshold the attacks ignore.
    pub ignored_threshold: u32,
    /// Percent added to the weapon's damage.
    pub damage_percent: i32,
    /// Percent points added to the chance of a critical hit.
    pub crit_chance: i32,
    /// Percent added to the critical bonus.
    pub crit_damage_percent: i32,
    /// Percent added to what an attack costs in action points.
    pub ap_cost_percent: i32,
    /// Percent added to what a reload costs in action points.
    pub reload_ap_percent: i32,
    /// Percent added to the weapon's spread.
    pub spread_percent: i32,
    /// What changes against particular kinds of foe, one entry per kind.
    pub versus: Vec<Versus>,
}

impl Edge {
    /// Adds up the modifiers that apply to a weapon governed by `skill`.
    pub(super) fn of<'a>(
        modifiers: impl IntoIterator<Item = &'a Modifier>,
        skill: Option<Skill>,
    ) -> Self {
        let mut edge = Self::default();
        let applies = |only: &Option<Skill>| only.is_none_or(|needed| Some(needed) == skill);
        for modifier in modifiers {
            match modifier {
                Modifier::DamageThreshold { amount } => {
                    edge.threshold = edge.threshold.saturating_add(i32::from(*amount));
                }
                Modifier::DamageResistance { percent } => {
                    edge.resistance = edge.resistance.saturating_add(i32::from(*percent));
                }
                Modifier::IgnoreThreshold { amount } => {
                    edge.ignored_threshold =
                        edge.ignored_threshold.saturating_add(u32::from(*amount));
                }
                Modifier::DamagePercent {
                    percent,
                    skill: only,
                    foe,
                } if applies(only) => match foe {
                    Some(kind) => edge.versus_mut(*kind).damage_percent += i32::from(*percent),
                    None => edge.damage_percent += i32::from(*percent),
                },
                Modifier::CritChance {
                    percent,
                    skill: only,
                } if applies(only) => edge.crit_chance += i32::from(*percent),
                Modifier::CritDamagePercent {
                    percent,
                    skill: only,
                    foe,
                } if applies(only) => match foe {
                    Some(kind) => edge.versus_mut(*kind).crit_damage_percent += i32::from(*percent),
                    None => edge.crit_damage_percent += i32::from(*percent),
                },
                Modifier::ApCostPercent {
                    percent,
                    skill: only,
                } if applies(only) => edge.ap_cost_percent += i32::from(*percent),
                Modifier::ReloadApPercent { percent } => {
                    edge.reload_ap_percent += i32::from(*percent);
                }
                Modifier::SpreadPercent {
                    percent,
                    skill: only,
                } if applies(only) => edge.spread_percent += i32::from(*percent),
                _ => {}
            }
        }
        edge
    }

    fn versus_mut(&mut self, kind: FoeKind) -> &mut Versus {
        let at = self
            .versus
            .iter()
            .position(|entry| entry.kind == kind)
            .unwrap_or_else(|| {
                self.versus.push(Versus {
                    kind,
                    damage_percent: 0,
                    crit_damage_percent: 0,
                });
                self.versus.len() - 1
            });
        &mut self.versus[at]
    }

    /// What an attack against `kind` gains, if anything.
    #[must_use]
    pub fn against(&self, kind: FoeKind) -> Option<&Versus> {
        self.versus.iter().find(|entry| entry.kind == kind)
    }
}

/// Percent added to what `item` restores, from the modifiers that apply to it.
pub(super) fn heal_percent<'a>(
    modifiers: impl IntoIterator<Item = &'a Modifier>,
    item: &ItemId,
) -> i32 {
    modifiers
        .into_iter()
        .filter_map(|modifier| match modifier {
            Modifier::HealPercent {
                percent,
                item: only,
            } if only.as_ref().is_none_or(|needed| needed == item) => Some(i32::from(*percent)),
            _ => None,
        })
        .sum()
}

/// Percent added to how fast a weapon wears.
pub(super) fn wear_percent<'a>(modifiers: impl IntoIterator<Item = &'a Modifier>) -> i32 {
    modifiers
        .into_iter()
        .filter_map(|modifier| match modifier {
            Modifier::WeaponWearPercent { percent } => Some(i32::from(*percent)),
            _ => None,
        })
        .sum()
}
