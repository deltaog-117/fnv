//! The one place that decides whether a [`Condition`] holds and applies an
//! [`Effect`].
//!
//! Dialogue options, quest rules and encounters all go through here, so a rule
//! means the same thing wherever it is written. Both matches are exhaustive: a new
//! vocabulary variant does not compile until it is handled.

use std::collections::HashMap;

use super::condition::{Condition, QuestCondition};
use super::effect::Effect;
use super::ids::ItemId;
use super::sink::{BatchError, EffectError, State, StateSink};
use super::view::{QuestStatus, StateView};
use super::vocabulary::{Comparison, QuestState};

/// Whether the condition holds in the given state.
///
/// # Examples
///
/// ```
/// use engine::shared::{evaluate, Condition, EmptyWorld, FlagId, ModelError};
///
/// let unset = Condition::Flag { flag: FlagId::try_new("met_doc")?, value: false };
/// assert!(evaluate(&unset, &EmptyWorld));
/// # Ok::<(), ModelError>(())
/// ```
#[must_use]
pub fn evaluate<V: StateView + ?Sized>(condition: &Condition, view: &V) -> bool {
    match condition {
        Condition::All { of } => of.iter().all(|inner| evaluate(inner, view)),
        Condition::Any { of } => of.iter().any(|inner| evaluate(inner, view)),
        Condition::Not { of } => !evaluate(of, view),
        Condition::Quest(quest) => quest_holds(quest, view),
        Condition::Reputation { faction, level, op } => {
            let standing = view.reputation(faction);
            match op {
                Comparison::AtLeast => standing >= *level,
                Comparison::AtMost => standing <= *level,
                Comparison::Is => standing == *level,
            }
        }
        Condition::Skill { skill, min } => view.skill(*skill) >= *min,
        Condition::Special { stat, min } => view.special(*stat) >= *min,
        Condition::Health { percent, op } => {
            let health = view.health();
            match op {
                Comparison::AtLeast => health >= *percent,
                Comparison::AtMost => health <= *percent,
                Comparison::Is => health == *percent,
            }
        }
        Condition::Perk { perk } => view.has_perk(perk),
        Condition::Karma { alignment } => view.alignment() == *alignment,
        Condition::Companion { companion, active } => view.companion_active(companion) == *active,
        Condition::HasItem { item, count } => view.item_count(item) >= count.get(),
        Condition::Wearing { equipped, faction } => {
            view.is_equipped(equipped)
                && faction
                    .as_ref()
                    .is_none_or(|faction| view.disguises_as(equipped, faction))
        }
        Condition::Flag { flag, value } => view.flag(flag) == *value,
        Condition::Npc { npc, state } => view.npc_is(npc, *state),
    }
}

fn quest_holds<V: StateView + ?Sized>(condition: &QuestCondition, view: &V) -> bool {
    let quest = &condition.quest;
    match condition.state {
        QuestState::NotStarted => view.quest_status(quest) == QuestStatus::NotStarted,
        QuestState::Active => view.quest_status(quest) == QuestStatus::Active,
        QuestState::Completed => view.quest_status(quest) == QuestStatus::Completed,
        QuestState::Failed => view.quest_status(quest) == QuestStatus::Failed,
        QuestState::StageAtLeast(stage) => view
            .quest_stage(quest)
            .is_some_and(|reached| reached >= stage),
        QuestState::StageIs(stage) => view.quest_stage(quest) == Some(stage),
    }
}

/// Applies one effect through the sink that owns the state it changes.
///
/// # Errors
///
/// [`EffectError::NotFinite`] when an amount is NaN or infinite; otherwise
/// whatever the owning sink reports.
pub fn apply<S: StateSink + ?Sized>(effect: &Effect, sink: &mut S) -> Result<(), EffectError> {
    check_finite(effect)?;
    match effect {
        Effect::StartQuest { quest } => sink.start_quest(quest),
        Effect::CompleteQuest { quest } => sink.complete_quest(quest),
        Effect::FailQuest { quest } => sink.fail_quest(quest),
        Effect::BlockQuest { quest } => sink.block_quest(quest),
        Effect::SetStage { quest, stage } => sink.set_stage(quest, *stage),
        Effect::Heal {} => sink.restore_health(),
        Effect::Reputation {
            faction,
            fame,
            infamy,
        } => sink.add_reputation(faction, *fame, *infamy),
        Effect::SetReputation { faction, level } => sink.set_reputation(faction, *level),
        Effect::Karma { amount } => sink.add_karma(*amount),
        Effect::Xp { amount } => sink.add_xp(*amount),
        Effect::Caps { amount } => sink.add_caps(*amount),
        Effect::GiveItem { item, count } => sink.give_item(item, *count),
        Effect::TakeItem { item, count } => sink.take_item(item, *count),
        Effect::SetFlag { flag, value } => sink.set_flag(flag, *value),
        Effect::CompanionJoin { companion } => sink.companion_join(companion),
        Effect::CompanionLeave { companion } => sink.companion_leave(companion),
        Effect::KillNpc { npc } => sink.kill_npc(npc),
        Effect::SetHostile { npc } => sink.set_hostile(npc),
        Effect::SetFriendly { npc } => sink.set_friendly(npc),
        Effect::RemoveNpc { npc } => sink.remove_npc(npc),
        Effect::UnlockEnding { ending } => sink.unlock_ending(ending),
        Effect::LockEnding { ending } => sink.lock_ending(ending),
    }
}

/// Applies a list of effects, all or (as far as the evaluator can tell) nothing.
///
/// The list is first checked against the current state: amounts must be finite,
/// and the player must hold the items and caps that the list takes, counting what
/// earlier effects in the same list give. Only then are the effects applied in
/// order. What the pre-flight cannot see, such as an id the owning feature does not
/// know, is reported by the sink mid-way; [`BatchError::applied`] says how many
/// effects had already taken hold.
///
/// # Errors
///
/// [`BatchError`] naming the failing effect.
pub fn apply_all<S: State + ?Sized>(effects: &[Effect], state: &mut S) -> Result<(), BatchError> {
    preflight(effects, &*state).map_err(|(index, source)| BatchError {
        index,
        tag: effects[index].tag(),
        applied: 0,
        source,
    })?;
    for (index, effect) in effects.iter().enumerate() {
        apply(effect, state).map_err(|source| BatchError {
            index,
            tag: effect.tag(),
            applied: index,
            source,
        })?;
    }
    Ok(())
}

/// Rejects an amount that is NaN or infinite. The wildcard is safe only because
/// every effect carrying an `f64` is named here; the unit tests cover each one.
fn check_finite(effect: &Effect) -> Result<(), EffectError> {
    let finite = match effect {
        Effect::Reputation { fame, infamy, .. } => fame.is_finite() && infamy.is_finite(),
        Effect::Karma { amount } | Effect::Xp { amount } | Effect::Caps { amount } => {
            amount.is_finite()
        }
        _ => true,
    };
    if finite {
        Ok(())
    } else {
        Err(EffectError::NotFinite {
            effect: effect.tag(),
        })
    }
}

fn preflight<V: StateView + ?Sized>(
    effects: &[Effect],
    view: &V,
) -> Result<(), (usize, EffectError)> {
    // Running balances, so a take that follows a give in the same list is checked
    // against what the player will hold by then.
    let mut held: HashMap<&ItemId, u64> = HashMap::new();
    let mut caps = f64::from(view.caps());
    for (index, effect) in effects.iter().enumerate() {
        check_finite(effect).map_err(|error| (index, error))?;
        match effect {
            Effect::GiveItem { item, count } => {
                *held
                    .entry(item)
                    .or_insert_with(|| u64::from(view.item_count(item))) += u64::from(count.get());
            }
            Effect::TakeItem { item, count } => {
                let balance = held
                    .entry(item)
                    .or_insert_with(|| u64::from(view.item_count(item)));
                let need = u64::from(count.get());
                if *balance < need {
                    return Err((
                        index,
                        EffectError::NotEnoughItems {
                            item: item.clone(),
                            have: u32::try_from(*balance).unwrap_or(u32::MAX),
                            need: count.get(),
                        },
                    ));
                }
                *balance -= need;
            }
            Effect::Caps { amount } => {
                caps += amount;
                if caps < 0.0 {
                    return Err((
                        index,
                        EffectError::NotEnoughCaps {
                            have: view.caps(),
                            change: *amount,
                        },
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
