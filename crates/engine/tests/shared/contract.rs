//! Keeps the Rust vocabulary and `schemas/common.schema.json` in step.
//!
//! The schema is the language-neutral contract (the Python tooling and mod
//! authors rely on it); the Rust types are checked against it in both
//! directions: everything Rust writes must satisfy the schema, and the names and
//! shapes the schema lists must be exactly the ones Rust knows.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use engine::shared::{
    Alignment, Comparison, Condition, Dlc, Effect, Modifier, NpcState, QuestState, ReputationLevel,
    Skill, Special,
};
use jsonschema::Validator;
use proptest::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};
use strum::IntoEnumIterator;

use super::strategies;

static SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/common.schema.json");
    let text = fs::read_to_string(&path).expect("schemas/common.schema.json is readable");
    serde_json::from_str(&text).expect("the schema is valid JSON")
});

/// A validator for one definition under `$defs`.
pub fn validator(def: &str) -> Validator {
    let mut root = SCHEMA.clone();
    let object = root.as_object_mut().expect("the schema is an object");
    // The relative $id would be resolved against a base URL we do not have.
    object.remove("$id");
    object.insert("$ref".into(), json!(format!("#/$defs/{def}")));
    jsonschema::validator_for(&root).expect("the schema compiles")
}

static CONDITION: LazyLock<Validator> = LazyLock::new(|| validator("condition"));
static EFFECT: LazyLock<Validator> = LazyLock::new(|| validator("effect"));
static MODIFIER: LazyLock<Validator> = LazyLock::new(|| validator("modifier"));

/// Follows a `{"$ref": "#/$defs/name"}` one step.
fn resolve(value: &Value) -> &Value {
    match value.get("$ref").and_then(Value::as_str) {
        Some(reference) => &SCHEMA["$defs"][reference.trim_start_matches("#/$defs/")],
        None => value,
    }
}

fn strings(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .expect("expected an array of strings")
        .iter()
        .map(|v| v.as_str().expect("expected a string").to_owned())
        .collect()
}

/// The `type` values the schema lists for a oneOf definition.
fn schema_tags(def: &str) -> BTreeSet<String> {
    SCHEMA["$defs"][def]["oneOf"]
        .as_array()
        .expect("the definition is a oneOf")
        .iter()
        .flat_map(branch_tags)
        .collect()
}

fn branch_tags(branch: &Value) -> BTreeSet<String> {
    let kind = &branch["properties"]["type"];
    match (kind.get("const"), kind.get("enum")) {
        (Some(only), _) => BTreeSet::from([only.as_str().expect("const is a string").to_owned()]),
        (None, Some(many)) => strings(many),
        (None, None) => panic!("a branch without a type: {branch}"),
    }
}

/// The enum values of `property` in the branch of `def` tagged `tag`.
fn branch_enum(def: &str, tag: &str, property: &str) -> BTreeSet<String> {
    let branch = SCHEMA["$defs"][def]["oneOf"]
        .as_array()
        .expect("the definition is a oneOf")
        .iter()
        .find(|branch| branch_tags(branch).contains(tag))
        .unwrap_or_else(|| panic!("no branch tagged {tag}"));
    strings(&resolve(&branch["properties"][property])["enum"])
}

fn rust_names<E: IntoEnumIterator + Serialize>() -> BTreeSet<String> {
    E::iter()
        .map(|value| {
            serde_json::to_value(value)
                .expect("enums serialize")
                .as_str()
                .expect("enums serialize to strings")
                .to_owned()
        })
        .collect()
}

fn as_set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|&n| n.to_owned()).collect()
}

#[test]
fn condition_types_match_the_schema() {
    assert_eq!(schema_tags("condition"), as_set(&Condition::ALL_TAGS));
    assert_eq!(
        Condition::ALL_TAGS.len(),
        as_set(&Condition::ALL_TAGS).len(),
        "duplicate tag"
    );
}

#[test]
fn effect_types_match_the_schema() {
    assert_eq!(schema_tags("effect"), as_set(&Effect::ALL_TAGS));
    assert_eq!(
        Effect::ALL_TAGS.len(),
        as_set(&Effect::ALL_TAGS).len(),
        "duplicate tag"
    );
}

#[test]
fn modifier_types_match_the_schema() {
    assert_eq!(schema_tags("modifier"), as_set(&Modifier::ALL_TAGS));
    assert_eq!(
        Modifier::ALL_TAGS.len(),
        as_set(&Modifier::ALL_TAGS).len(),
        "duplicate tag"
    );
}

#[test]
fn closed_sets_match_the_schema() {
    let defs = &SCHEMA["$defs"];
    assert_eq!(strings(&defs["dlc"]["enum"]), rust_names::<Dlc>());
    assert_eq!(strings(&defs["skill"]["enum"]), rust_names::<Skill>());
    assert_eq!(strings(&defs["special"]["enum"]), rust_names::<Special>());
    assert_eq!(
        branch_enum("condition", "reputation", "level"),
        rust_names::<ReputationLevel>()
    );
    assert_eq!(
        branch_enum("effect", "set_reputation", "level"),
        rust_names::<ReputationLevel>()
    );
    assert_eq!(
        branch_enum("condition", "reputation", "op"),
        rust_names::<Comparison>()
    );
    assert_eq!(
        branch_enum("condition", "karma", "alignment"),
        rust_names::<Alignment>()
    );
    assert_eq!(
        branch_enum("condition", "npc", "state"),
        rust_names::<NpcState>()
    );
    assert_eq!(
        branch_enum("condition", "quest", "state"),
        as_set(&QuestState::ALL_TAGS)
    );
    assert_eq!(
        branch_enum("modifier", "skill", "skill"),
        rust_names::<Skill>()
    );
}

proptest! {
    #[test]
    fn every_condition_satisfies_the_schema_and_round_trips(condition in strategies::condition()) {
        let json = serde_json::to_value(&condition).expect("conditions serialize");
        prop_assert!(CONDITION.is_valid(&json), "the schema rejects {json}");
        prop_assert_eq!(json["type"].as_str(), Some(condition.tag()));
        let back: Condition = serde_json::from_value(json).expect("what we wrote parses");
        prop_assert_eq!(back, condition);
    }

    #[test]
    fn every_modifier_satisfies_the_schema_and_round_trips(modifier in strategies::modifier()) {
        let json = serde_json::to_value(modifier).expect("modifiers serialize");
        prop_assert!(MODIFIER.is_valid(&json), "the schema rejects {json}");
        prop_assert_eq!(json["type"].as_str(), Some(modifier.tag()));
        let back: Modifier = serde_json::from_value(json).expect("what we wrote parses");
        prop_assert_eq!(back, modifier);
    }

    #[test]
    fn every_effect_satisfies_the_schema_and_round_trips(effect in strategies::effect()) {
        let json = serde_json::to_value(&effect).expect("effects serialize");
        prop_assert!(EFFECT.is_valid(&json), "the schema rejects {json}");
        prop_assert_eq!(json["type"].as_str(), Some(effect.tag()));
        let back: Effect = serde_json::from_value(json).expect("what we wrote parses");
        prop_assert_eq!(back, effect);
    }
}

/// Documents that the schema and Rust agree on what is invalid.
#[test]
fn both_sides_reject_the_same_invalid_conditions() {
    let invalid = [
        json!({"type": "quest", "quest": "q", "state": "stage_at_least"}),
        json!({"type": "quest", "quest": "q", "state": "stage_is"}),
        json!({"type": "quest", "quest": "q", "state": "active", "stage": 10}),
        json!({"type": "quest", "quest": "q", "state": "paused"}),
        json!({"type": "quest", "quest": "q", "state": "active", "extra": true}),
        json!({"type": "quest", "quest": "Bad Id", "state": "active"}),
        json!({"type": "skill", "skill": "guns", "min": 101}),
        json!({"type": "special", "stat": "luck", "min": 0}),
        json!({"type": "has_item", "item": "stimpak", "count": 0}),
        json!({"type": "all", "of": []}),
        json!({"type": "not"}),
        json!({"type": "teleport"}),
        json!({"skill": "guns", "min": 5}),
    ];
    for doc in invalid {
        assert!(!CONDITION.is_valid(&doc), "the schema accepts {doc}");
        assert!(
            serde_json::from_value::<Condition>(doc.clone()).is_err(),
            "Rust accepts {doc}"
        );
    }
}

#[test]
fn both_sides_reject_the_same_invalid_effects() {
    let invalid = [
        json!({"type": "set_stage", "quest": "q"}),
        json!({"type": "set_stage", "quest": "q", "stage": -1}),
        json!({"type": "give_item", "item": "stimpak", "count": 0}),
        json!({"type": "karma"}),
        json!({"type": "start_quest", "quest": "q", "extra": 1}),
        json!({"type": "teleport"}),
    ];
    for doc in invalid {
        assert!(!EFFECT.is_valid(&doc), "the schema accepts {doc}");
        assert!(
            serde_json::from_value::<Effect>(doc.clone()).is_err(),
            "Rust accepts {doc}"
        );
    }
}

#[test]
fn both_sides_reject_the_same_invalid_modifiers() {
    let invalid = [
        json!({"type": "skill_points_per_level", "amount": -1}),
        json!({"type": "skill_points_per_level", "amount": 256}),
        json!({"type": "xp_percent", "percent": 128}),
        json!({"type": "xp_percent", "percent": -129}),
        json!({"type": "carry_weight", "pounds": -1}),
        json!({"type": "max_hp", "amount": 65536}),
        json!({"type": "all_skills"}),
        json!({"type": "skill", "skill": "guns"}),
        json!({"type": "skill", "skill": "swimming", "amount": 5}),
        json!({"type": "all_skills", "amount": 5, "extra": true}),
        json!({"type": "teleport"}),
    ];
    for doc in invalid {
        assert!(!MODIFIER.is_valid(&doc), "the schema accepts {doc}");
        assert!(
            serde_json::from_value::<Modifier>(doc.clone()).is_err(),
            "Rust accepts {doc}"
        );
    }
}

/// Documents that defaults mean the same thing on both sides.
#[test]
fn omitted_optional_fields_take_the_documented_defaults() {
    let reputation = json!({"type": "reputation", "faction": "ncr", "level": "liked"});
    assert!(CONDITION.is_valid(&reputation));
    assert!(matches!(
        serde_json::from_value(reputation),
        Ok(Condition::Reputation {
            op: Comparison::AtLeast,
            ..
        })
    ));

    let item = json!({"type": "has_item", "item": "stimpak"});
    assert!(CONDITION.is_valid(&item));
    assert!(matches!(
        serde_json::from_value(item),
        Ok(Condition::HasItem { count, .. }) if count.get() == 1
    ));

    let flag = json!({"type": "flag", "flag": "met_sunny"});
    assert!(CONDITION.is_valid(&flag));
    assert!(matches!(
        serde_json::from_value(flag),
        Ok(Condition::Flag { value: true, .. })
    ));

    let give = json!({"type": "give_item", "item": "stimpak"});
    assert!(EFFECT.is_valid(&give));
    assert!(matches!(
        serde_json::from_value(give),
        Ok(Effect::GiveItem { count, .. }) if count.get() == 1
    ));
}
