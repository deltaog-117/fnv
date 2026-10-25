//! The runner against the real dialogue data in `data/engine/`.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use engine::dialogue::{Conversation, DialogueBook};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    Effect, GameData, QuestId, QuestStage, QuestStatus, QuestView, Scope, apply_all,
};

use crate::support::{dialogue_id, key, keys};

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

fn quest(text: &str) -> QuestId {
    QuestId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn game(data: &GameData) -> Game {
    Game::new(Arc::new(
        QuestBook::new(data.quests.iter()).expect("book builds"),
    ))
}

#[test]
fn every_dialogue_file_builds_into_the_book() {
    let data = data();
    let book = DialogueBook::new(data.dialogue.iter()).expect("all 517 files are well formed");
    assert!(book.contains(&dialogue_id("docmitchell")));
}

#[test]
fn every_authored_topic_can_be_reached_from_a_start_topic() {
    let data = data();
    for d in data.dialogue.iter().filter(|d| !d.start.is_empty()) {
        let mut reached: BTreeSet<_> = d.start.iter().collect();
        let mut frontier: Vec<_> = d.start.iter().collect();
        while let Some(from) = frontier.pop() {
            let topic = d
                .topics
                .iter()
                .find(|t| &t.key == from)
                .expect("links resolve");
            for next in &topic.next {
                if reached.insert(next) {
                    frontier.push(next);
                }
            }
        }
        for topic in &d.topics {
            let authored = !topic.next.is_empty()
                || !topic.conditions.is_empty()
                || !topic.effects.is_empty()
                || topic.check.is_some()
                || topic.ends;
            assert!(
                !authored || reached.contains(&topic.key),
                "{}: authored topic {} cannot be reached",
                d.id,
                topic.key
            );
        }
    }
}

#[test]
fn doc_mitchell_answers_questions_and_says_goodbye() {
    let data = data();
    let book = Arc::new(DialogueBook::new(data.dialogue.iter()).expect("book builds"));
    let mut game = game(&data);
    let mut talk = Conversation::open(book, &dialogue_id("docmitchell")).expect("authored");

    let menu = talk.choices(&game);
    assert!(menu.iter().any(|c| c.text == "What town is this?"));
    // The gunfight help topic needs the quest to be active.
    assert!(!keys(&menu).contains(&"vfreeformgoodspringsdocmitchellshootouthelp"));

    let about = key("vfreeformgoodspringsdocmitchelltellmeaboutyou");
    talk.choose(&mut game, &about).expect("on offer");
    assert_eq!(talk.choices(&game).len(), 2, "two follow-ups");

    let bye = key("goodbye");
    talk.choose(&mut game, &key("vfreeformgoodspringsdocmitchelltopic006"))
        .expect("back to the start menu");
    talk.choose(&mut game, &bye).expect("on offer");
    assert!(talk.is_over());
}

#[test]
fn sunny_starts_the_saddle_quest_once() {
    let data = data();
    let book = Arc::new(DialogueBook::new(data.dialogue.iter()).expect("book builds"));
    let mut game = game(&data);
    let mut talk = Conversation::open(book, &dialogue_id("gssunnysmiles")).expect("authored");

    let teach = key("vfreeformgoodspringsgssunnysmilestopic019");
    assert!(keys(&talk.choices(&game)).contains(&teach.as_str()));
    talk.choose(&mut game, &teach).expect("on offer");

    let saddle = quest("back_in_the_saddle");
    assert_eq!(game.quest_status(&saddle), QuestStatus::Active);
    assert_eq!(game.quest_stage(&saddle), Some(QuestStage::new(4)));
    assert!(!keys(&talk.choices(&game)).contains(&teach.as_str()));
}

#[test]
fn sunny_moves_the_gunfight_forward_when_asked_about_it() {
    let data = data();
    let book = Arc::new(DialogueBook::new(data.dialogue.iter()).expect("book builds"));
    let mut game = game(&data);
    let mut talk = Conversation::open(book, &dialogue_id("gssunnysmiles")).expect("authored");
    let help = key("vfreeformgoodspringsgoodspringssunnysmilestopic001");
    assert!(!keys(&talk.choices(&game)).contains(&help.as_str()));

    let gunfight = quest("ghost_town_gunfight");
    apply_all(
        &[Effect::SetStage {
            quest: gunfight.clone(),
            stage: QuestStage::new(5),
        }],
        &mut game,
    )
    .expect("stage 5 is reached");
    talk.choose(&mut game, &help).expect("on offer at stage 5");
    assert_eq!(game.quest_stage(&gunfight), Some(QuestStage::new(10)));
    assert_eq!(talk.choices(&game).len(), 2, "Trudy and Easy Pete follow");
    assert!(!keys(&talk.choices(&game)).contains(&help.as_str()));
}
