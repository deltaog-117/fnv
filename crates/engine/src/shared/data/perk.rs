//! Perk and trait records.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::shared::{Dlc, Modifier, PerkId, Skill, SkillLevel, Special, SpecialLevel, TraitId};

/// What a character needs before a perk can be taken.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerkRequirements {
    /// The lowest level.
    pub level: Option<u32>,
    /// The lowest value of each named attribute.
    #[serde(default)]
    pub special: BTreeMap<Special, SpecialLevel>,
    /// The lowest value of each named skill.
    #[serde(default)]
    pub skills: BTreeMap<Skill, SkillLevel>,
    /// A requirement the import could not structure, such as "Kill 50 abominations".
    /// The game cannot check it, so a perk with one is never offered.
    pub other: Option<String>,
}

/// A perk.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Perk {
    /// The perk's unique id.
    pub id: PerkId,
    /// Display name.
    pub name: String,
    /// The game or add-on it belongs to.
    pub dlc: Dlc,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// The game's form id.
    pub form_id: Option<String>,
    /// What the wiki says it is.
    pub description: Option<String>,
    /// How many times it can be taken; once when absent.
    pub ranks: Option<u8>,
    /// What is needed to take it.
    pub requirements: Option<PerkRequirements>,
    /// The requirements, as the wiki words them.
    pub requires_text: Option<String>,
    /// The effects, as the wiki words them.
    pub effects_text: Option<String>,
    /// What the game applies for each rank. Empty for a perk whose effect is not
    /// built: it can be taken and recorded, and changes no number.
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}

/// A trait chosen when the character is made.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trait {
    /// The trait's unique id.
    pub id: TraitId,
    /// Display name.
    pub name: String,
    /// The game or add-on it belongs to.
    pub dlc: Dlc,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// The game's form id.
    pub form_id: Option<String>,
    /// What the wiki says it is.
    pub description: Option<String>,
    /// What is needed to take it.
    pub requires: Option<String>,
    /// The benefit, as the wiki words it.
    pub benefits: Option<String>,
    /// The penalty, as the wiki words it.
    pub penalties: Option<String>,
    /// What the game applies. Empty for a trait whose effects are not built.
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}
