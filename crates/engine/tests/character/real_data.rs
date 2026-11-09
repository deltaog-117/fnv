//! The character formulas against the real data in `data/engine/`.

use std::fs;
use std::path::Path;

use engine::character::{
    BASE_SKILL_HALVES, Character, CharacterRules, Level, PerkBook, SpecialSet, TAG_COUNT,
};
use engine::shared::{CharacterView, GameData, Modifier, Scope, Skill, Special, SpecialLevel};
use serde_json::Value;
use strum::IntoEnumIterator;

use super::progression::{perk_id, trait_id};

fn read(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/engine")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).expect("file exists")).expect("file parses")
}

fn formula(id: &str) -> Value {
    read("formulas.json")["formulas"]
        .as_array()
        .expect("formulas is a list")
        .iter()
        .find(|f| f["id"] == id)
        .unwrap_or_else(|| panic!("formula {id} exists"))
        .clone()
}

#[test]
fn the_standard_rules_match_the_curated_constants() {
    let constants = read("formulas.json")["constants"].clone();
    let number =
        |key: &str| u32::try_from(constants[key].as_u64().expect("a number")).expect("fits");
    let from_file = CharacterRules::new(
        number("xp_base"),
        number("xp_step"),
        number("level_cap_base"),
        number("ap_base_cap"),
    )
    .expect("the file's constants are valid");
    assert_eq!(CharacterRules::standard(), from_file);
}

#[test]
fn the_perk_rate_is_the_one_in_the_file() {
    let constants = read("formulas.json")["constants"].clone();
    let rate = u32::try_from(constants["perk_rate"].as_u64().expect("a number")).expect("fits");
    assert_eq!(CharacterRules::standard().perk_rate(), rate);
    let from_file = CharacterRules::standard()
        .with_perk_rate(rate)
        .expect("a legal rate");
    assert_eq!(CharacterRules::standard(), from_file);
    assert!(CharacterRules::standard().with_perk_rate(0).is_err());
}

#[test]
fn the_skill_points_per_level_are_the_formula_in_the_file() {
    assert_eq!(
        formula("skill_points_per_level")["expression"],
        "10 + 0.5*intelligence"
    );
    // The same, in half points: 20 + intelligence.
    assert_eq!(BASE_SKILL_HALVES, 20);
    assert_eq!(TAG_COUNT, 3);
}

#[test]
fn the_written_formulas_are_the_ones_in_the_file() {
    // The expressions are implemented by hand in `Character`; if the file changes
    // one, this fails so the code gets reviewed too.
    for (id, expression) in [
        ("max_hp", "100 + 20*endurance + 5*(level - 1)"),
        ("action_points", "min(65 + 3*agility, ap_base_cap)"),
        ("carry_weight", "150 + 10*strength"),
        ("xp_to_next_level", "xp_base + xp_step*(level - 1)"),
    ] {
        assert_eq!(formula(id)["expression"], expression, "{id}");
    }
}

#[test]
fn every_skill_starts_from_the_attribute_the_file_names() {
    let low = SpecialSet::uniform(SpecialLevel::try_new(1).expect("legal"));
    let high = SpecialLevel::try_new(10).expect("legal");
    for skill in Skill::iter() {
        let name = serde_json::to_value(skill).expect("serializes");
        let name = name.as_str().expect("a name");
        let entry = formula(&format!("skill_initial_{name}"));
        let stat: Special = serde_json::from_value(entry["vars"][0].clone())
            .unwrap_or_else(|_| panic!("{name}: first var is an attribute"));
        let var = entry["vars"][0].as_str().expect("a name");
        assert_eq!(
            entry["expression"],
            format!("2 + 2*{var} + ceil(luck/2)"),
            "{name}"
        );
        let raised = Character::new(low.with(stat, high), CharacterRules::standard());
        // 2 + 2 * 10 + ceil(1 / 2)
        assert_eq!(
            raised.skill(skill).get(),
            23,
            "{name} should follow {stat:?}"
        );
        for other in Special::iter().filter(|other| *other != stat && *other != Special::Luck) {
            let raised = Character::new(low.with(other, high), CharacterRules::standard());
            // 2 + 2 * 1 + ceil(1 / 2)
            assert_eq!(
                raised.skill(skill).get(),
                5,
                "{name} should ignore {other:?}"
            );
        }
    }
}

#[test]
fn the_xp_totals_match_the_level_table() {
    let rules = CharacterRules::standard();
    let levels = read("levels.json");
    for row in levels.as_array().expect("a list") {
        let level =
            Level::try_new(u32::try_from(row["level"].as_u64().expect("a number")).expect("fits"))
                .expect("a legal level");
        let total = u32::try_from(row["xp_total"].as_u64().expect("a number")).expect("fits");
        assert_eq!(rules.xp_for_level(level), total, "level {}", level.get());
    }
}

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

/// What the wiki's own text must still say for each modeled perk and trait: the
/// modifiers are authored by hand from it, so a changed text means a changed
/// import and the modifiers get reviewed.
#[test]
fn the_wiki_text_still_says_what_each_modeled_perk_and_trait_does() {
    let data = data();
    for (id, said) in [
        ("educated", "+2 skill points at each level up"),
        ("swift_learner", "10% bonus to Experience Points earned"),
        ("strong_back", "additional 50 lbs"),
        ("action_boy_action_girl", "+15/30 Action Points"),
        ("life_giver", "+30 HP"),
        ("toughness", "Permanent +3 to DT"),
        ("finesse", "equivalent to +5 Luck"),
        ("better_criticals", "+50% Damage with critical hits"),
        ("math_wrath", "Reduce all AP costs by 10 percent."),
        ("rapid_reload", "25% faster reload"),
        ("piercing_strike", "Ignore 15 points of a target's DT"),
        ("fast_metabolism", "20% Health bonus when using stimpaks"),
        ("demolition_expert", "+20%/40%/60% Damage with explosives"),
        (
            "entomologist",
            "+50% Damage when attacking a mutated insect.",
        ),
        (
            "hunter",
            "75% more critical damage against animals and mutated animals.",
        ),
    ] {
        let perk = data
            .perks
            .get(&perk_id(id))
            .unwrap_or_else(|| panic!("perk {id} exists"));
        let text = perk.effects_text.as_deref().unwrap_or_default();
        assert!(text.contains(said), "{id}: {text:?}");
        assert!(!perk.modifiers.is_empty(), "{id} is modeled");
    }
    for (id, benefit, penalty) in [
        (
            "good_natured",
            "+5 to Barter, Medicine, Repair, Science, and Speech",
            "-5 to Energy Weapons, Explosives, Guns, Melee Weapons, and Unarmed",
        ),
        (
            "skilled",
            "+5 to all skills",
            "-10% to Experience Points gained",
        ),
        (
            "built_to_destroy",
            "+3% Critical Chance",
            "+15% weapon wear",
        ),
        (
            "kamikaze",
            "+10 Action Points",
            "-2 Damage Threshold, -15% Damage Resistance",
        ),
        (
            "heavy_handed",
            "+20% damage (Melee and Unarmed only)",
            "-60% Critical damage. (Melee and Unarmed only)",
        ),
        (
            "fast_shot",
            "AP costs are decreased by 20% for all firearms with Energy Weapons and Guns",
            "Weapon Spread increases by 20%",
        ),
        (
            "trigger_discipline",
            "+20% accuracy with Guns and Energy Weapons",
            "+20% AP required per shot",
        ),
    ] {
        let record = data
            .traits
            .get(&trait_id(id))
            .unwrap_or_else(|| panic!("trait {id} exists"));
        assert!(
            record
                .benefits
                .as_deref()
                .unwrap_or_default()
                .contains(benefit),
            "{id}: {:?}",
            record.benefits
        );
        assert!(
            record
                .penalties
                .as_deref()
                .unwrap_or_default()
                .contains(penalty),
            "{id}: {:?}",
            record.penalties
        );
        assert!(!record.modifiers.is_empty(), "{id} is modeled");
    }
}

#[test]
fn only_the_modeled_perks_and_traits_carry_modifiers() {
    let data = data();
    let perks: Vec<&str> = data
        .perks
        .iter()
        .filter(|perk| !perk.modifiers.is_empty())
        .map(|perk| perk.id.as_str())
        .collect();
    assert_eq!(
        perks,
        [
            "action_boy_action_girl",
            "better_criticals",
            // Arcade Gannon grants it while he is in the party.
            "better_healing",
            "demolition_expert",
            "educated",
            "entomologist",
            "fast_metabolism",
            "finesse",
            "hunter",
            "life_giver",
            "math_wrath",
            "piercing_strike",
            "rapid_reload",
            "strong_back",
            "swift_learner",
            "toughness"
        ]
    );
    let traits: Vec<&str> = data
        .traits
        .iter()
        .filter(|record| !record.modifiers.is_empty())
        .map(|record| record.id.as_str())
        .collect();
    assert_eq!(
        traits,
        [
            "built_to_destroy",
            "fast_shot",
            "good_natured",
            "heavy_handed",
            "kamikaze",
            "skilled",
            "trigger_discipline"
        ]
    );
}

#[test]
fn good_natured_moves_ten_skills_by_five_and_leaves_three_alone() {
    let data = data();
    let record = data
        .traits
        .get(&trait_id("good_natured"))
        .expect("the trait exists");
    let moved: Vec<(Skill, i8)> = record
        .modifiers
        .iter()
        .map(|modifier| match modifier {
            Modifier::Skill { skill, amount } => (*skill, *amount),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(moved.len(), 10);
    assert!(moved.iter().all(|(_, amount)| amount.abs() == 5));
    for untouched in [Skill::Lockpick, Skill::Sneak, Skill::Survival] {
        assert!(moved.iter().all(|(skill, _)| *skill != untouched));
    }
}

#[test]
fn the_base_game_book_holds_no_add_on_content() {
    let data = data();
    let book = PerkBook::base_game(&data);
    assert!(book.perks().count() > 100);
    assert!(
        book.perks()
            .all(|perk| perk.dlc == engine::shared::Dlc::Base)
    );
    assert!(
        book.traits()
            .all(|record| record.dlc == engine::shared::Dlc::Base)
    );
    assert!(book.character_trait(&trait_id("skilled")).is_none());
    assert!(book.character_trait(&trait_id("good_natured")).is_some());
}

#[test]
fn a_real_character_made_with_a_real_trait_levels_up_and_takes_a_real_perk() {
    let data = data();
    let book = PerkBook::base_game(&data);
    let special = SpecialSet::default().with(
        Special::Intelligence,
        SpecialLevel::try_new(10).expect("legal"),
    );
    let mut character = Character::create(
        special,
        &[Skill::Guns, Skill::Speech, Skill::Sneak],
        &[trait_id("good_natured")],
        CharacterRules::standard(),
        &book,
    )
    .expect("a legal character");
    character.grant_xp(CharacterRules::standard().xp_for_level(Level::try_new(4).expect("legal")));
    let open: Vec<_> = book.open_to(&character);
    assert!(open.iter().any(|perk| perk.id.as_str() == "educated"));
    character
        .choose_perk(&book, &perk_id("educated"))
        .expect("level 4, intelligence 10");
    assert!(character.has_perk(&perk_id("educated")));
}
