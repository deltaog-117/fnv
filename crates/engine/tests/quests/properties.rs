//! The lifecycle laws, checked over random sequences of effects.

use engine::shared::{QuestSink, QuestStatus, QuestView};
use proptest::prelude::*;

use crate::support::{id, log, stage};

#[derive(Debug, Clone)]
enum Op {
    Start,
    Complete,
    Fail,
    Block,
    Stage(u32),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::Start),
        Just(Op::Complete),
        Just(Op::Fail),
        Just(Op::Block),
        // 7 and 40 are not defined stages, so refusals are exercised too.
        prop::sample::select(vec![5, 7, 10, 20, 30, 40]).prop_map(Op::Stage),
    ]
}

proptest! {
    #[test]
    fn the_laws_hold_after_every_effect(ops in prop::collection::vec(op(), 0..40)) {
        let quest = id("q");
        let mut log = log();
        for op in ops {
            let status = log.quest_status(&quest);
            let before = log.quest_stage(&quest);
            let _ = match op {
                Op::Start => log.start_quest(&quest),
                Op::Complete => log.complete_quest(&quest),
                Op::Fail => log.fail_quest(&quest),
                Op::Block => log.block_quest(&quest),
                Op::Stage(n) => log.set_stage(&quest, stage(n)),
            };
            let after_status = log.quest_status(&quest);
            let after = log.quest_stage(&quest);

            // Terminal states never change.
            if matches!(status, QuestStatus::Completed | QuestStatus::Failed) {
                prop_assert_eq!(after_status, status);
            }
            // A quest never goes back to not started.
            if status != QuestStatus::NotStarted {
                prop_assert_ne!(after_status, QuestStatus::NotStarted);
            }
            // Stages only move forward.
            if let Some(before) = before {
                prop_assert!(after.is_some_and(|after| after >= before));
            }
            // Completed through a stage means that stage is the finish stage.
            if after_status == QuestStatus::Completed
                && status == QuestStatus::Active
                && after != before
            {
                prop_assert_eq!(after, Some(stage(30)));
            }
            // Only defined stages are ever recorded.
            if let Some(after) = after {
                prop_assert!([5, 10, 20, 30].contains(&after.get()));
            }
        }
    }

    #[test]
    fn a_failed_effect_changes_nothing(ops in prop::collection::vec(op(), 0..40)) {
        let quest = id("q");
        let mut log = log();
        for op in ops {
            let before = log.clone();
            let result = match op {
                Op::Start => log.start_quest(&quest),
                Op::Complete => log.complete_quest(&quest),
                Op::Fail => log.fail_quest(&quest),
                Op::Block => log.block_quest(&quest),
                Op::Stage(n) => log.set_stage(&quest, stage(n)),
            };
            if result.is_err() {
                prop_assert_eq!(log.quest_status(&quest), before.quest_status(&quest));
                prop_assert_eq!(log.quest_stage(&quest), before.quest_stage(&quest));
            }
        }
    }
}
