//! The sum of every modifier the character's perks and traits make.

use strum::EnumCount;

use crate::shared::{Modifier, Skill};

/// What perks and traits add up to. Kept as sums, so the derived numbers read a
/// field and never walk the perks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Bonuses {
    /// Half points per level-up, on top of the base.
    pub(super) skill_halves: u32,
    pub(super) xp_percent: i32,
    pub(super) carry_weight: u32,
    pub(super) hp: u32,
    pub(super) ap: u32,
    /// Per skill, with the modifiers that name every skill already added in, so a
    /// skill read looks at one number.
    pub(super) skills: [i32; Skill::COUNT],
}

impl Bonuses {
    /// Adds one modifier. Sums saturate, so no pile of perks can wrap a number.
    pub(super) fn add(&mut self, modifier: &Modifier) {
        match *modifier {
            Modifier::SkillPointsPerLevel { amount } => {
                self.skill_halves = self.skill_halves.saturating_add(2 * u32::from(amount));
            }
            Modifier::XpPercent { percent } => {
                self.xp_percent = self.xp_percent.saturating_add(i32::from(percent));
            }
            Modifier::CarryWeight { pounds } => {
                self.carry_weight = self.carry_weight.saturating_add(u32::from(pounds));
            }
            Modifier::MaxHp { amount } => self.hp = self.hp.saturating_add(u32::from(amount)),
            Modifier::ActionPoints { amount } => {
                self.ap = self.ap.saturating_add(u32::from(amount));
            }
            Modifier::AllSkills { amount } => {
                for slot in &mut self.skills {
                    *slot = slot.saturating_add(i32::from(amount));
                }
            }
            Modifier::Skill { skill, amount } => {
                let slot = &mut self.skills[skill as usize];
                *slot = slot.saturating_add(i32::from(amount));
            }
        }
    }
}
