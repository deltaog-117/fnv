//! Two stand-in game states for the evaluator tests.
//!
//! `MemoryState` is a small but honest implementation of every view and sink, so
//! effects can be applied and the result read back through conditions.
//! `HashedView` answers every question from a hash of the question, which gives
//! arbitrary yet consistent states for checking the laws of the evaluator.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;

use engine::shared::{
    Alignment, CharacterSink, CharacterView, CompanionId, CompanionSink, CompanionView,
    EffectError, EndingId, FactionId, FlagId, HealthPercent, InventorySink, InventoryView, ItemId,
    NpcId, NpcState, PerkId, QuestId, QuestSink, QuestStage, QuestStatus, QuestView,
    ReputationLevel, Skill, SkillLevel, Special, SpecialLevel, StandingSink, StandingView,
    WorldSink, WorldView,
};
use strum::IntoEnumIterator;

#[derive(Debug, Clone, Default, PartialEq)]
struct Npc {
    dead: bool,
    hostile: bool,
    removed: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryState {
    quests: HashMap<QuestId, (QuestStatus, Option<QuestStage>)>,
    blocked: HashSet<QuestId>,
    levels: HashMap<FactionId, ReputationLevel>,
    points: HashMap<FactionId, (f64, f64)>,
    karma: f64,
    xp: f64,
    caps: f64,
    items: HashMap<ItemId, u32>,
    equipped: HashMap<ItemId, HashSet<FactionId>>,
    skills: HashMap<Skill, SkillLevel>,
    specials: HashMap<Special, SpecialLevel>,
    perks: HashSet<PerkId>,
    // None is full health: a derived Default must not start the player at zero.
    health: Option<HealthPercent>,
    companions: HashSet<CompanionId>,
    flags: HashSet<FlagId>,
    npcs: HashMap<NpcId, Npc>,
    endings: HashSet<EndingId>,
}

impl MemoryState {
    /// Makes an NPC known to the state; effects on unknown NPCs fail.
    #[must_use]
    pub fn with_npc(mut self, npc: &NpcId) -> Self {
        self.npcs.insert(npc.clone(), Npc::default());
        self
    }

    #[must_use]
    pub fn with_skill(mut self, skill: Skill, level: u32) -> Self {
        self.skills
            .insert(skill, SkillLevel::try_new(level).expect("level in range"));
        self
    }

    #[must_use]
    pub fn with_health(mut self, percent: u32) -> Self {
        self.health = Some(HealthPercent::try_new(percent).expect("percent in range"));
        self
    }

    #[must_use]
    pub fn with_perk(mut self, perk: &PerkId) -> Self {
        self.perks.insert(perk.clone());
        self
    }

    #[must_use]
    pub fn wearing(mut self, item: &ItemId, disguises: &[FactionId]) -> Self {
        self.equipped
            .insert(item.clone(), disguises.iter().cloned().collect());
        self
    }

    pub fn ending_unlocked(&self, ending: &EndingId) -> bool {
        self.endings.contains(ending)
    }

    pub fn is_blocked(&self, quest: &QuestId) -> bool {
        self.blocked.contains(quest)
    }

    pub fn xp(&self) -> f64 {
        self.xp
    }

    pub fn points(&self, faction: &FactionId) -> (f64, f64) {
        self.points.get(faction).copied().unwrap_or_default()
    }

    fn quest_entry(&mut self, quest: &QuestId) -> &mut (QuestStatus, Option<QuestStage>) {
        self.quests
            .entry(quest.clone())
            .or_insert((QuestStatus::NotStarted, None))
    }

    fn npc_mut(&mut self, npc: &NpcId) -> Result<&mut Npc, EffectError> {
        self.npcs.get_mut(npc).ok_or_else(|| EffectError::Unknown {
            kind: "npc",
            id: npc.as_str().to_owned(),
        })
    }
}

impl QuestView for MemoryState {
    fn quest_status(&self, quest: &QuestId) -> QuestStatus {
        self.quests
            .get(quest)
            .map_or(QuestStatus::NotStarted, |(status, _)| *status)
    }

    fn quest_stage(&self, quest: &QuestId) -> Option<QuestStage> {
        self.quests.get(quest).and_then(|(_, stage)| *stage)
    }
}

impl StandingView for MemoryState {
    fn reputation(&self, faction: &FactionId) -> ReputationLevel {
        self.levels
            .get(faction)
            .copied()
            .unwrap_or(ReputationLevel::Neutral)
    }

    fn alignment(&self) -> Alignment {
        if self.karma > 50.0 {
            Alignment::Good
        } else if self.karma < -50.0 {
            Alignment::Evil
        } else {
            Alignment::Neutral
        }
    }
}

impl CharacterView for MemoryState {
    fn skill(&self, skill: Skill) -> SkillLevel {
        self.skills.get(&skill).copied().unwrap_or_default()
    }

    fn special(&self, stat: Special) -> SpecialLevel {
        self.specials.get(&stat).copied().unwrap_or_default()
    }

    fn has_perk(&self, perk: &PerkId) -> bool {
        self.perks.contains(perk)
    }

    fn health(&self) -> HealthPercent {
        self.health.unwrap_or(HealthPercent::FULL)
    }
}

impl InventoryView for MemoryState {
    fn item_count(&self, item: &ItemId) -> u32 {
        self.items.get(item).copied().unwrap_or(0)
    }

    fn caps(&self) -> u32 {
        self.caps.floor() as u32
    }

    fn is_equipped(&self, item: &ItemId) -> bool {
        self.equipped.contains_key(item)
    }

    fn disguises_as(&self, item: &ItemId, faction: &FactionId) -> bool {
        self.equipped
            .get(item)
            .is_some_and(|factions| factions.contains(faction))
    }
}

impl CompanionView for MemoryState {
    fn companion_active(&self, companion: &CompanionId) -> bool {
        self.companions.contains(companion)
    }
}

impl WorldView for MemoryState {
    fn flag(&self, flag: &FlagId) -> bool {
        self.flags.contains(flag)
    }

    fn npc_is(&self, npc: &NpcId, state: NpcState) -> bool {
        self.npcs.get(npc).is_some_and(|known| match state {
            NpcState::Alive => !known.dead && !known.removed,
            NpcState::Dead => known.dead,
            NpcState::Hostile => known.hostile,
            // Nothing in this stand-in models meeting someone.
            NpcState::Met => false,
        })
    }
}

impl QuestSink for MemoryState {
    fn start_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        if self.blocked.contains(quest) {
            return Err(EffectError::Refused {
                effect: "start_quest",
                reason: format!("{} is blocked", quest.as_str()),
            });
        }
        let entry = self.quest_entry(quest);
        if entry.0 == QuestStatus::NotStarted {
            entry.0 = QuestStatus::Active;
        }
        Ok(())
    }

    fn complete_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quest_entry(quest).0 = QuestStatus::Completed;
        Ok(())
    }

    fn fail_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quest_entry(quest).0 = QuestStatus::Failed;
        Ok(())
    }

    fn block_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.blocked.insert(quest.clone());
        self.fail_quest(quest)
    }

    fn set_stage(&mut self, quest: &QuestId, stage: QuestStage) -> Result<(), EffectError> {
        let entry = self.quest_entry(quest);
        entry.1 = Some(stage);
        if entry.0 == QuestStatus::NotStarted {
            entry.0 = QuestStatus::Active;
        }
        Ok(())
    }
}

impl StandingSink for MemoryState {
    fn add_reputation(
        &mut self,
        faction: &FactionId,
        fame: f64,
        infamy: f64,
    ) -> Result<(), EffectError> {
        let points = self.points.entry(faction.clone()).or_default();
        points.0 += fame;
        points.1 += infamy;
        Ok(())
    }

    fn set_reputation(
        &mut self,
        faction: &FactionId,
        level: ReputationLevel,
    ) -> Result<(), EffectError> {
        self.levels.insert(faction.clone(), level);
        Ok(())
    }

    fn add_karma(&mut self, amount: f64) -> Result<(), EffectError> {
        self.karma += amount;
        Ok(())
    }
}

impl CharacterSink for MemoryState {
    fn add_xp(&mut self, amount: f64) -> Result<(), EffectError> {
        self.xp += amount;
        Ok(())
    }

    fn restore_health(&mut self) -> Result<(), EffectError> {
        self.health = None;
        Ok(())
    }
}

impl InventorySink for MemoryState {
    fn add_caps(&mut self, amount: f64) -> Result<(), EffectError> {
        if self.caps + amount < 0.0 {
            return Err(EffectError::NotEnoughCaps {
                have: self.caps(),
                change: amount,
            });
        }
        self.caps += amount;
        Ok(())
    }

    fn give_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError> {
        *self.items.entry(item.clone()).or_insert(0) += count.get();
        Ok(())
    }

    fn take_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError> {
        let have = self.item_count(item);
        if have < count.get() {
            return Err(EffectError::NotEnoughItems {
                item: item.clone(),
                have,
                need: count.get(),
            });
        }
        self.items.insert(item.clone(), have - count.get());
        Ok(())
    }
}

impl CompanionSink for MemoryState {
    fn companion_join(&mut self, companion: &CompanionId) -> Result<(), EffectError> {
        self.companions.insert(companion.clone());
        Ok(())
    }

    fn companion_leave(&mut self, companion: &CompanionId) -> Result<(), EffectError> {
        self.companions.remove(companion);
        Ok(())
    }
}

impl WorldSink for MemoryState {
    fn set_flag(&mut self, flag: &FlagId, value: bool) -> Result<(), EffectError> {
        if value {
            self.flags.insert(flag.clone());
        } else {
            self.flags.remove(flag);
        }
        Ok(())
    }

    fn kill_npc(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.npc_mut(npc)?.dead = true;
        Ok(())
    }

    fn set_hostile(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.npc_mut(npc)?.hostile = true;
        Ok(())
    }

    fn set_friendly(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.npc_mut(npc)?.hostile = false;
        Ok(())
    }

    fn remove_npc(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.npc_mut(npc)?.removed = true;
        Ok(())
    }

    fn unlock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError> {
        self.endings.insert(ending.clone());
        Ok(())
    }

    fn lock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError> {
        self.endings.remove(ending);
        Ok(())
    }
}

/// A state that answers each question from a hash of the question and a seed.
/// The answers are arbitrary but stable, so the same question always gets the
/// same answer, which is all the laws of the evaluator need.
#[derive(Debug, Clone, Copy)]
pub struct HashedView(pub u64);

impl HashedView {
    fn bits(&self, question: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        question.hash(&mut hasher);
        hasher.finish()
    }

    fn below(&self, question: &impl Hash, bound: u64) -> u32 {
        u32::try_from(self.bits(question) % bound).expect("bound fits in u32")
    }
}

impl QuestView for HashedView {
    fn quest_status(&self, quest: &QuestId) -> QuestStatus {
        match self.below(&("status", quest), 4) {
            0 => QuestStatus::NotStarted,
            1 => QuestStatus::Active,
            2 => QuestStatus::Completed,
            _ => QuestStatus::Failed,
        }
    }

    fn quest_stage(&self, quest: &QuestId) -> Option<QuestStage> {
        let n = self.below(&("stage", quest), 1_001);
        (n < 1_000).then(|| QuestStage::new(n))
    }
}

impl StandingView for HashedView {
    fn reputation(&self, faction: &FactionId) -> ReputationLevel {
        let n = self.below(&("rep", faction), ReputationLevel::iter().count() as u64);
        ReputationLevel::iter()
            .nth(n as usize)
            .expect("index is below the variant count")
    }

    fn alignment(&self) -> Alignment {
        match self.below(&"karma", 3) {
            0 => Alignment::Good,
            1 => Alignment::Neutral,
            _ => Alignment::Evil,
        }
    }
}

impl CharacterView for HashedView {
    fn skill(&self, skill: Skill) -> SkillLevel {
        SkillLevel::try_new(self.below(&("skill", skill), 101)).expect("below 101")
    }

    fn special(&self, stat: Special) -> SpecialLevel {
        SpecialLevel::try_new(1 + self.below(&("special", stat), 10)).expect("between 1 and 10")
    }

    fn has_perk(&self, perk: &PerkId) -> bool {
        self.below(&("perk", perk), 2) == 1
    }

    fn health(&self) -> HealthPercent {
        HealthPercent::try_new(self.below(&"health", 101)).expect("below 101")
    }
}

impl InventoryView for HashedView {
    fn item_count(&self, item: &ItemId) -> u32 {
        self.below(&("count", item), 6)
    }

    fn caps(&self) -> u32 {
        self.below(&"caps", 500)
    }

    fn is_equipped(&self, item: &ItemId) -> bool {
        self.below(&("worn", item), 2) == 1
    }

    fn disguises_as(&self, item: &ItemId, faction: &FactionId) -> bool {
        self.below(&("disguise", item, faction), 2) == 1
    }
}

impl CompanionView for HashedView {
    fn companion_active(&self, companion: &CompanionId) -> bool {
        self.below(&("companion", companion), 2) == 1
    }
}

impl WorldView for HashedView {
    fn flag(&self, flag: &FlagId) -> bool {
        self.below(&("flag", flag), 2) == 1
    }

    fn npc_is(&self, npc: &NpcId, state: NpcState) -> bool {
        self.below(&("npc", npc, state), 2) == 1
    }
}
