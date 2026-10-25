//! Each rule of the quest lifecycle, one case at a time.

use engine::quests::{QuestBook, QuestBookError};
use engine::shared::{EffectError, QuestSink, QuestStatus, QuestView};

use crate::support::{id, log, quest, stage};

#[test]
fn a_new_quest_has_not_started() {
    let log = log();
    assert_eq!(log.quest_status(&id("q")), QuestStatus::NotStarted);
    assert_eq!(log.quest_stage(&id("q")), None);
}

#[test]
fn starting_makes_it_active_and_is_idempotent() {
    let mut log = log();
    log.start_quest(&id("q")).expect("starts");
    log.set_stage(&id("q"), stage(10)).expect("advances");
    log.start_quest(&id("q"))
        .expect("starting again is a no-op");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Active);
    assert_eq!(log.quest_stage(&id("q")), Some(stage(10)));
}

#[test]
fn reaching_a_stage_starts_an_unstarted_quest() {
    let mut log = log();
    log.set_stage(&id("q"), stage(5)).expect("advances");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Active);
}

#[test]
fn a_finish_stage_completes_and_the_stage_is_kept() {
    let mut log = log();
    log.set_stage(&id("q"), stage(30)).expect("advances");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Completed);
    assert_eq!(log.quest_stage(&id("q")), Some(stage(30)));
}

#[test]
fn stages_only_move_forward_but_may_skip() {
    let mut log = log();
    log.set_stage(&id("q"), stage(20)).expect("skips ahead");
    assert!(matches!(
        log.set_stage(&id("q"), stage(10)),
        Err(EffectError::Refused { .. })
    ));
    log.set_stage(&id("q"), stage(20))
        .expect("same stage is a no-op");
    assert_eq!(log.quest_stage(&id("q")), Some(stage(20)));
}

#[test]
fn an_undefined_stage_or_quest_is_unknown() {
    let mut log = log();
    assert!(matches!(
        log.set_stage(&id("q"), stage(7)),
        Err(EffectError::Unknown { .. })
    ));
    for result in [
        log.start_quest(&id("nope")),
        log.complete_quest(&id("nope")),
        log.fail_quest(&id("nope")),
        log.block_quest(&id("nope")),
        log.set_stage(&id("nope"), stage(5)),
    ] {
        assert!(matches!(result, Err(EffectError::Unknown { .. })));
    }
}

#[test]
fn only_an_active_quest_can_be_completed() {
    let mut log = log();
    assert!(matches!(
        log.complete_quest(&id("q")),
        Err(EffectError::Refused { .. })
    ));
    log.start_quest(&id("q")).expect("starts");
    log.complete_quest(&id("q")).expect("completes");
    log.complete_quest(&id("q"))
        .expect("completing again is a no-op");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Completed);
}

#[test]
fn failed_is_terminal() {
    let mut log = log();
    log.set_stage(&id("q"), stage(10)).expect("advances");
    log.fail_quest(&id("q")).expect("fails");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Failed);
    assert_eq!(log.quest_stage(&id("q")), Some(stage(10)));
    for result in [
        log.start_quest(&id("q")),
        log.complete_quest(&id("q")),
        log.set_stage(&id("q"), stage(20)),
    ] {
        assert!(matches!(result, Err(EffectError::Refused { .. })));
    }
}

#[test]
fn completed_is_terminal() {
    let mut log = log();
    log.set_stage(&id("q"), stage(30)).expect("completes");
    log.fail_quest(&id("q"))
        .expect("a late fail changes nothing");
    log.block_quest(&id("q"))
        .expect("a late lock-out changes nothing");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Completed);
    assert!(matches!(
        log.set_stage(&id("q"), stage(5)),
        Err(EffectError::Refused { .. })
    ));
}

#[test]
fn a_quest_can_be_locked_out_before_it_starts() {
    let mut log = log();
    log.block_quest(&id("q")).expect("blocks");
    assert_eq!(log.quest_status(&id("q")), QuestStatus::Failed);
    assert!(matches!(
        log.start_quest(&id("q")),
        Err(EffectError::Refused { .. })
    ));
}

#[test]
fn the_journal_lists_only_active_quests() {
    let book = QuestBook::new([
        &quest("a", &[(5, false), (10, true)]),
        &quest("b", &[(5, false)]),
    ])
    .expect("stages are in order");
    let mut log = engine::quests::QuestLog::new(std::sync::Arc::new(book));
    log.set_stage(&id("a"), stage(5)).expect("advances");
    log.set_stage(&id("b"), stage(5)).expect("advances");
    log.fail_quest(&id("b")).expect("fails");
    let active: Vec<_> = log.active().collect();
    assert_eq!(active, vec![(&id("a"), Some(stage(5)))]);
}

#[test]
fn a_clone_is_independent() {
    let mut log = log();
    let before = log.clone();
    log.set_stage(&id("q"), stage(10)).expect("advances");
    assert_eq!(before.quest_status(&id("q")), QuestStatus::NotStarted);
}

#[test]
fn the_book_rejects_unordered_stages_and_duplicate_quests() {
    assert_eq!(
        QuestBook::new([&quest("q", &[(10, false), (5, false)])]).unwrap_err(),
        QuestBookError::StagesOutOfOrder {
            quest: id("q"),
            stage: 5
        }
    );
    assert!(matches!(
        QuestBook::new([&quest("q", &[(5, false), (5, true)])]),
        Err(QuestBookError::StagesOutOfOrder { .. })
    ));
    let twice = quest("q", &[(5, true)]);
    assert!(matches!(
        QuestBook::new([&twice, &twice]),
        Err(QuestBookError::DuplicateQuest { .. })
    ));
}
