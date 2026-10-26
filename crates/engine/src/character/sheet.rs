//! The character's attributes, experience and growth, and what they imply.

use std::collections::{BTreeMap, BTreeSet};

use strum::{EnumCount, IntoEnumIterator};

use super::bonus::Bonuses;
use super::book::PerkBook;
use super::error::{CreationError, ProgressError};
use super::rules::{
    BASE_SKILL_HALVES, CharacterRules, Level, MAX_TRAITS, START_SPECIAL_POINTS, TAG_BONUS,
    TAG_COUNT,
};
use crate::shared::{
    CharacterSink, CharacterView, EffectError, PerkId, Skill, SkillLevel, Special, SpecialLevel,
    TraitId, whole_amount,
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

    /// The seven values added up.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.0.iter().map(|level| u32::from(level.get())).sum()
    }
}

impl Default for SpecialSet {
    /// A 5 in everything: a stand-in for a character that was not made by hand.
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

/// The player character: attributes, experience, and what the levels gave.
///
/// The level is derived from experience, never stored. What each level hands out
/// (skill points, a perk to choose) is stored, because it cannot be derived once
/// the player has spent it, and `settled` records how far the handing out has
/// gone, so no level pays twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    special: SpecialSet,
    xp: u32,
    rules: CharacterRules,
    /// Points the player put into each skill.
    spent: [u8; Skill::COUNT],
    tagged: [bool; Skill::COUNT],
    traits: BTreeSet<TraitId>,
    /// Perks taken, with the rank reached.
    perks: BTreeMap<PerkId, u8>,
    bonuses: Bonuses,
    /// The highest level whose rewards have been handed out.
    settled: Level,
    /// Skill points banked and not spent, in half points: the file's half points
    /// are carried over, so the count stays exact.
    skill_halves: u32,
    /// Perks that may be chosen now.
    perk_picks: u32,
}

impl Default for Character {
    /// A new level-1 character with the default attributes and the base rules.
    fn default() -> Self {
        Self::new(SpecialSet::default(), CharacterRules::standard())
    }
}

impl Character {
    /// A new character with no experience, no tagged skill and no trait.
    #[must_use]
    pub fn new(special: SpecialSet, rules: CharacterRules) -> Self {
        Self::at_start(special, rules, Level::FIRST, 0)
    }

    /// A character made the way the player makes one: the attributes use exactly
    /// the points a new character has, three skills are tagged and up to two
    /// traits are chosen.
    ///
    /// # Errors
    ///
    /// [`CreationError`] when the attributes do not add up, the wrong number of
    /// skills is tagged, a skill or trait is repeated, or a trait is unknown.
    pub fn create(
        special: SpecialSet,
        tags: &[Skill],
        traits: &[TraitId],
        rules: CharacterRules,
        book: &PerkBook,
    ) -> Result<Self, CreationError> {
        if special.total() != START_SPECIAL_POINTS {
            return Err(CreationError::SpecialTotal {
                expected: START_SPECIAL_POINTS,
                got: special.total(),
            });
        }
        if tags.len() != TAG_COUNT {
            return Err(CreationError::TagCount {
                expected: TAG_COUNT,
                got: tags.len(),
            });
        }
        if traits.len() > MAX_TRAITS {
            return Err(CreationError::TooManyTraits {
                max: MAX_TRAITS,
                got: traits.len(),
            });
        }
        let mut character = Self::new(special, rules);
        for skill in tags {
            if std::mem::replace(&mut character.tagged[*skill as usize], true) {
                return Err(CreationError::DuplicateTag(*skill));
            }
        }
        for id in traits {
            let record = book
                .character_trait(id)
                .ok_or_else(|| CreationError::UnknownTrait(id.clone()))?;
            if !character.traits.insert(id.clone()) {
                return Err(CreationError::DuplicateTrait(id.clone()));
            }
            record
                .modifiers
                .iter()
                .for_each(|modifier| character.bonuses.add(modifier));
        }
        Ok(character)
    }

    /// A character with no history beyond its level: the experience of that level,
    /// no more. Non-player characters take their numbers from the same formulas
    /// this way, so a fight reads one set of rules. Nothing is waiting to be
    /// spent.
    #[must_use]
    pub fn at_level(special: SpecialSet, rules: CharacterRules, level: Level) -> Self {
        Self::at_start(special, rules, level, rules.xp_for_level(level))
    }

    fn at_start(special: SpecialSet, rules: CharacterRules, settled: Level, xp: u32) -> Self {
        Self {
            special,
            xp,
            rules,
            spent: [0; Skill::COUNT],
            tagged: [false; Skill::COUNT],
            traits: BTreeSet::new(),
            perks: BTreeMap::new(),
            bonuses: Bonuses::default(),
            settled,
            skill_halves: 0,
            perk_picks: 0,
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
        // The XP total of a level rises with the level, so bisection finds the
        // highest one reached. Measured with `benches/character.rs`: it reads the
        // level several times per fight and the linear scan from the cap cost about
        // 24 ns a call against about 5 ns here. Level 1 needs no XP, so `low` is
        // always a level that has been reached.
        let reached =
            |n: u32| Level::try_new(n).is_ok_and(|level| self.rules.xp_for_level(level) <= self.xp);
        let (mut low, mut high) = (u32::from(Level::MIN), self.rules.level_cap().get());
        while low < high {
            let middle = low + (high - low).div_ceil(2);
            if reached(middle) {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        Level::try_new(low).unwrap_or(Level::FIRST)
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

    /// Maximum hit points: `100 + 20 * endurance + 5 * (level - 1)`, plus perks.
    #[must_use]
    pub fn max_hp(&self) -> u32 {
        100 + 20 * u32::from(self.special.get(Special::Endurance).get())
            + 5 * (self.level().get() - 1)
            + self.bonuses.hp
    }

    /// Action points: `min(65 + 3 * agility, ap_base_cap)`, plus perks, which may
    /// take it past the cap.
    #[must_use]
    pub fn action_points(&self) -> u32 {
        (65 + 3 * u32::from(self.special.get(Special::Agility).get())).min(self.rules.ap_cap())
            + self.bonuses.ap
    }

    /// Carry weight in pounds: `150 + 10 * strength`, plus perks.
    #[must_use]
    pub fn carry_weight(&self) -> u32 {
        150 + 10 * u32::from(self.special.get(Special::Strength).get()) + self.bonuses.carry_weight
    }

    /// Adds experience, scaled by the percentage perks and traits give, and pays
    /// out every level it reaches. Saturates rather than wrapping.
    pub fn grant_xp(&mut self, amount: u32) {
        let percent = (100 + i64::from(self.bonuses.xp_percent)).max(0);
        let scaled = u64::from(amount) * percent.unsigned_abs() / 100;
        self.xp = self
            .xp
            .saturating_add(u32::try_from(scaled).unwrap_or(u32::MAX));
        self.settle();
    }

    /// Hands out what each level reached since the last call gives: skill points
    /// every level, and a perk to choose every `perk_rate` levels.
    fn settle(&mut self) {
        let level = self.level();
        for reached in (self.settled.get() + 1)..=level.get() {
            self.skill_halves = self.skill_halves.saturating_add(self.halves_per_level());
            if reached % self.rules.perk_rate() == 0 {
                self.perk_picks += 1;
            }
        }
        self.settled = self.settled.max(level);
    }

    /// `10 + 0.5 * intelligence`, in half points, plus what perks add.
    fn halves_per_level(&self) -> u32 {
        BASE_SKILL_HALVES
            + u32::from(self.special.get(Special::Intelligence).get())
            + self.bonuses.skill_halves
    }

    /// Whole skill points waiting to be spent.
    #[must_use]
    pub fn skill_points(&self) -> u32 {
        self.skill_halves / 2
    }

    /// Perks waiting to be chosen.
    #[must_use]
    pub fn perk_picks(&self) -> u32 {
        self.perk_picks
    }

    /// Points the player has put into a skill.
    #[must_use]
    pub fn points_spent(&self, skill: Skill) -> u32 {
        u32::from(self.spent[skill as usize])
    }

    /// Whether the skill was tagged when the character was made.
    #[must_use]
    pub fn is_tagged(&self, skill: Skill) -> bool {
        self.tagged[skill as usize]
    }

    /// The traits, ordered by id.
    pub fn traits(&self) -> impl Iterator<Item = &TraitId> {
        self.traits.iter()
    }

    /// The perks taken, with the rank of each, ordered by id.
    pub fn perks(&self) -> impl Iterator<Item = (&PerkId, u8)> {
        self.perks.iter().map(|(id, rank)| (id, *rank))
    }

    /// How many ranks of the perk are taken; 0 when it is not.
    #[must_use]
    pub fn perk_rank(&self, perk: &PerkId) -> u8 {
        self.perks.get(perk).copied().unwrap_or(0)
    }

    /// Spends one skill point to raise a skill by one.
    ///
    /// # Errors
    ///
    /// [`ProgressError::NoSkillPoints`] when no whole point is banked, and
    /// [`ProgressError::SkillAtMaximum`] when the skill is at 100. Nothing is
    /// spent on a refusal.
    pub fn spend_skill_point(&mut self, skill: Skill) -> Result<(), ProgressError> {
        if self.skill_halves < 2 {
            return Err(ProgressError::NoSkillPoints);
        }
        if self.skill(skill).get() >= SkillLevel::MAX {
            return Err(ProgressError::SkillAtMaximum(skill));
        }
        self.skill_halves -= 2;
        let slot = &mut self.spent[skill as usize];
        *slot = slot.saturating_add(1);
        Ok(())
    }

    /// Uses a waiting pick to take a perk, and applies what it modifies.
    ///
    /// # Errors
    ///
    /// [`ProgressError`] when no pick is waiting, the perk is unknown or fully
    /// taken, or a requirement is unmet. Nothing changes on a refusal.
    pub fn choose_perk(&mut self, book: &PerkBook, id: &PerkId) -> Result<(), ProgressError> {
        if self.perk_picks == 0 {
            return Err(ProgressError::NoPerkPick);
        }
        let perk = book.check(id, self)?;
        perk.modifiers
            .iter()
            .for_each(|modifier| self.bonuses.add(modifier));
        *self.perks.entry(id.clone()).or_insert(0) += 1;
        self.perk_picks -= 1;
        Ok(())
    }

    /// Every skill with its level now, in the order of the [`Skill`] enum.
    pub fn skills(&self) -> impl Iterator<Item = (Skill, SkillLevel)> {
        Skill::iter().map(|skill| (skill, self.skill(skill)))
    }
}

impl CharacterView for Character {
    /// `2 + 2 * stat + ceil(luck / 2)`, plus the points spent, 15 if tagged and
    /// what perks and traits add, kept between 0 and 100.
    fn skill(&self, skill: Skill) -> SkillLevel {
        let stat = i64::from(self.special.get(governing_stat(skill)).get());
        let luck = i64::from(self.special.get(Special::Luck).get());
        let slot = skill as usize;
        let tag = if self.tagged[slot] {
            i64::from(TAG_BONUS)
        } else {
            0
        };
        let value = 2
            + 2 * stat
            + (luck + 1) / 2
            + i64::from(self.spent[slot])
            + tag
            + i64::from(self.bonuses.skills[slot]);
        let capped = value.clamp(i64::from(SkillLevel::MIN), i64::from(SkillLevel::MAX));
        SkillLevel::try_new(u32::try_from(capped).unwrap_or(0))
            .expect("the value was clamped to the skill range")
    }

    fn special(&self, stat: Special) -> SpecialLevel {
        self.special.get(stat)
    }

    fn has_perk(&self, perk: &PerkId) -> bool {
        self.perks.contains_key(perk)
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
