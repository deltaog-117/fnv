//! Property tests for ids and bounded numbers.

use engine::shared::{QuestId, SkillLevel, SpecialLevel};
use proptest::prelude::*;
use serde_json::Value;

use super::contract::validator;

/// An independent statement of the id rule, kept different from the
/// implementation on purpose so the test is not a copy of the code under test.
fn reference_is_valid(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('_')
        && text
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

/// Mostly-plausible text, so both accepted and rejected cases are common.
fn id_like_text() -> impl Strategy<Value = String> {
    prop_oneof!["[a-zA-Z0-9_ -]{0,10}", any::<String>()]
}

proptest! {
    #[test]
    fn id_acceptance_matches_the_reference_rule(text in id_like_text()) {
        prop_assert_eq!(QuestId::try_new(text.clone()).is_ok(), reference_is_valid(&text));
    }

    #[test]
    fn id_acceptance_matches_the_schema(text in id_like_text()) {
        let schema_says = validator("id").is_valid(&Value::String(text.clone()));
        prop_assert_eq!(QuestId::try_new(text).is_ok(), schema_says);
    }

    #[test]
    fn valid_ids_survive_a_json_round_trip(text in "[a-z0-9][a-z0-9_]{0,30}") {
        let id = QuestId::try_new(text.clone()).expect("generated text is a valid id");
        let json = serde_json::to_string(&id).expect("ids serialize");
        let back: QuestId = serde_json::from_str(&json).expect("what we wrote parses");
        prop_assert_eq!(back.as_str(), text);
    }

    #[test]
    fn invalid_id_text_is_rejected_when_parsing_json(text in "[A-Z _-][a-zA-Z0-9_ -]{0,10}") {
        let json = serde_json::to_string(&text).expect("text serializes");
        prop_assert!(serde_json::from_str::<QuestId>(&json).is_err());
    }

    #[test]
    fn skill_levels_accept_exactly_zero_to_one_hundred(n in any::<u32>()) {
        prop_assert_eq!(SkillLevel::try_new(n).is_ok(), n <= 100);
    }

    #[test]
    fn special_levels_accept_exactly_one_to_ten(n in any::<u32>()) {
        prop_assert_eq!(SpecialLevel::try_new(n).is_ok(), (1..=10).contains(&n));
    }
}
