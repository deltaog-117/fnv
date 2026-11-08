//! Perks and traits that apply only while the character is hurt or at some hour.

use engine::character::{Character, CharacterRules, PerkBook, Situation, SpecialSet};
use std::path::Path;
use std::sync::LazyLock;

use engine::shared::{
    CharacterView, ConditionalModifiers, Dlc, GameData, HealthPercent, MinuteOfDay, Scope, Skill,
    Special, SpecialLevel, TraitId,
};
use proptest::prelude::*;
use serde_json::json;

static DATA: LazyLock<GameData> = LazyLock::new(|| {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
});

fn data() -> &'static GameData {
    &DATA
}

fn at(hour: u32, minute: u32) -> MinuteOfDay {
    MinuteOfDay::new(hour, minute).expect("a valid time")
}

fn situation(health: u32, hour: u32) -> Situation {
    Situation {
        health: HealthPercent::try_new(health).expect("a valid share"),
        time: at(hour, 0),
    }
}

fn with_trait(data: &GameData, id: &str) -> Character {
    // Hot Blooded and Early Bird belong to Old World Blues, which the real game does
    // not allow yet, so the book here does.
    let book = PerkBook::new(
        data.perks.iter(),
        data.traits.iter(),
        &[Dlc::Base, Dlc::OldWorldBlues],
    );
    // Six fives and a ten make the 40 points a new character has to spend.
    let special = SpecialSet::uniform(SpecialLevel::try_new(5).expect("legal"))
        .with(Special::Strength, SpecialLevel::try_new(10).expect("legal"));
    Character::create(
        special,
        &[Skill::Guns, Skill::Speech, Skill::Sneak],
        &[TraitId::try_new(id).expect("a valid id")],
        CharacterRules::standard(),
        &book,
    )
    .expect("a legal character")
}

#[test]
fn hot_blooded_costs_agility_and_perception_and_adds_damage_below_half_health() {
    let data = data();
    let mut hero = with_trait(data, "hot_blooded");
    let agility = hero.special(Special::Agility).get();
    let perception = hero.special(Special::Perception).get();
    let calm = hero.edge(None).damage_percent;

    hero.set_situation(situation(49, 8));
    assert_eq!(hero.special(Special::Agility).get(), agility - 2);
    assert_eq!(hero.special(Special::Perception).get(), perception - 2);
    assert_eq!(hero.edge(None).damage_percent, calm + 10);

    hero.set_situation(situation(50, 8));
    assert_eq!(
        hero.special(Special::Agility).get(),
        agility,
        "half is not below"
    );
    assert_eq!(hero.edge(None).damage_percent, calm);
}

#[test]
fn early_bird_follows_the_hour_and_never_leaves_the_scale() {
    let data = data();
    let mut hero = with_trait(data, "early_bird");
    // A new game starts at 08:00, inside the morning, so settle at a neutral hour
    // before taking the plain values.
    hero.set_situation(situation(100, 14));
    let strength = hero.special(Special::Strength).get();
    assert_eq!(strength, 10, "the fixture starts at the top of the scale");
    let luck = hero.special(Special::Luck).get();

    hero.set_situation(situation(100, 9));
    assert_eq!(hero.special(Special::Strength).get(), 10, "capped at 10");
    assert_eq!(hero.special(Special::Luck).get(), luck + 2);

    hero.set_situation(situation(100, 14));
    assert_eq!(
        hero.special(Special::Luck).get(),
        luck,
        "noon to six is neutral"
    );

    hero.set_situation(situation(100, 23));
    assert_eq!(hero.special(Special::Luck).get(), luck - 1);
    hero.set_situation(situation(100, 5));
    assert_eq!(
        hero.special(Special::Luck).get(),
        luck - 1,
        "the night crosses midnight"
    );
    hero.set_situation(situation(100, 6));
    assert_eq!(
        hero.special(Special::Luck).get(),
        luck + 2,
        "six is the start of morning"
    );
}

#[test]
fn the_wiki_texts_are_what_the_data_says() {
    let data = data();
    let text = |id: &str| {
        data.traits
            .iter()
            .find(|t| t.id.as_str() == id)
            .and_then(|t| t.description.clone())
            .unwrap_or_default()
    };
    assert!(text("hot_blooded").contains("below 50%") && text("hot_blooded").contains("+10%"));
    assert!(text("early_bird").contains("+2") && text("early_bird").contains("-1"));
    let perk = |id: &str| {
        data.perks
            .iter()
            .find(|p| p.id.as_str() == id)
            .and_then(|p| p.description.clone())
            .unwrap_or_default()
    };
    assert!(perk("nerd_rage").contains("Strength is raised to 10"));
    assert!(perk("nerd_rage").contains("15 points to Damage Threshold"));
    assert!(perk("nerd_rage").contains("20% or below"));
    assert!(perk("night_person").contains("sun is down"));
}

#[test]
fn nerd_rage_and_night_person_are_modeled_from_the_data() {
    let data = data();
    let nerd_rage = data
        .perks
        .iter()
        .find(|p| p.id.as_str() == "nerd_rage")
        .expect("the perk exists");
    assert_eq!(nerd_rage.conditional.len(), 1);
    let night_person = data
        .perks
        .iter()
        .find(|p| p.id.as_str() == "night_person")
        .expect("the perk exists");
    assert_eq!(night_person.conditional.len(), 1);
}

fn gate(
    when: serde_json::Value,
    modifiers: serde_json::Value,
) -> Result<ConditionalModifiers, String> {
    serde_json::from_value(json!({"when": when, "modifiers": modifiers}))
        .map_err(|error| error.to_string())
}

#[test]
fn a_gate_may_read_only_health_and_the_time() {
    let health = json!({"type": "health", "percent": 50, "op": "at_most"});
    let night = json!({"type": "time_of_day", "from": "18:00", "to": "06:00"});
    let bonus = json!([{"type": "damage_percent", "percent": 5}]);
    assert!(gate(json!([health]), bonus.clone()).is_ok());
    assert!(gate(json!([health, night]), bonus.clone()).is_ok());
    assert!(
        gate(
            json!([{"type": "any", "of": [night, {"type": "not", "of": health}]}]),
            bonus.clone()
        )
        .is_ok()
    );
    for other in [
        json!({"type": "skill", "skill": "guns", "min": 50}),
        json!({"type": "special", "stat": "luck", "min": 5}),
        json!({"type": "flag", "flag": "met_sunny"}),
        json!({"type": "all", "of": [health, {"type": "perk", "perk": "toughness"}]}),
    ] {
        let refused = gate(json!([other]), bonus.clone());
        assert!(
            refused.is_err_and(|e| e.contains("health and the time of day")),
            "{other}"
        );
    }
    assert!(
        gate(json!([]), bonus).is_err(),
        "a gate with no conditions is refused"
    );
    assert!(
        gate(json!([health]), json!([])).is_err(),
        "so is one that changes nothing"
    );
}

#[test]
fn a_gate_may_not_move_the_hit_points_its_own_condition_reads() {
    let health = json!({"type": "health", "percent": 50, "op": "at_most"});
    let night = json!({"type": "time_of_day", "from": "18:00", "to": "06:00"});
    let refused = |when: &serde_json::Value, modifier: serde_json::Value| {
        gate(json!([when]), json!([modifier])).is_err()
    };
    assert!(refused(&night, json!({"type": "max_hp", "amount": 10})));
    assert!(refused(&health, json!({"type": "max_hp", "amount": 10})));
    assert!(refused(
        &health,
        json!({"type": "all_special", "amount": 1})
    ));
    assert!(refused(
        &health,
        json!({"type": "special", "stat": "endurance", "amount": 1})
    ));
    assert!(!refused(
        &health,
        json!({"type": "special", "stat": "strength", "amount": 1})
    ));
    assert!(!refused(
        &night,
        json!({"type": "all_special", "amount": 1})
    ));
    assert!(!refused(
        &night,
        json!({"type": "special", "stat": "endurance", "amount": 1})
    ));
}

proptest! {
    /// Where the character stands depends only on the situation it is in now, not on
    /// the situations it passed through to get there, and no attribute leaves 1 to
    /// 10.
    #[test]
    fn the_situation_decides_and_not_the_path(
        path in proptest::collection::vec((0u32..=100, 0u32..24), 0..8),
        last in (0u32..=100, 0u32..24),
    ) {
        let data = data();
        let mut wandered = with_trait(data, "early_bird");
        for (health, hour) in path {
            wandered.set_situation(situation(health, hour));
        }
        wandered.set_situation(situation(last.0, last.1));
        let mut direct = with_trait(data, "early_bird");
        direct.set_situation(situation(last.0, last.1));
        prop_assert_eq!(&wandered, &direct);
        for stat in [Special::Strength, Special::Perception, Special::Endurance,
                     Special::Charisma, Special::Intelligence, Special::Agility, Special::Luck] {
            let value = wandered.special(stat).get();
            prop_assert!((1..=10).contains(&value));
        }
    }
}
