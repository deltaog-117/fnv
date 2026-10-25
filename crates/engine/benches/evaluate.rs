//! Benchmark stub for the condition evaluator, the hot path of dialogue, quest
//! and encounter checks.
//!
//! Evaluates a nested condition tree many times against an empty world and prints
//! the median time per evaluation. Run with `cargo bench -p engine --bench
//! evaluate`. Record the number as the baseline; a regression above 5% should
//! fail CI once a comparing harness (for example criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::shared::{
    Comparison, Condition, EmptyWorld, FactionId, FlagId, ItemId, NonEmpty, QuestCondition,
    QuestId, QuestStage, QuestState, ReputationLevel, Skill, SkillLevel, evaluate,
};

const RUNS: usize = 15;
const EVALUATIONS: u32 = 200_000;

fn list(items: Vec<Condition>) -> NonEmpty<Condition> {
    NonEmpty::try_new(items).expect("the list has elements")
}

/// A tree shaped like a real dialogue gate: a quest stage, a skill, a reputation
/// and an item, with one negation and one alternative.
fn gate() -> Condition {
    let id = |text: &str| text.to_owned();
    Condition::All {
        of: list(vec![
            Condition::Quest(QuestCondition::new(
                QuestId::try_new(id("ghost_town_gunfight")).expect("valid id"),
                QuestState::StageAtLeast(QuestStage::new(20)),
            )),
            Condition::Any {
                of: list(vec![
                    Condition::Skill {
                        skill: Skill::Speech,
                        min: SkillLevel::try_new(40).expect("in range"),
                    },
                    Condition::Reputation {
                        faction: FactionId::try_new(id("goodsprings")).expect("valid id"),
                        level: ReputationLevel::Liked,
                        op: Comparison::AtLeast,
                    },
                ]),
            },
            Condition::Not {
                of: Box::new(Condition::Flag {
                    flag: FlagId::try_new(id("powder_gangers_hostile")).expect("valid id"),
                    value: true,
                }),
            },
            Condition::HasItem {
                item: ItemId::try_new(id("stimpak")).expect("valid id"),
                count: std::num::NonZeroU32::MIN,
            },
        ]),
    }
}

fn median_run(condition: &Condition) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..EVALUATIONS {
                // Keep the result alive so the evaluation cannot be optimised away.
                std::hint::black_box(evaluate(
                    std::hint::black_box(condition),
                    std::hint::black_box(&EmptyWorld),
                ));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let total = median_run(&gate());
    println!(
        "evaluate gate: {:?} per evaluation ({EVALUATIONS} per run)",
        total / EVALUATIONS
    );
}
