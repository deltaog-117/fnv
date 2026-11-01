//! The reputation laws, checked over random effects.

use std::sync::Arc;

use engine::reputation::{MAX_POINTS, ReputationBook, Standing, standing as level_of};
use engine::shared::{EffectError, Renown, ReputationLevel, StandingSink, StandingView};
use proptest::prelude::*;
use strum::IntoEnumIterator;

use crate::support::{id, modeled, tier};

/// Thresholds that start at zero and rise, as the book requires.
fn thresholds() -> impl Strategy<Value = [u32; 4]> {
    (1u32..40, 1u32..40, 1u32..40).prop_map(|(a, b, c)| [0, a, a + b, a + b + c])
}

fn standing_over(thresholds: [u32; 4]) -> Standing {
    let record = modeled("town", thresholds);
    Standing::new(Arc::new(
        ReputationBook::new([&record]).expect("valid thresholds build"),
    ))
}

fn snapshot(standing: &Standing) -> (f64, f64, f64) {
    let town = id("town");
    (
        standing.renown_points(&town, Renown::Fame),
        standing.renown_points(&town, Renown::Infamy),
        standing.karma().get(),
    )
}

#[derive(Debug, Clone)]
enum Op {
    Gain(f64, f64),
    Force(ReputationLevel),
    Karma(f64),
    /// Aimed at a faction that does not exist.
    Stray(f64, f64),
    /// A loss.
    Loss(f64),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0.0f64..60.0, 0.0f64..60.0).prop_map(|(f, i)| Op::Gain(f, i)),
        prop::sample::select(ReputationLevel::iter().collect::<Vec<_>>()).prop_map(Op::Force),
        (-3_000.0f64..3_000.0).prop_map(Op::Karma),
        (0.0f64..60.0, 0.0f64..60.0).prop_map(|(f, i)| Op::Stray(f, i)),
        (0.001f64..60.0).prop_map(Op::Loss),
    ]
}

fn run(standing: &mut Standing, op: &Op) -> Result<(), EffectError> {
    let town = id("town");
    match op {
        Op::Gain(fame, infamy) => standing.add_reputation(&town, *fame, *infamy),
        Op::Force(level) => standing.set_reputation(&town, *level),
        Op::Karma(amount) => standing.add_karma(*amount),
        Op::Stray(fame, infamy) => standing.add_reputation(&id("nowhere"), *fame, *infamy),
        Op::Loss(amount) => standing.add_reputation(&town, -*amount, 0.0),
    }
}

proptest! {
    #[test]
    fn scores_and_karma_stay_finite_and_in_range(
        thresholds in thresholds(),
        ops in prop::collection::vec(op(), 0..40),
    ) {
        let mut standing = standing_over(thresholds);
        for op in &ops {
            let _ = run(&mut standing, op);
            let (fame, infamy, karma) = snapshot(&standing);
            prop_assert!((0.0..=MAX_POINTS).contains(&fame));
            prop_assert!((0.0..=MAX_POINTS).contains(&infamy));
            prop_assert!((-1000.0..=1000.0).contains(&karma));
        }
    }

    #[test]
    fn a_refused_effect_changes_nothing(
        thresholds in thresholds(),
        ops in prop::collection::vec(op(), 0..40),
    ) {
        let mut standing = standing_over(thresholds);
        for op in &ops {
            let before = snapshot(&standing);
            if run(&mut standing, op).is_err() {
                prop_assert_eq!(snapshot(&standing), before, "{:?}", op);
            }
        }
    }

    #[test]
    fn gaining_never_lowers_a_score_or_a_tier(
        thresholds in thresholds(),
        gains in prop::collection::vec((0.0f64..60.0, 0.0f64..60.0), 0..40),
    ) {
        let mut standing = standing_over(thresholds);
        let town = id("town");
        for (fame, infamy) in gains {
            let before = (
                snapshot(&standing),
                standing.renown_tier(&town, Renown::Fame),
                standing.renown_tier(&town, Renown::Infamy),
            );
            standing.add_reputation(&town, fame, infamy).expect("gains are accepted");
            let (fame_points, infamy_points, _) = snapshot(&standing);
            prop_assert!(fame_points >= before.0.0);
            prop_assert!(infamy_points >= before.0.1);
            prop_assert!(standing.renown_tier(&town, Renown::Fame) >= before.1);
            prop_assert!(standing.renown_tier(&town, Renown::Infamy) >= before.2);
        }
    }

    #[test]
    fn a_forced_standing_reads_back_as_itself(
        thresholds in thresholds(),
        before in prop::collection::vec(op(), 0..10),
        level in prop::sample::select(ReputationLevel::iter().collect::<Vec<_>>()),
    ) {
        let mut standing = standing_over(thresholds);
        for op in &before {
            let _ = run(&mut standing, op);
        }
        standing.set_reputation(&id("town"), level).expect("the town is modeled");
        prop_assert_eq!(standing.reputation(&id("town")), level);
    }

    #[test]
    fn a_higher_score_never_has_a_lower_tier(
        thresholds in thresholds(),
        a in 0.0f64..200.0,
        b in 0.0f64..200.0,
    ) {
        let record = modeled("town", thresholds);
        let book = ReputationBook::new([&record]).expect("valid thresholds build");
        let model = book.get(&id("town")).expect("the town is modeled");
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        prop_assert!(model.tier(low) <= model.tier(high));
    }

    #[test]
    fn more_fame_with_no_infamy_never_lowers_the_standing(
        low in 0u32..=3,
        high in 0u32..=3,
    ) {
        prop_assume!(low <= high);
        prop_assert!(level_of(tier(low), tier(0)) <= level_of(tier(high), tier(0)));
    }

    #[test]
    fn karma_is_good_exactly_from_the_threshold_up(amount in -1_500.0f64..1_500.0) {
        let mut standing = standing_over([0, 1, 2, 3]);
        standing.add_karma(amount).expect("finite amounts are accepted");
        let karma = standing.karma().get();
        prop_assert_eq!(
            standing.alignment() == engine::shared::Alignment::Good,
            karma >= 250.0
        );
        prop_assert_eq!(
            standing.alignment() == engine::shared::Alignment::Evil,
            karma <= -250.0
        );
    }
}
