//! The reputation book built from `data/engine/`.

use std::path::Path;

use engine::reputation::{ReputationBook, standing};
use engine::shared::{GameData, ReputationLevel, Scope};

use crate::support::{id, tier};

fn real_book() -> ReputationBook {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    ReputationBook::from_data(&data).expect("the reputation book builds")
}

#[test]
fn thirteen_factions_and_towns_track_reputation() {
    let book = real_book();
    assert_eq!(book.len(), 13);
    assert!(book.get(&id("goodsprings")).is_some());
    assert!(book.get(&id("new_california_republic")).is_some());
}

#[test]
fn goodsprings_tiers_start_at_three_eight_and_fifteen() {
    let book = real_book();
    let model = book
        .get(&id("goodsprings"))
        .expect("Goodsprings is modeled");
    assert_eq!(model.thresholds(), [0, 3, 8, 15]);
    assert_eq!(model.tier(2.0), tier(0));
    assert_eq!(model.tier(3.0), tier(1));
}

/// The seven standings must agree with the wiki's own titles wherever a title is
/// one of the seven names, for every modeled faction.
#[test]
fn the_seven_standings_agree_with_the_titles_of_every_faction() {
    let book = real_book();
    for (faction, model) in book.iter() {
        let expect = |fame: u32, infamy: u32, title: &str, level: ReputationLevel| {
            assert_eq!(
                model.title(tier(fame), tier(infamy)),
                title,
                "{faction}: title at fame {fame}, infamy {infamy}"
            );
            assert_eq!(
                standing(tier(fame), tier(infamy)),
                level,
                "{faction}: {title}"
            );
        };
        expect(0, 0, "Neutral", ReputationLevel::Neutral);
        expect(1, 0, "Accepted", ReputationLevel::Accepted);
        expect(2, 0, "Liked", ReputationLevel::Liked);
        expect(3, 0, "Idolized", ReputationLevel::Idolized);
        expect(0, 1, "Shunned", ReputationLevel::Shunned);
        expect(0, 2, "Hated", ReputationLevel::Shunned);
        expect(0, 3, "Vilified", ReputationLevel::Vilified);
        expect(1, 1, "Mixed", ReputationLevel::Mixed);
    }
}
