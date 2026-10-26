//! The constants behind leveling, from `data/engine/formulas.json`.

use serde::{Deserialize, Serialize};

use crate::shared::ModelError;

/// A character level, 1 to the highest level any add-on allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Level(u8);

impl Level {
    /// The first level, which needs no experience.
    pub const FIRST: Self = Self(1);
    /// The lowest level number.
    pub const MIN: u8 = 1;
    /// The level cap with every add-on installed.
    pub const MAX: u8 = 50;

    /// Builds a level, rejecting 0 and anything above [`Level::MAX`].
    ///
    /// # Errors
    ///
    /// [`ModelError::OutOfRange`] when `value` is outside 1 to 50.
    pub fn try_new(value: u32) -> Result<Self, ModelError> {
        match u8::try_from(value) {
            Ok(level) if (Self::MIN..=Self::MAX).contains(&level) => Ok(Self(level)),
            _ => Err(ModelError::OutOfRange {
                what: "level",
                min: u32::from(Self::MIN),
                max: u32::from(Self::MAX),
                got: value,
            }),
        }
    }

    /// The level as a plain number.
    #[must_use]
    pub fn get(self) -> u32 {
        u32::from(self.0)
    }
}

impl TryFrom<u32> for Level {
    type Error = ModelError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<Level> for u32 {
    fn from(level: Level) -> Self {
        level.get()
    }
}

/// Points shared out among the seven attributes when a character is made (five
/// in each, plus five more to place), as the wiki states it for the base game.
pub const START_SPECIAL_POINTS: u32 = 40;
/// How many skills are tagged at the start.
pub const TAG_COUNT: usize = 3;
/// Points a tagged skill is raised by.
pub const TAG_BONUS: i32 = 15;
/// How many traits a new character may choose.
pub const MAX_TRAITS: usize = 2;
/// The skill points of every level-up before intelligence and perks, in half
/// points (`10 + 0.5 * intelligence` is `20 + intelligence` halves), so the half
/// points the file says are carried over stay exact.
pub const BASE_SKILL_HALVES: u32 = 20;

/// A perk every second level (`perk_rate` in `formulas.json`).
const STANDARD_PERK_RATE: u32 = 2;

/// The constants of the leveling formulas.
///
/// They mirror `constants` in `formulas.json` (`xp_base`, `xp_step`,
/// `level_cap_base`, `ap_base_cap`, `perk_rate`); a real-data test fails if
/// [`CharacterRules::standard`] drifts from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterRules {
    xp_base: u32,
    xp_step: u32,
    level_cap: Level,
    ap_cap: u32,
    perk_rate: u32,
}

impl CharacterRules {
    /// Builds the rules.
    ///
    /// # Errors
    ///
    /// [`ModelError::OutOfRange`] when the first level needs no XP, the cap is not a
    /// level, or the action-point cap is zero.
    pub fn new(
        xp_base: u32,
        xp_step: u32,
        level_cap: u32,
        ap_cap: u32,
    ) -> Result<Self, ModelError> {
        // A zero base would make every level reachable at 0 XP.
        let range = |what, got| ModelError::OutOfRange {
            what,
            min: 1,
            max: u32::MAX,
            got,
        };
        if xp_base == 0 {
            return Err(range("xp base", xp_base));
        }
        if ap_cap == 0 {
            return Err(range("action point cap", ap_cap));
        }
        Ok(Self {
            xp_base,
            xp_step,
            level_cap: Level::try_new(level_cap)?,
            ap_cap,
            perk_rate: STANDARD_PERK_RATE,
        })
    }

    /// The same rules with a perk every `rate` levels.
    ///
    /// # Errors
    ///
    /// [`ModelError::OutOfRange`] when `rate` is zero, which would never grant one.
    pub fn with_perk_rate(mut self, rate: u32) -> Result<Self, ModelError> {
        if rate == 0 {
            return Err(ModelError::OutOfRange {
                what: "perk rate",
                min: 1,
                max: u32::MAX,
                got: rate,
            });
        }
        self.perk_rate = rate;
        Ok(self)
    }

    /// The base game's rules: 200 XP for level 2, 150 more each level, a cap of 30
    /// at most 95 base action points and a perk every second level.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            xp_base: 200,
            xp_step: 150,
            level_cap: Level(30),
            ap_cap: 95,
            perk_rate: STANDARD_PERK_RATE,
        }
    }

    /// A perk is chosen every this many levels.
    #[must_use]
    pub fn perk_rate(&self) -> u32 {
        self.perk_rate
    }

    /// The highest level reachable.
    #[must_use]
    pub fn level_cap(&self) -> Level {
        self.level_cap
    }

    /// The most action points the base formula gives.
    #[must_use]
    pub fn ap_cap(&self) -> u32 {
        self.ap_cap
    }

    /// The total XP at which `level` is reached: the sum of `xp_to_next_level` over
    /// every earlier level, so `xp_base * (n - 1) + xp_step * (n - 1)(n - 2) / 2`.
    #[must_use]
    pub fn xp_for_level(&self, level: Level) -> u32 {
        let earlier = u64::from(level.get() - 1);
        // (n - 1)(n - 2) is a product of consecutive numbers, so it is even.
        let steps = earlier * earlier.saturating_sub(1) / 2;
        let total = u64::from(self.xp_base) * earlier + u64::from(self.xp_step) * steps;
        u32::try_from(total).unwrap_or(u32::MAX)
    }
}
