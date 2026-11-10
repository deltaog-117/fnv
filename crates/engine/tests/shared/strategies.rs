//! `proptest` strategies that build arbitrary valid model values.

use std::num::{NonZeroU16, NonZeroU32};

use engine::shared::{
    Ailment, Alignment, ArmorClass, CompanionId, Comparison, Condition, Effect, EndingId,
    FactionId, FlagId, FoeKind, HealthPercent, ItemId, MinuteOfDay, Modifier, NonEmpty, NpcId,
    NpcState, PerkId, QuestCondition, QuestEvent, QuestId, QuestStage, QuestState, Rads, Renown,
    RenownTier, ReputationLevel, Skill, SkillLevel, Special, SpecialLevel, TimeWindow,
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

fn quest_event() -> impl Strategy<Value = QuestEvent> {
    prop_oneof![
        Just(QuestEvent::Started),
        Just(QuestEvent::Completed),
        Just(QuestEvent::Failed),
        stage().prop_map(QuestEvent::Stage),
    ]
}

fn armor_class() -> impl Strategy<Value = Option<ArmorClass>> {
    proptest::option::of(select(vec![
        ArmorClass::Light,
        ArmorClass::Medium,
        ArmorClass::Heavy,
        ArmorClass::Power,
    ]))
}

fn count() -> impl Strategy<Value = NonZeroU32> {
    (1u32..1_000).prop_map(|n| NonZeroU32::new(n).expect("range starts at one"))
}

/// Finite numbers only: JSON cannot carry NaN or infinity.
fn amount() -> impl Strategy<Value = f64> {
    -1.0e6f64..1.0e6f64
}

/// A minute of the day.
pub fn minute_of_day() -> impl Strategy<Value = MinuteOfDay> {
    (0u32..24, 0u32..60).prop_map(|(hour, minute)| {
        MinuteOfDay::new(hour, minute).expect("hour and minute are in range")
    })
}

/// A window of the day that begins and ends at different minutes.
pub fn time_window() -> impl Strategy<Value = TimeWindow> {
    (minute_of_day(), minute_of_day())
        .prop_filter("both ends may not be the same minute", |(from, to)| {
            from != to
        })
        .prop_map(|(from, to)| TimeWindow::new(from, to).expect("the ends differ"))
}

fn leaf_condition() -> impl Strategy<Value = Condition> {
    let first = prop_oneof![
        (quest_id(), quest_state())
            .prop_map(|(quest, state)| Condition::Quest(QuestCondition::new(quest, state))),
        (quest_id(), quest_event(), count())
            .prop_map(|(quest, event, days)| { Condition::DaysSince { quest, event, days } }),
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
        (0u32..=100, any_of::<Comparison>()).prop_map(|(percent, op)| Condition::Health {
            percent: HealthPercent::try_new(percent).expect("range is valid"),
            op,
        }),
        (0u32..=999).prop_map(|min| Condition::Radiation {
            min: Rads::try_new(min).expect("range is valid"),
        }),
        Just(Condition::Addicted {}),
        time_window().prop_map(Condition::TimeOfDay),
    ]
    .boxed();
    let second = prop_oneof![
        (
            faction_id(),
            any_of::<Renown>(),
            0u32..=3,
            any_of::<Comparison>()
        )
            .prop_map(|(faction, renown, tier, op)| Condition::ReputationTier {
                faction,
                renown,
                tier: RenownTier::try_new(tier).expect("range is valid"),
                op,
            }),
        (faction_id(), any_of::<Renown>(), 0u32..=1_000).prop_map(|(faction, renown, min)| {
            Condition::ReputationPoints {
                faction,
                renown,
                min,
            }
        }),
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
        Just(Effect::Heal {}),
        any_of::<Ailment>().prop_map(|ailment| Effect::Cure { ailment }),
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

/// Any modifier, with amounts across the whole range each field allows and each
/// condition present or absent.
pub fn modifier() -> impl Strategy<Value = Modifier> {
    let skill = || proptest::option::of(any_of::<Skill>());
    let foe = || proptest::option::of(any_of::<FoeKind>());
    let sheet = prop_oneof![
        any::<u8>().prop_map(|amount| Modifier::SkillPointsPerLevel { amount }),
        any::<i8>().prop_map(|percent| Modifier::XpPercent { percent }),
        any::<u16>().prop_map(|pounds| Modifier::CarryWeight { pounds }),
        any::<u16>().prop_map(|amount| Modifier::MaxHp { amount }),
        any::<u8>().prop_map(|amount| Modifier::ActionPoints { amount }),
        any::<i8>().prop_map(|amount| Modifier::AllSkills { amount }),
        any::<i8>().prop_map(|amount| Modifier::AllSpecial { amount }),
        (any_of::<Special>(), any::<i8>())
            .prop_map(|(stat, amount)| Modifier::Special { stat, amount }),
        (any_of::<Skill>(), any::<i8>())
            .prop_map(|(skill, amount)| Modifier::Skill { skill, amount }),
    ]
    .boxed();
    let armor = prop_oneof![
        any::<i8>().prop_map(|amount| Modifier::DamageThreshold { amount }),
        any::<i8>().prop_map(|percent| Modifier::DamageResistance { percent }),
        any::<u8>().prop_map(|amount| Modifier::IgnoreThreshold { amount }),
        any::<i8>().prop_map(|percent| Modifier::WeaponWearPercent { percent }),
        any::<i8>().prop_map(|percent| Modifier::ReloadApPercent { percent }),
    ]
    .boxed();
    let weapon = prop_oneof![
        (any::<i8>(), skill(), foe()).prop_map(|(percent, skill, foe)| Modifier::DamagePercent {
            percent,
            skill,
            foe
        }),
        (any::<i8>(), skill(), armor_class()).prop_map(|(percent, skill, armor)| {
            Modifier::CritChance {
                percent,
                skill,
                armor,
            }
        }),
        (any::<i8>(), armor_class())
            .prop_map(|(percent, armor)| Modifier::CritChanceAgainst { percent, armor }),
        (
            0u32..=100,
            any_of::<Special>(),
            any::<i8>(),
            1u16..=u16::MAX
        )
            .prop_map(|(below, stat, amount, minutes)| Modifier::AfterFightBoost {
                below_health: HealthPercent::try_new(below).expect("range is valid"),
                stat,
                amount,
                minutes: NonZeroU16::new(minutes).expect("range starts at one"),
            }),
        (any::<i8>(), skill(), foe()).prop_map(|(percent, skill, foe)| {
            Modifier::CritDamagePercent {
                percent,
                skill,
                foe,
            }
        }),
        (any::<i8>(), skill())
            .prop_map(|(percent, skill)| Modifier::ApCostPercent { percent, skill }),
        (any::<i8>(), skill())
            .prop_map(|(percent, skill)| Modifier::SpreadPercent { percent, skill }),
        (any::<i8>(), proptest::option::of(item_id()))
            .prop_map(|(percent, item)| Modifier::HealPercent { percent, item }),
    ]
    .boxed();
    prop_oneof![sheet, armor, weapon]
}
