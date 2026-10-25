//! What the book refuses to build.

use engine::dialogue::{DialogueBook, DialogueBookError};
use serde_json::json;

use crate::support::{dialogue, topic};

#[test]
fn a_repeated_dialogue_id_is_refused() {
    let one = dialogue("d", &[], vec![topic("a", json!({}))]);
    let again = one.clone();
    assert!(matches!(
        DialogueBook::new([&one, &again]),
        Err(DialogueBookError::DuplicateDialogue { .. })
    ));
}

#[test]
fn a_repeated_topic_key_is_refused() {
    let d = dialogue("d", &[], vec![topic("a", json!({})), topic("a", json!({}))]);
    assert!(matches!(
        DialogueBook::new([&d]),
        Err(DialogueBookError::DuplicateTopic { .. })
    ));
}

#[test]
fn a_start_or_next_link_to_a_missing_topic_is_refused() {
    let bad_start = dialogue("d", &["ghost"], vec![topic("a", json!({}))]);
    let bad_next = dialogue("d", &["a"], vec![topic("a", json!({"next": ["ghost"]}))]);
    for d in [&bad_start, &bad_next] {
        assert!(matches!(
            DialogueBook::new([d]),
            Err(DialogueBookError::UnknownTopic { .. })
        ));
    }
}

#[test]
fn an_ending_topic_with_follow_ups_is_refused() {
    let d = dialogue(
        "d",
        &["a"],
        vec![
            topic("a", json!({"ends": true, "next": ["b"]})),
            topic("b", json!({})),
        ],
    );
    assert!(matches!(
        DialogueBook::new([&d]),
        Err(DialogueBookError::EndingHasNext { .. })
    ));
}

#[test]
fn an_unauthored_dialogue_is_a_valid_book_entry() {
    let d = dialogue("d", &[], vec![topic("a", json!({}))]);
    let book = DialogueBook::new([&d]).expect("no links is fine");
    assert!(book.contains(&d.id));
}
