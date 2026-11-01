//! Each effect on the standing, and what it refuses.

use engine::shared::{Alignment, EffectError, Renown, ReputationLevel, StandingSink, StandingView};

use crate::support::{id, standing, tier};

#[test]
fn gaining_fame_climbs_the_tiers_and_changes_the_title() {
    let mut standing = standing();
    let town = id("town");
    assert_eq!(standing.title(&town), Some("Neutral"));

    standing.add_reputation(&town, 3.0, 0.0).expect("gains");
    assert_eq!(standing.renown_tier(&town, Renown::Fame), tier(1));
    assert_eq!(standing.reputation(&town), ReputationLevel::Accepted);
    assert_eq!(standing.title(&town), Some("Accepted"));

    standing.add_reputation(&town, 0.0, 15.0).expect("gains");
    assert_eq!(standing.reputation(&town), ReputationLevel::Mixed);
    assert_eq!(standing.title(&town), Some("Merciful Thug"));
}

#[test]
fn gains_add_up() {
    let mut standing = standing();
    let town = id("town");
    standing.add_reputation(&town, 1.5, 2.0).expect("gains");
    standing.add_reputation(&town, 1.5, 0.5).expect("gains");
    assert_eq!(standing.renown_points(&town, Renown::Fame), 3.0);
    assert_eq!(standing.renown_points(&town, Renown::Infamy), 2.5);
    assert_eq!(standing.renown_tier(&town, Renown::Fame), tier(1));
    assert_eq!(standing.renown_tier(&town, Renown::Infamy), tier(0));
}

#[test]
fn an_unknown_or_unmodeled_faction_is_refused_and_reads_as_neutral() {
    let mut standing = standing();
    for name in ["gang", "nowhere"] {
        let faction = id(name);
        assert!(matches!(
            standing.add_reputation(&faction, 1.0, 0.0),
            Err(EffectError::Unknown { .. })
        ));
        assert!(matches!(
            standing.set_reputation(&faction, ReputationLevel::Liked),
            Err(EffectError::Unknown { .. })
        ));
        assert_eq!(standing.reputation(&faction), ReputationLevel::Neutral);
        assert_eq!(standing.renown_tier(&faction, Renown::Fame), tier(0));
        assert_eq!(standing.renown_points(&faction, Renown::Infamy), 0.0);
        assert_eq!(standing.title(&faction), None);
    }
}

#[test]
fn a_loss_is_refused_and_a_half_valid_gain_changes_nothing() {
    let mut standing = standing();
    let town = id("town");
    standing.add_reputation(&town, 4.0, 4.0).expect("gains");
    for (fame, infamy) in [(-1.0, 0.0), (5.0, -0.5), (-0.1, -0.1)] {
        assert!(matches!(
            standing.add_reputation(&town, fame, infamy),
            Err(EffectError::Refused { .. })
        ));
    }
    assert_eq!(standing.renown_points(&town, Renown::Fame), 4.0);
    assert_eq!(standing.renown_points(&town, Renown::Infamy), 4.0);
}

#[test]
fn a_number_that_is_not_finite_is_refused() {
    let mut standing = standing();
    let town = id("town");
    for (fame, infamy) in [(f64::NAN, 0.0), (0.0, f64::INFINITY)] {
        assert!(matches!(
            standing.add_reputation(&town, fame, infamy),
            Err(EffectError::NotFinite { .. })
        ));
    }
    assert!(matches!(
        standing.add_karma(f64::NAN),
        Err(EffectError::NotFinite { .. })
    ));
    assert_eq!(standing.renown_points(&town, Renown::Fame), 0.0);
    assert_eq!(standing.karma().get(), 0.0);
}

#[test]
fn a_score_stops_at_the_cap() {
    let mut standing = standing();
    let town = id("town");
    for _ in 0..3 {
        standing.add_reputation(&town, 9_000.0, 0.0).expect("gains");
    }
    assert_eq!(
        standing.renown_points(&town, Renown::Fame),
        engine::reputation::MAX_POINTS
    );
    assert_eq!(standing.renown_tier(&town, Renown::Fame), tier(3));
}

#[test]
fn forcing_a_standing_replaces_both_scores_and_can_lower_them() {
    let mut standing = standing();
    let town = id("town");
    standing.add_reputation(&town, 40.0, 40.0).expect("gains");
    standing
        .set_reputation(&town, ReputationLevel::Neutral)
        .expect("forces");
    assert_eq!(standing.renown_points(&town, Renown::Fame), 0.0);
    assert_eq!(standing.renown_points(&town, Renown::Infamy), 0.0);
    assert_eq!(standing.reputation(&town), ReputationLevel::Neutral);
}

#[test]
fn karma_turns_good_or_evil_at_the_threshold_and_stops_at_the_edge() {
    let mut standing = standing();
    assert_eq!(standing.alignment(), Alignment::Neutral);

    standing.add_karma(249.0).expect("shifts");
    assert_eq!(standing.alignment(), Alignment::Neutral);
    standing.add_karma(1.0).expect("shifts");
    assert_eq!(standing.alignment(), Alignment::Good);

    standing.add_karma(-500.0).expect("shifts");
    assert_eq!(standing.alignment(), Alignment::Evil);

    standing.add_karma(-5_000.0).expect("shifts");
    assert_eq!(standing.karma().get(), -1000.0);
    standing.add_karma(1.0e9).expect("shifts");
    assert_eq!(standing.karma().get(), 1000.0);
}
