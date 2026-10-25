//! The rules of a conversation, pinned with small hand-built dialogues.

use engine::dialogue::{Conversation, DialogueError};
use serde_json::json;

use crate::support::{book, dialogue, dialogue_id, game, has_flag, key, keys, talk, topic};

fn set(flag: &str) -> serde_json::Value {
    json!({"type": "set_flag", "flag": flag})
}

#[test]
fn a_conversation_opens_at_the_start_topics_in_order() {
    let d = dialogue(
        "d",
        &["b", "a"],
        vec![
            topic("a", json!({})),
            topic("b", json!({})),
            topic("c", json!({})),
        ],
    );
    let talk = talk(&d);
    let game = game();
    assert_eq!(keys(&talk.choices(&game)), ["b", "a"]);
}

#[test]
fn a_choice_shows_the_prompt_then_the_label_then_the_key() {
    let d = dialogue(
        "d",
        &["p", "l", "k"],
        vec![
            topic("p", json!({"prompt": "prompt", "label": "label"})),
            serde_json::from_value(json!({
                "key": "l", "section": "topics", "label": "label",
                "responses": [{"text": "x"}],
            }))
            .unwrap(),
            serde_json::from_value(json!({
                "key": "k", "section": "topics", "responses": [{"text": "x"}],
            }))
            .unwrap(),
        ],
    );
    let texts: Vec<String> = talk(&d)
        .choices(&game())
        .into_iter()
        .map(|choice| choice.text)
        .collect();
    assert_eq!(texts, ["prompt", "label", "k"]);
}

#[test]
fn a_topic_without_follow_ups_returns_to_the_start() {
    let d = dialogue(
        "d",
        &["a", "b"],
        vec![topic("a", json!({})), topic("b", json!({}))],
    );
    let (mut talk, mut game) = (talk(&d), game());
    let reply = talk.choose(&mut game, &key("a")).expect("a is on offer");
    assert_eq!(reply.lines[0].text, "answer a");
    assert!(!reply.over);
    assert_eq!(keys(&talk.choices(&game)), ["a", "b"]);
}

#[test]
fn a_topic_with_follow_ups_offers_only_those() {
    let d = dialogue(
        "d",
        &["a", "b"],
        vec![
            topic("a", json!({"next": ["c"]})),
            topic("b", json!({})),
            topic("c", json!({})),
        ],
    );
    let (mut talk, mut game) = (talk(&d), game());
    talk.choose(&mut game, &key("a")).expect("a is on offer");
    assert_eq!(keys(&talk.choices(&game)), ["c"]);
    assert!(matches!(
        talk.choose(&mut game, &key("b")),
        Err(DialogueError::NotOffered { .. })
    ));
}

#[test]
fn follow_ups_that_are_all_unavailable_fall_back_to_the_start() {
    let hidden = json!({"conditions": [{"type": "flag", "flag": "never"}]});
    let d = dialogue(
        "d",
        &["a"],
        vec![topic("a", json!({"next": ["c"]})), topic("c", hidden)],
    );
    let (mut talk, mut game) = (talk(&d), game());
    talk.choose(&mut game, &key("a")).expect("a is on offer");
    assert_eq!(keys(&talk.choices(&game)), ["a"]);
}

#[test]
fn conditions_hide_a_topic_until_they_hold() {
    let d = dialogue(
        "d",
        &["unlock", "secret"],
        vec![
            topic("unlock", json!({"effects": [set("known")]})),
            topic(
                "secret",
                json!({"conditions": [{"type": "flag", "flag": "known"}]}),
            ),
        ],
    );
    let (mut talk, mut game) = (talk(&d), game());
    assert_eq!(keys(&talk.choices(&game)), ["unlock"]);
    assert!(matches!(
        talk.choose(&mut game, &key("secret")),
        Err(DialogueError::NotOffered { .. })
    ));
    talk.choose(&mut game, &key("unlock")).expect("on offer");
    assert!(has_flag(&game, "known"));
    assert_eq!(keys(&talk.choices(&game)), ["unlock", "secret"]);
}

#[test]
fn a_failing_check_lists_the_option_but_locks_it() {
    let gate = json!({"check": {"type": "flag", "flag": "ready"}});
    let d = dialogue(
        "d",
        &["ready", "gated"],
        vec![
            topic("ready", json!({"effects": [set("ready")]})),
            topic("gated", gate),
        ],
    );
    let (mut talk, mut game) = (talk(&d), game());
    let choices = talk.choices(&game);
    assert_eq!(keys(&choices), ["ready", "gated"]);
    assert!(choices[0].unlocked && choices[0].requires.is_none());
    assert!(!choices[1].unlocked && choices[1].requires.is_some());
    assert!(matches!(
        talk.choose(&mut game, &key("gated")),
        Err(DialogueError::Locked { .. })
    ));

    talk.choose(&mut game, &key("ready")).expect("on offer");
    assert!(talk.choices(&game)[1].unlocked);
    talk.choose(&mut game, &key("gated")).expect("now unlocked");
}

#[test]
fn an_ending_topic_closes_the_conversation() {
    let d = dialogue("d", &["bye"], vec![topic("bye", json!({"ends": true}))]);
    let (mut talk, mut game) = (talk(&d), game());
    let reply = talk.choose(&mut game, &key("bye")).expect("on offer");
    assert!(reply.over && talk.is_over());
    assert!(talk.choices(&game).is_empty());
    assert_eq!(
        talk.choose(&mut game, &key("bye")),
        Err(DialogueError::Over)
    );
}

#[test]
fn quest_effects_run_through_the_game() {
    let d = dialogue(
        "d",
        &["go"],
        vec![topic(
            "go",
            json!({"effects": [
                {"type": "start_quest", "quest": "q"},
                {"type": "set_stage", "quest": "q", "stage": 5},
            ]}),
        )],
    );
    let (mut talk, mut game) = (talk(&d), game());
    talk.choose(&mut game, &key("go")).expect("on offer");
    let q = engine::shared::QuestId::try_new("q".to_owned()).unwrap();
    assert_eq!(
        engine::shared::QuestView::quest_stage(&game, &q).map(|s| s.get()),
        Some(5)
    );
}

#[test]
fn a_failing_effect_leaves_the_state_and_the_conversation_untouched() {
    let d = dialogue(
        "d",
        &["bad", "other"],
        vec![
            topic(
                "bad",
                json!({"effects": [set("half"), {"type": "set_stage", "quest": "missing", "stage": 5}],
                       "next": ["other"]}),
            ),
            topic("other", json!({})),
        ],
    );
    let (mut talk, mut game) = (talk(&d), game());
    let before = talk.choices(&game);
    let result = talk.choose(&mut game, &key("bad"));
    assert!(
        matches!(result, Err(DialogueError::Effects { .. })),
        "{result:?}"
    );
    assert!(
        !has_flag(&game, "half"),
        "the first effect must be rolled back"
    );
    assert_eq!(talk.choices(&game), before);
}

#[test]
fn a_topic_that_does_not_exist_is_not_offered() {
    let d = dialogue("d", &["a"], vec![topic("a", json!({}))]);
    let (mut talk, mut game) = (talk(&d), game());
    assert!(matches!(
        talk.choose(&mut game, &key("nope")),
        Err(DialogueError::NotOffered { .. })
    ));
}

#[test]
fn opening_needs_a_known_dialogue_with_start_topics() {
    let authored = dialogue("a", &["t"], vec![topic("t", json!({}))]);
    let bare = dialogue("b", &[], vec![topic("t", json!({}))]);
    let both = book(&[authored, bare]);
    assert!(matches!(
        Conversation::open(both.clone(), &dialogue_id("zzz")),
        Err(DialogueError::UnknownDialogue { .. })
    ));
    assert!(matches!(
        Conversation::open(both, &dialogue_id("b")),
        Err(DialogueError::NotAuthored { .. })
    ));
}
