//! The character formulas against the real data in `data/engine/`.

use std::fs;
use std::path::Path;

use engine::character::{Character, CharacterRules, Level, SpecialSet};
use engine::shared::{CharacterView, Skill, Special, SpecialLevel};
use serde_json::Value;
use strum::IntoEnumIterator;

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
