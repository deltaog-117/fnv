//! The character's attributes and experience, and what they imply.

use strum::EnumCount;

use super::rules::{CharacterRules, Level};
use crate::shared::{
    CharacterSink, CharacterView, EffectError, Skill, SkillLevel, Special, SpecialLevel,
    whole_amount,
};

/// The seven S.P.E.C.I.A.L. values; every one always holds a legal level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialSet([SpecialLevel; Special::COUNT]);

impl SpecialSet {
    /// Every attribute at the same level.
    #[must_use]
    pub fn uniform(level: SpecialLevel) -> Self {
        Self([level; Special::COUNT])
    }

    /// The same set with one attribute changed.
    #[must_use]
    pub fn with(mut self, stat: Special, level: SpecialLevel) -> Self {
        self.0[stat as usize] = level;
        self
    }

    /// One attribute.
    #[must_use]
    pub fn get(&self, stat: Special) -> SpecialLevel {
        self.0[stat as usize]
    }
}

impl Default for SpecialSet {
    /// A 5 in everything: a stand-in until character creation exists.
    fn default() -> Self {
        let five = SpecialLevel::try_new(5).expect("5 is a legal S.P.E.C.I.A.L. level");
        Self::uniform(five)
    }
}

/// The attribute a skill starts from (`skill_initial_*` in `formulas.json`).
fn governing_stat(skill: Skill) -> Special {
    match skill {
        Skill::Barter | Skill::Speech => Special::Charisma,
        Skill::EnergyWeapons | Skill::Explosives | Skill::Lockpick => Special::Perception,
        Skill::Guns | Skill::Sneak => Special::Agility,
        Skill::Medicine | Skill::Repair | Skill::Science => Special::Intelligence,
        Skill::MeleeWeapons => Special::Strength,
        Skill::Survival | Skill::Unarmed => Special::Endurance,
    }
}

/// The player character: attributes plus the XP earned.
///
/// Level, hit points and the rest are derived on demand, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Character {
    special: SpecialSet,
    xp: u32,
    rules: CharacterRules,
}

impl Default for Character {
    /// A new level-1 character with the default attributes and the base rules.
    fn default() -> Self {
        Self::new(SpecialSet::default(), CharacterRules::standard())
    }
}

impl Character {
    /// A new character with no experience.
    #[must_use]
    pub fn new(special: SpecialSet, rules: CharacterRules) -> Self {
        Self {
            special,
            xp: 0,
            rules,
        }
    }

    /// The attributes.
    #[must_use]
    pub fn attributes(&self) -> &SpecialSet {
        &self.special
    }

    /// Total experience earned.
    #[must_use]
    pub fn xp(&self) -> u32 {
        self.xp
    }

    /// The level: the highest one whose XP total has been reached, up to the cap.
    #[must_use]
    pub fn level(&self) -> Level {
        (1..=self.rules.level_cap().get())
            .rev()
            .filter_map(|n| Level::try_new(n).ok())
            .find(|level| self.rules.xp_for_level(*level) <= self.xp)
            // Level 1 needs no XP, so the search always ends there; this only
            // keeps the function total.
            .unwrap_or(Level::FIRST)
    }

    /// XP still needed for the next level; `None` at the cap.
    #[must_use]
    pub fn xp_to_next(&self) -> Option<u32> {
        let level = self.level();
        if level >= self.rules.level_cap() {
            return None;
        }
        let next = Level::try_new(level.get() + 1).ok()?;
        Some(self.rules.xp_for_level(next) - self.xp)
    }

    /// Maximum hit points: `100 + 20 * endurance + 5 * (level - 1)`.
    #[must_use]
    pub fn max_hp(&self) -> u32 {
        100 + 20 * u32::from(self.special.get(Special::Endurance).get())
            + 5 * (self.level().get() - 1)
    }

    /// Action points: `min(65 + 3 * agility, ap_base_cap)`.
    #[must_use]
    pub fn action_points(&self) -> u32 {
        (65 + 3 * u32::from(self.special.get(Special::Agility).get())).min(self.rules.ap_cap())
    }

    /// Carry weight in pounds: `150 + 10 * strength`.
    #[must_use]
    pub fn carry_weight(&self) -> u32 {
        150 + 10 * u32::from(self.special.get(Special::Strength).get())
    }

    /// Adds experience, saturating rather than wrapping.
    pub fn grant_xp(&mut self, amount: u32) {
        self.xp = self.xp.saturating_add(amount);
    }
}

impl CharacterView for Character {
    /// The starting value: `2 + 2 * stat + ceil(luck / 2)`, at most 100.
    fn skill(&self, skill: Skill) -> SkillLevel {
        let stat = u32::from(self.special.get(governing_stat(skill)).get());
        let luck = u32::from(self.special.get(Special::Luck).get());
        let value = (2 + 2 * stat + luck.div_ceil(2)).min(u32::from(SkillLevel::MAX));
        SkillLevel::try_new(value).expect("the value was capped to the skill range")
    }

    fn special(&self, stat: Special) -> SpecialLevel {
        self.special.get(stat)
    }
}

impl CharacterSink for Character {
    fn add_xp(&mut self, amount: f64) -> Result<(), EffectError> {
        let whole = whole_amount("xp", amount)?;
        let earned = u32::try_from(whole).map_err(|_| EffectError::Refused {
            effect: "xp",
            reason: format!("{whole} would take experience away"),
        })?;
        self.grant_xp(earned);
        Ok(())
    }
}
