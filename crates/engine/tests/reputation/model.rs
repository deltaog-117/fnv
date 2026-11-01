//! The tiers, the seven standings and the book.

use engine::reputation::{ReputationBook, ReputationError, standing};
use engine::shared::{Faction, ReputationLevel};
use serde_json::json;

use crate::support::{book, grid, id, modeled, plain, tier};

#[test]
fn a_score_reaches_a_tier_exactly_at_its_threshold() {
    let book = book();
    let model = book.get(&id("town")).expect("the town is modeled");
    let cases = [
        (0.0, 0),
        (2.99, 0),
        (3.0, 1),
        (7.5, 1),
        (8.0, 2),
        (14.99, 2),
        (15.0, 3),
        (1.0e9, 3),
    ];
    for (points, expected) in cases {
        assert_eq!(model.tier(points), tier(expected), "{points} points");
    }
}

#[test]
fn the_seven_standings_follow_the_two_tiers() {
    use ReputationLevel::{Accepted, Idolized, Liked, Mixed, Neutral, Shunned, Vilified};
    // Rows are the infamy tier, columns the fame tier.
    let expected = [
        [Neutral, Accepted, Liked, Idolized],
        [Shunned, Mixed, Mixed, Mixed],
        [Shunned, Mixed, Mixed, Mixed],
        [Vilified, Mixed, Mixed, Mixed],
    ];
    for (infamy, row) in expected.iter().enumerate() {
        for (fame, level) in row.iter().enumerate() {
            let (fame, infamy) = (
                tier(u32::try_from(fame).expect("small")),
                tier(u32::try_from(infamy).expect("small")),
            );
            assert_eq!(
                standing(fame, infamy),
                *level,
                "fame {fame:?} infamy {infamy:?}"
            );
        }
    }
}

#[test]
fn the_title_is_read_from_the_grid_by_infamy_row_and_fame_column() {
    let book = book();
    let model = book.get(&id("town")).expect("the town is modeled");
    assert_eq!(model.title(tier(0), tier(0)), "Neutral");
    assert_eq!(model.title(tier(1), tier(1)), "Mixed");
    assert_eq!(model.title(tier(1), tier(3)), "Merciful Thug");
    assert_eq!(model.title(tier(3), tier(3)), "Wild Child");
    assert_eq!(model.title(tier(3), tier(0)), "Idolized");
}

#[test]
fn a_faction_without_a_model_is_left_out_of_the_book() {
    let book = book();
    assert_eq!(book.len(), 1);
    assert!(book.get(&id("gang")).is_none());
    assert!(book.get(&id("nowhere")).is_none());
}

#[test]
fn a_repeated_faction_id_is_refused() {
    let records = [plain("gang"), plain("gang")];
    assert_eq!(
        ReputationBook::new(&records).expect_err("the id repeats"),
        ReputationError::DuplicateFaction(id("gang"))
    );
}

#[test]
fn a_model_without_titles_or_thresholds_is_refused() {
    for reputation in [
        json!({"thresholds": [0, 3, 8, 15]}),
        json!({"titles": grid()}),
        json!({}),
    ] {
        let record: Faction = serde_json::from_value(json!({
            "id": "half", "name": "half", "kind": "town", "dlc": "base",
            "reputation": reputation,
        }))
        .expect("the literal is a valid faction record");
        assert_eq!(
            ReputationBook::new([&record]).expect_err("the model is incomplete"),
            ReputationError::Incomplete(id("half"))
        );
    }
}

#[test]
fn thresholds_must_start_at_zero_and_rise() {
    for thresholds in [[1, 2, 3, 4], [0, 3, 3, 9], [0, 5, 4, 9], [0, 0, 1, 2]] {
        let record = modeled("odd", thresholds);
        assert_eq!(
            ReputationBook::new([&record]).expect_err("the thresholds are wrong"),
            ReputationError::BadThresholds {
                faction: id("odd"),
                thresholds
            },
            "{thresholds:?}"
        );
    }
}

#[test]
fn forcing_a_standing_uses_the_fewest_points_that_reach_it() {
    let book = book();
    let model = book.get(&id("town")).expect("the town is modeled");
    assert_eq!(model.points_for(ReputationLevel::Neutral), (0.0, 0.0));
    assert_eq!(model.points_for(ReputationLevel::Liked), (8.0, 0.0));
    assert_eq!(model.points_for(ReputationLevel::Shunned), (0.0, 3.0));
    assert_eq!(model.points_for(ReputationLevel::Vilified), (0.0, 15.0));
    assert_eq!(model.points_for(ReputationLevel::Mixed), (3.0, 3.0));
}
