//! `proptest` strategies that build arbitrary valid model values.

use std::num::NonZeroU32;

use engine::shared::{
    Alignment, CompanionId, Comparison, Condition, Effect, EndingId, FactionId, FlagId, ItemId,
    Modifier, NonEmpty, NpcId, NpcState, PerkId, QuestCondition, QuestId, QuestStage, QuestState,
    ReputationLevel, Skill, SkillLevel, Special, SpecialLevel,
};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;
use strum::IntoEnumIterator;

/// Text that matches the id pattern.
pub fn id_text() -> impl Strategy<Value = String> {
    "[a-z0-9][a-z0-9_]{0,24}"
}

macro_rules! id_strategy {
    ($name:ident, $ty:ty) => {
        pub fn $name() -> impl Strategy<Value = $ty> {
            id_text().prop_map(|text| <$ty>::try_new(text).expect("generated text is a valid id"))
        }
    };
}

id_strategy!(quest_id, QuestId);
id_strategy!(faction_id, FactionId);
id_strategy!(perk_id, PerkId);
id_strategy!(companion_id, CompanionId);
id_strategy!(item_id, ItemId);
id_strategy!(flag_id, FlagId);
id_strategy!(npc_id, NpcId);
id_strategy!(ending_id, EndingId);

fn any_of<E: IntoEnumIterator + Clone + std::fmt::Debug + 'static>() -> impl Strategy<Value = E> {
    select(E::iter().collect::<Vec<_>>())
}

fn stage() -> impl Strategy<Value = QuestStage> {
    (0u32..1_000).prop_map(QuestStage::new)
}

fn quest_state() -> impl Strategy<Value = QuestState> {
    prop_oneof![
        Just(QuestState::NotStarted),
        Just(QuestState::Active),
        Just(QuestState::Completed),
        Just(QuestState::Failed),
        stage().prop_map(QuestState::StageAtLeast),
        stage().prop_map(QuestState::StageIs),
    ]
}

fn count() -> impl Strategy<Value = NonZeroU32> {
    (1u32..1_000).prop_map(|n| NonZeroU32::new(n).expect("range starts at one"))
}

/// Finite numbers only: JSON cannot carry NaN or infinity.
fn amount() -> impl Strategy<Value = f64> {
    -1.0e6f64..1.0e6f64
}

fn leaf_condition() -> impl Strategy<Value = Condition> {
    let first = prop_oneof![
        (quest_id(), quest_state())
            .prop_map(|(quest, state)| Condition::Quest(QuestCondition::new(quest, state))),
        (
            faction_id(),
            any_of::<ReputationLevel>(),
            any_of::<Comparison>()
        )
            .prop_map(|(faction, level, op)| Condition::Reputation { faction, level, op }),
        (any_of::<Skill>(), 0u32..=100).prop_map(|(skill, min)| Condition::Skill {
            skill,
            min: SkillLevel::try_new(min).expect("range is valid"),
        }),
        (any_of::<Special>(), 1u32..=10).prop_map(|(stat, min)| Condition::Special {
            stat,
            min: SpecialLevel::try_new(min).expect("range is valid"),
        }),
        perk_id().prop_map(|perk| Condition::Perk { perk }),
    ]
    .boxed();
    let second = prop_oneof![
        any_of::<Alignment>().prop_map(|alignment| Condition::Karma { alignment }),
        (companion_id(), any::<bool>())
            .prop_map(|(companion, active)| Condition::Companion { companion, active }),
        (item_id(), count()).prop_map(|(item, count)| Condition::HasItem { item, count }),
        (item_id(), proptest::option::of(faction_id()))
            .prop_map(|(equipped, faction)| Condition::Wearing { equipped, faction }),
        (flag_id(), any::<bool>()).prop_map(|(flag, value)| Condition::Flag { flag, value }),
        (npc_id(), any_of::<NpcState>()).prop_map(|(npc, state)| Condition::Npc { npc, state }),
    ]
    .boxed();
    prop_oneof![first, second]
}

/// Arbitrary conditions, including nested `all`, `any` and `not`.
pub fn condition() -> impl Strategy<Value = Condition> {
    leaf_condition().prop_recursive(3, 24, 3, |inner| {
        prop_oneof![
            vec(inner.clone(), 1..4).prop_map(|items| Condition::All {
                of: NonEmpty::try_new(items).expect("length is at least one"),
            }),
            vec(inner.clone(), 1..4).prop_map(|items| Condition::Any {
                of: NonEmpty::try_new(items).expect("length is at least one"),
            }),
            inner.prop_map(|condition| Condition::Not {
                of: Box::new(condition),
            }),
        ]
    })
}

/// Arbitrary effects.
pub fn effect() -> impl Strategy<Value = Effect> {
    let quests = prop_oneof![
        quest_id().prop_map(|quest| Effect::StartQuest { quest }),
        quest_id().prop_map(|quest| Effect::CompleteQuest { quest }),
        quest_id().prop_map(|quest| Effect::FailQuest { quest }),
        quest_id().prop_map(|quest| Effect::BlockQuest { quest }),
        (quest_id(), stage()).prop_map(|(quest, stage)| Effect::SetStage { quest, stage }),
    ]
    .boxed();
    let standing = prop_oneof![
        (faction_id(), amount(), amount()).prop_map(|(faction, fame, infamy)| Effect::Reputation {
            faction,
            fame,
            infamy,
        }),
        (faction_id(), any_of::<ReputationLevel>())
            .prop_map(|(faction, level)| Effect::SetReputation { faction, level }),
        amount().prop_map(|amount| Effect::Karma { amount }),
        amount().prop_map(|amount| Effect::Xp { amount }),
        amount().prop_map(|amount| Effect::Caps { amount }),
    ]
    .boxed();
    let things = prop_oneof![
        (item_id(), count()).prop_map(|(item, count)| Effect::GiveItem { item, count }),
        (item_id(), count()).prop_map(|(item, count)| Effect::TakeItem { item, count }),
        (flag_id(), any::<bool>()).prop_map(|(flag, value)| Effect::SetFlag { flag, value }),
        companion_id().prop_map(|companion| Effect::CompanionJoin { companion }),
        companion_id().prop_map(|companion| Effect::CompanionLeave { companion }),
    ]
    .boxed();
    let world = prop_oneof![
        npc_id().prop_map(|npc| Effect::KillNpc { npc }),
        npc_id().prop_map(|npc| Effect::SetHostile { npc }),
        npc_id().prop_map(|npc| Effect::SetFriendly { npc }),
        npc_id().prop_map(|npc| Effect::RemoveNpc { npc }),
        ending_id().prop_map(|ending| Effect::UnlockEnding { ending }),
        ending_id().prop_map(|ending| Effect::LockEnding { ending }),
    ]
    .boxed();
    prop_oneof![quests, standing, things, world]
}

/// Any modifier, with amounts across the whole range each field allows.
pub fn modifier() -> impl Strategy<Value = Modifier> {
    prop_oneof![
        any::<u8>().prop_map(|amount| Modifier::SkillPointsPerLevel { amount }),
        any::<i8>().prop_map(|percent| Modifier::XpPercent { percent }),
        any::<u16>().prop_map(|pounds| Modifier::CarryWeight { pounds }),
        any::<u16>().prop_map(|amount| Modifier::MaxHp { amount }),
        any::<u8>().prop_map(|amount| Modifier::ActionPoints { amount }),
        any::<i8>().prop_map(|amount| Modifier::AllSkills { amount }),
        (any_of::<Skill>(), any::<i8>())
            .prop_map(|(skill, amount)| Modifier::Skill { skill, amount }),
    ]
}
