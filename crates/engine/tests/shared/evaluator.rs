//! The evaluator: what conditions mean, what effects do, and the laws that tie
//! them together.
//!
//! Examples pin the meaning of each condition. Property tests check the laws of
//! `all`/`any`/`not`, the ordering of comparisons, and that a batch of effects is
//! either applied or rejected without a trace.

use std::num::NonZeroU32;

use engine::shared::{
    Comparison, Condition, Effect, EffectError, EmptyWorld, EndingId, FactionId, FlagId, ItemId,
    NonEmpty, NpcId, NpcState, PerkId, QuestCondition, QuestId, QuestStage, QuestState,
    ReputationLevel, Skill, SkillLevel, apply, apply_all, evaluate,
};
use proptest::collection::vec;
use proptest::prelude::*;
use strum::IntoEnumIterator;

use super::memory::{HashedView, MemoryState};
use super::strategies;

fn quest(id: &str) -> QuestId {
    QuestId::try_new(id).expect("literal is a valid id")
}

fn item(id: &str) -> ItemId {
    ItemId::try_new(id).expect("literal is a valid id")
}

fn flag(id: &str) -> FlagId {
    FlagId::try_new(id).expect("literal is a valid id")
}

fn npc(id: &str) -> NpcId {
    NpcId::try_new(id).expect("literal is a valid id")
}

fn faction(id: &str) -> FactionId {
    FactionId::try_new(id).expect("literal is a valid id")
}

fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("tests pass a non-zero count")
}

fn quest_is(id: &str, state: QuestState) -> Condition {
    Condition::Quest(QuestCondition::new(quest(id), state))
}

fn all_of(of: Vec<Condition>) -> Condition {
    Condition::All {
        of: NonEmpty::try_new(of).expect("tests pass a non-empty list"),
    }
}

fn any_of(of: Vec<Condition>) -> Condition {
    Condition::Any {
        of: NonEmpty::try_new(of).expect("tests pass a non-empty list"),
    }
}

fn not(of: Condition) -> Condition {
    Condition::Not { of: Box::new(of) }
}

#[test]
fn an_empty_world_has_the_documented_defaults() {
    let world = EmptyWorld;
    let yes = [
        quest_is("q", QuestState::NotStarted),
        Condition::Flag {
            flag: flag("f"),
            value: false,
        },
        Condition::Skill {
            skill: Skill::Guns,
            min: SkillLevel::try_new(0).expect("in range"),
        },
        Condition::Companion {
            companion: engine::shared::CompanionId::try_new("c").expect("valid id"),
            active: false,
        },
        Condition::Reputation {
            faction: faction("ncr"),
            level: ReputationLevel::Neutral,
            op: Comparison::Is,
        },
        Condition::Karma {
            alignment: engine::shared::Alignment::Neutral,
        },
    ];
    for condition in &yes {
        assert!(evaluate(condition, &world), "{condition:?}");
    }
    let no = [
        quest_is("q", QuestState::Active),
        quest_is("q", QuestState::StageAtLeast(QuestStage::new(0))),
        Condition::Perk {
            perk: PerkId::try_new("p").expect("valid id"),
        },
        Condition::HasItem {
            item: item("i"),
            count: count(1),
        },
        Condition::Npc {
            npc: npc("n"),
            state: NpcState::Alive,
        },
    ];
    for condition in &no {
        assert!(!evaluate(condition, &world), "{condition:?}");
    }
}

#[test]
fn quest_states_follow_the_effects_that_move_them() {
    let mut state = MemoryState::default();
    let q = quest("ghost_town_gunfight");
    assert!(evaluate(
        &quest_is("ghost_town_gunfight", QuestState::NotStarted),
        &state
    ));

    apply(&Effect::StartQuest { quest: q.clone() }, &mut state).expect("start applies");
    assert!(evaluate(
        &quest_is("ghost_town_gunfight", QuestState::Active),
        &state
    ));

    apply(
        &Effect::SetStage {
            quest: q.clone(),
            stage: QuestStage::new(20),
        },
        &mut state,
    )
    .expect("set_stage applies");
    let at = |stage| quest_is("ghost_town_gunfight", stage);
    assert!(evaluate(
        &at(QuestState::StageIs(QuestStage::new(20))),
        &state
    ));
    assert!(evaluate(
        &at(QuestState::StageAtLeast(QuestStage::new(10))),
        &state
    ));
    assert!(!evaluate(
        &at(QuestState::StageAtLeast(QuestStage::new(30))),
        &state
    ));

    apply(&Effect::CompleteQuest { quest: q }, &mut state).expect("complete applies");
    assert!(evaluate(&at(QuestState::Completed), &state));
    // A finished quest keeps its last stage.
    assert!(evaluate(
        &at(QuestState::StageIs(QuestStage::new(20))),
        &state
    ));
}

#[test]
fn a_blocked_quest_cannot_start() {
    let mut state = MemoryState::default();
    let q = quest("wild_card");
    apply(&Effect::BlockQuest { quest: q.clone() }, &mut state).expect("block applies");
    assert!(state.is_blocked(&q));
    assert!(evaluate(&quest_is("wild_card", QuestState::Failed), &state));
    let refused = apply(&Effect::StartQuest { quest: q }, &mut state);
    assert!(
        matches!(refused, Err(EffectError::Refused { .. })),
        "{refused:?}"
    );
}

#[test]
fn wearing_needs_the_item_and_the_disguise_when_a_faction_is_named() {
    let armor = item("ncr_ranger_armor");
    let state = MemoryState::default().wearing(&armor, &[faction("ncr")]);
    let wearing = |faction| Condition::Wearing {
        equipped: armor.clone(),
        faction,
    };
    assert!(evaluate(&wearing(None), &state));
    assert!(evaluate(&wearing(Some(faction("ncr"))), &state));
    assert!(!evaluate(&wearing(Some(faction("legion"))), &state));
    assert!(!evaluate(
        &Condition::Wearing {
            equipped: item("vault_suit"),
            faction: None,
        },
        &state
    ));
}

#[test]
fn reputation_comparisons_use_the_level_order() {
    let mut state = MemoryState::default();
    apply(
        &Effect::SetReputation {
            faction: faction("goodsprings"),
            level: ReputationLevel::Liked,
        },
        &mut state,
    )
    .expect("set_reputation applies");
    let check = |level, op| Condition::Reputation {
        faction: faction("goodsprings"),
        level,
        op,
    };
    assert!(evaluate(
        &check(ReputationLevel::Accepted, Comparison::AtLeast),
        &state
    ));
    assert!(evaluate(
        &check(ReputationLevel::Idolized, Comparison::AtMost),
        &state
    ));
    assert!(evaluate(
        &check(ReputationLevel::Liked, Comparison::Is),
        &state
    ));
    assert!(!evaluate(
        &check(ReputationLevel::Idolized, Comparison::AtLeast),
        &state
    ));
}

#[test]
fn skills_perks_and_npcs_are_read_from_their_owners() {
    let doc = npc("doc_mitchell");
    let state = MemoryState::default()
        .with_skill(Skill::Medicine, 40)
        .with_perk(&PerkId::try_new("toughness").expect("valid id"))
        .with_npc(&doc);
    let skill = |min| Condition::Skill {
        skill: Skill::Medicine,
        min: SkillLevel::try_new(min).expect("in range"),
    };
    assert!(evaluate(&skill(40), &state));
    assert!(!evaluate(&skill(41), &state));
    assert!(evaluate(
        &Condition::Perk {
            perk: PerkId::try_new("toughness").expect("valid id"),
        },
        &state
    ));
    assert!(evaluate(
        &Condition::Npc {
            npc: doc,
            state: NpcState::Alive,
        },
        &state
    ));
}

#[test]
fn a_batch_reports_the_effect_that_failed_and_how_many_took_hold() {
    let mut state = MemoryState::default().with_npc(&npc("sunny_smiles"));
    let effects = [
        Effect::SetFlag {
            flag: flag("met_sunny"),
            value: true,
        },
        Effect::KillNpc { npc: npc("nobody") },
        Effect::SetFlag {
            flag: flag("after"),
            value: true,
        },
    ];
    let error = apply_all(&effects, &mut state).expect_err("the second effect names nobody");
    assert_eq!((error.index, error.applied, error.tag), (1, 1, "kill_npc"));
    assert!(matches!(
        error.source,
        EffectError::Unknown { kind: "npc", .. }
    ));
    let set = |name| Condition::Flag {
        flag: flag(name),
        value: true,
    };
    assert!(evaluate(&set("met_sunny"), &state));
    assert!(!evaluate(&set("after"), &state));
}

#[test]
fn a_batch_that_overspends_changes_nothing() {
    let mut state = MemoryState::default();
    let effects = [
        Effect::SetFlag {
            flag: flag("paid"),
            value: true,
        },
        Effect::GiveItem {
            item: item("stimpak"),
            count: count(1),
        },
        Effect::TakeItem {
            item: item("stimpak"),
            count: count(2),
        },
    ];
    let error = apply_all(&effects, &mut state).expect_err("two taken, one held");
    assert_eq!((error.index, error.applied), (2, 0));
    assert!(matches!(
        error.source,
        EffectError::NotEnoughItems {
            have: 1,
            need: 2,
            ..
        }
    ));
    assert_eq!(state, MemoryState::default());
}

#[test]
fn a_take_may_follow_a_give_in_the_same_batch() {
    let mut state = MemoryState::default();
    let effects = [
        Effect::GiveItem {
            item: item("stimpak"),
            count: count(2),
        },
        Effect::TakeItem {
            item: item("stimpak"),
            count: count(2),
        },
    ];
    apply_all(&effects, &mut state).expect("the player holds two by the time they are taken");
}

#[test]
fn amounts_that_are_not_finite_are_rejected_before_anything_changes() {
    let bad = [
        Effect::Karma { amount: f64::NAN },
        Effect::Xp {
            amount: f64::INFINITY,
        },
        Effect::Caps {
            amount: f64::NEG_INFINITY,
        },
        Effect::Reputation {
            faction: faction("ncr"),
            fame: 1.0,
            infamy: f64::NAN,
        },
    ];
    for effect in bad {
        let mut state = MemoryState::default();
        let batch = [
            Effect::SetFlag {
                flag: flag("first"),
                value: true,
            },
            effect.clone(),
        ];
        let error = apply_all(&batch, &mut state).expect_err("non-finite amount");
        assert!(
            matches!(error.source, EffectError::NotFinite { .. }),
            "{effect:?}"
        );
        assert_eq!(error.applied, 0);
        assert_eq!(state, MemoryState::default());
    }
}

#[test]
fn spending_more_caps_than_held_is_rejected() {
    let mut state = MemoryState::default();
    apply(&Effect::Caps { amount: 10.0 }, &mut state).expect("caps added");
    let error = apply_all(&[Effect::Caps { amount: -11.0 }], &mut state).expect_err("too poor");
    assert!(matches!(
        error.source,
        EffectError::NotEnoughCaps { have: 10, .. }
    ));
}

proptest! {
    #[test]
    fn not_not_is_the_identity(seed: u64, condition in strategies::condition()) {
        let view = HashedView(seed);
        prop_assert_eq!(evaluate(&not(not(condition.clone())), &view), evaluate(&condition, &view));
    }

    #[test]
    fn all_and_any_agree_with_the_iterator_methods(
        seed: u64,
        conditions in vec(strategies::condition(), 1..5),
    ) {
        let view = HashedView(seed);
        let results: Vec<bool> = conditions.iter().map(|c| evaluate(c, &view)).collect();
        prop_assert_eq!(evaluate(&all_of(conditions.clone()), &view), results.iter().all(|r| *r));
        prop_assert_eq!(evaluate(&any_of(conditions), &view), results.iter().any(|r| *r));
    }

    #[test]
    fn de_morgan_holds(seed: u64, conditions in vec(strategies::condition(), 1..5)) {
        let view = HashedView(seed);
        let negated: Vec<Condition> = conditions.iter().cloned().map(not).collect();
        prop_assert_eq!(
            evaluate(&not(all_of(conditions.clone())), &view),
            evaluate(&any_of(negated.clone()), &view)
        );
        prop_assert_eq!(
            evaluate(&not(any_of(conditions)), &view),
            evaluate(&all_of(negated), &view)
        );
    }

    #[test]
    fn a_lower_skill_requirement_holds_whenever_a_higher_one_does(
        seed: u64,
        skill in prop::sample::select(Skill::iter().collect::<Vec<_>>()),
        low in 0u32..=100,
        high in 0u32..=100,
    ) {
        let (low, high) = (low.min(high), low.max(high));
        let view = HashedView(seed);
        let needs = |min| Condition::Skill {
            skill,
            min: SkillLevel::try_new(min).expect("in range"),
        };
        if evaluate(&needs(high), &view) {
            prop_assert!(evaluate(&needs(low), &view));
        }
    }

    #[test]
    fn an_exact_standing_is_both_at_least_and_at_most(
        seed: u64,
        id in strategies::faction_id(),
        level in prop::sample::select(ReputationLevel::iter().collect::<Vec<_>>()),
    ) {
        let view = HashedView(seed);
        let check = |op| Condition::Reputation { faction: id.clone(), level, op };
        prop_assert_eq!(
            evaluate(&check(Comparison::Is), &view),
            evaluate(&check(Comparison::AtLeast), &view) && evaluate(&check(Comparison::AtMost), &view)
        );
    }

    #[test]
    fn being_exactly_at_a_stage_means_being_at_least_there(
        seed: u64,
        id in strategies::quest_id(),
        stage in 0u32..1_000,
    ) {
        let view = HashedView(seed);
        let at = |state| Condition::Quest(QuestCondition::new(id.clone(), state));
        if evaluate(&at(QuestState::StageIs(QuestStage::new(stage))), &view) {
            prop_assert!(evaluate(&at(QuestState::StageAtLeast(QuestStage::new(stage))), &view));
        }
    }

    #[test]
    fn a_flag_set_by_an_effect_reads_back(id in strategies::flag_id(), value: bool) {
        let mut state = MemoryState::default();
        apply(&Effect::SetFlag { flag: id.clone(), value }, &mut state).expect("set_flag applies");
        let read = Condition::Flag { flag: id, value };
        prop_assert!(evaluate(&read, &state));
    }

    #[test]
    fn items_given_are_held_and_items_taken_are_gone(
        id in strategies::item_id(),
        n in 1u32..1_000,
    ) {
        let mut state = MemoryState::default();
        let n = count(n);
        apply(&Effect::GiveItem { item: id.clone(), count: n }, &mut state).expect("give applies");
        let holds = Condition::HasItem { item: id.clone(), count: n };
        prop_assert!(evaluate(&holds, &state));
        apply(&Effect::TakeItem { item: id, count: n }, &mut state).expect("take applies");
        prop_assert!(!evaluate(&holds, &state));
    }

    #[test]
    fn a_batch_of_arbitrary_effects_never_leaves_a_partial_trace_it_does_not_report(
        effects in vec(strategies::effect(), 0..8),
    ) {
        let before = MemoryState::default();
        let mut state = before.clone();
        if let Err(error) = apply_all(&effects, &mut state) {
            // The state must equal "the first `applied` effects, applied one by one".
            let mut expected = before;
            for effect in &effects[..error.applied] {
                apply(effect, &mut expected).expect("these took hold the first time");
            }
            prop_assert_eq!(state, expected);
        }
    }

    #[test]
    fn the_preflight_accepts_exactly_the_batches_the_player_can_afford(
        steps in vec((0usize..3, 1u32..4, any::<bool>()), 0..10),
    ) {
        let pool = [item("a"), item("b"), item("c")];
        let effects: Vec<Effect> = steps
            .iter()
            .map(|&(which, n, give)| {
                let (item, count) = (pool[which].clone(), count(n));
                if give {
                    Effect::GiveItem { item, count }
                } else {
                    Effect::TakeItem { item, count }
                }
            })
            .collect();
        // An independent model of the running balance.
        let mut balance = [0i64; 3];
        let affordable = steps.iter().all(|&(which, n, give)| {
            balance[which] += if give { i64::from(n) } else { -i64::from(n) };
            balance[which] >= 0
        });
        let mut state = MemoryState::default();
        let outcome = apply_all(&effects, &mut state);
        prop_assert_eq!(outcome.is_ok(), affordable);
        if !affordable {
            prop_assert_eq!(state, MemoryState::default());
        }
    }
}

#[test]
fn every_ending_effect_reads_back() {
    let mut state = MemoryState::default();
    let ending = EndingId::try_new("house_always_wins").expect("valid id");
    apply(
        &Effect::UnlockEnding {
            ending: ending.clone(),
        },
        &mut state,
    )
    .expect("unlock applies");
    assert!(state.ending_unlocked(&ending));
    apply(
        &Effect::LockEnding {
            ending: ending.clone(),
        },
        &mut state,
    )
    .expect("lock applies");
    assert!(!state.ending_unlocked(&ending));
}

#[test]
fn xp_and_reputation_points_accumulate() {
    let mut state = MemoryState::default();
    let effects = [
        Effect::Xp { amount: 25.0 },
        Effect::Xp { amount: 10.0 },
        Effect::Reputation {
            faction: faction("ncr"),
            fame: 3.0,
            infamy: 1.0,
        },
        Effect::Reputation {
            faction: faction("ncr"),
            fame: 2.0,
            infamy: 0.0,
        },
    ];
    apply_all(&effects, &mut state).expect("all four apply");
    assert!((state.xp() - 35.0).abs() < f64::EPSILON);
    assert_eq!(state.points(&faction("ncr")), (5.0, 1.0));
}
