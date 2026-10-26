//! The perks and traits the game offers, and who may take which.

use std::collections::BTreeMap;

use super::error::{ProgressError, Unmet};
use super::sheet::Character;
use crate::shared::{CharacterView, Dlc, GameData, Perk, PerkId, Trait, TraitId};

/// Every perk and trait on offer, from the add-ons that are installed.
///
/// Built once from the loaded data and then only read, so a character holds ids
/// and asks the book what they mean.
#[derive(Debug, Clone, Default)]
pub struct PerkBook {
    perks: BTreeMap<PerkId, Perk>,
    traits: BTreeMap<TraitId, Trait>,
}

impl PerkBook {
    /// A book of the perks and traits that belong to one of the `allowed` games.
    #[must_use]
    pub fn new<'a>(
        perks: impl IntoIterator<Item = &'a Perk>,
        traits: impl IntoIterator<Item = &'a Trait>,
        allowed: &[Dlc],
    ) -> Self {
        Self {
            perks: perks
                .into_iter()
                .filter(|perk| allowed.contains(&perk.dlc))
                .map(|perk| (perk.id.clone(), perk.clone()))
                .collect(),
            traits: traits
                .into_iter()
                .filter(|record| allowed.contains(&record.dlc))
                .map(|record| (record.id.clone(), record.clone()))
                .collect(),
        }
    }

    /// The base game's perks and traits: the add-ons have no places to visit yet.
    #[must_use]
    pub fn base_game(data: &GameData) -> Self {
        Self::new(data.perks.iter(), data.traits.iter(), &[Dlc::Base])
    }

    /// A perk.
    #[must_use]
    pub fn perk(&self, id: &PerkId) -> Option<&Perk> {
        self.perks.get(id)
    }

    /// A trait.
    #[must_use]
    pub fn character_trait(&self, id: &TraitId) -> Option<&Trait> {
        self.traits.get(id)
    }

    /// Every perk, ordered by id.
    pub fn perks(&self) -> impl Iterator<Item = &Perk> {
        self.perks.values()
    }

    /// Every trait, ordered by id.
    pub fn traits(&self) -> impl Iterator<Item = &Trait> {
        self.traits.values()
    }

    /// How many times a perk can be taken.
    #[must_use]
    pub fn max_rank(perk: &Perk) -> u8 {
        perk.ranks.unwrap_or(1).max(1)
    }

    /// Whether the character could take the perk now, ignoring whether a pick is
    /// waiting.
    ///
    /// # Errors
    ///
    /// [`ProgressError`] saying why not: the perk is unknown, every rank is taken,
    /// or a requirement is unmet. A requirement the import could not structure can
    /// never be checked, so such a perk is never open.
    pub fn check(&self, id: &PerkId, character: &Character) -> Result<&Perk, ProgressError> {
        let perk = self
            .perks
            .get(id)
            .ok_or_else(|| ProgressError::UnknownPerk(id.clone()))?;
        if character.perk_rank(id) >= Self::max_rank(perk) {
            return Err(ProgressError::PerkMaxed(id.clone()));
        }
        match first_unmet(perk, character) {
            Some(unmet) => Err(ProgressError::PerkLocked {
                perk: id.clone(),
                unmet,
            }),
            None => Ok(perk),
        }
    }

    /// The perks the character could take now, ordered by id.
    #[must_use]
    pub fn open_to(&self, character: &Character) -> Vec<&Perk> {
        self.perks
            .values()
            .filter(|perk| self.check(&perk.id, character).is_ok())
            .collect()
    }
}

fn first_unmet(perk: &Perk, character: &Character) -> Option<Unmet> {
    let requirements = perk.requirements.as_ref()?;
    if let Some(need) = requirements.level
        && character.level().get() < need
    {
        return Some(Unmet::Level { need });
    }
    let low_stat = requirements
        .special
        .iter()
        .find(|(stat, need)| character.special(**stat) < **need);
    if let Some((stat, need)) = low_stat {
        return Some(Unmet::Special {
            stat: *stat,
            need: need.get(),
        });
    }
    let low_skill = requirements
        .skills
        .iter()
        .find(|(skill, need)| character.skill(**skill) < **need);
    if let Some((skill, need)) = low_skill {
        return Some(Unmet::Skill {
            skill: *skill,
            need: need.get(),
        });
    }
    requirements.other.clone().map(Unmet::Other)
}
