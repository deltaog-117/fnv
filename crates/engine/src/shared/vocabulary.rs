//! The closed sets of names the game rules talk about.
//!
//! Each enum serializes to the exact strings listed in
//! `schemas/common.schema.json`; a contract test keeps the two in step.

use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter};

/// Which game or add-on a piece of content belongs to.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum Dlc {
    /// The base game.
    Base,
    /// Dead Money.
    DeadMoney,
    /// Honest Hearts.
    HonestHearts,
    /// Old World Blues.
    OldWorldBlues,
    /// Lonesome Road.
    LonesomeRoad,
    /// Gun Runners' Arsenal.
    GunRunnersArsenal,
}

/// The thirteen skills.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    /// Barter.
    Barter,
    /// Energy Weapons.
    EnergyWeapons,
    /// Explosives.
    Explosives,
    /// Guns.
    Guns,
    /// Lockpick.
    Lockpick,
    /// Medicine.
    Medicine,
    /// Melee Weapons.
    MeleeWeapons,
    /// Repair.
    Repair,
    /// Science.
    Science,
    /// Sneak.
    Sneak,
    /// Speech.
    Speech,
    /// Survival.
    Survival,
    /// Unarmed.
    Unarmed,
}

/// The seven S.P.E.C.I.A.L. attributes.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum Special {
    /// Strength.
    Strength,
    /// Perception.
    Perception,
    /// Endurance.
    Endurance,
    /// Charisma.
    Charisma,
    /// Intelligence.
    Intelligence,
    /// Agility.
    Agility,
    /// Luck.
    Luck,
}

/// A reputation standing, ordered from worst to best.
///
/// The order is a simplification used by conditions such as "liked or better".
/// The full fame/infamy matrix, where some standings are not strictly comparable,
/// belongs to the reputation feature.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum ReputationLevel {
    /// Hated everywhere the faction reaches.
    Vilified,
    /// Distrusted.
    Shunned,
    /// Both helpful and harmful deeds are known.
    Mixed,
    /// Not known to the faction.
    Neutral,
    /// Known as helpful.
    Accepted,
    /// Well regarded.
    Liked,
    /// Revered.
    Idolized,
}

/// How a reputation condition compares the current standing with its level.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    /// The standing is this level or better.
    #[default]
    AtLeast,
    /// The standing is this level or worse.
    AtMost,
    /// The standing is exactly this level.
    Is,
}

/// The player's overall moral alignment.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// Good karma.
    Good,
    /// Neutral karma.
    Neutral,
    /// Evil karma.
    Evil,
}

/// What can be asked about a non-player character.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
)]
#[serde(rename_all = "snake_case")]
pub enum NpcState {
    /// The NPC is alive.
    Alive,
    /// The NPC is dead.
    Dead,
    /// The NPC is hostile to the player.
    Hostile,
    /// The player has met the NPC.
    Met,
}

/// A quest stage number, as shown in the quest data (5, 10, 20, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuestStage(u32);

impl QuestStage {
    /// Wraps a stage number.
    #[must_use]
    pub fn new(stage: u32) -> Self {
        Self(stage)
    }

    /// The stage number.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Where a quest stands. The stage-based states carry their stage number, so a
/// state that needs one cannot be built without it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestState {
    /// The quest has not begun.
    NotStarted,
    /// The quest is in progress.
    Active,
    /// The quest finished successfully.
    Completed,
    /// The quest failed or was locked out.
    Failed,
    /// The quest has reached this stage or a later one.
    StageAtLeast(QuestStage),
    /// The quest is exactly at this stage.
    StageIs(QuestStage),
}

impl QuestState {
    /// Every wire name of a quest state, in schema order.
    pub const ALL_TAGS: [&'static str; 6] = [
        "not_started",
        "active",
        "completed",
        "failed",
        "stage_at_least",
        "stage_is",
    ];

    /// The wire name of this state.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::StageAtLeast(_) => "stage_at_least",
            Self::StageIs(_) => "stage_is",
        }
    }
}
