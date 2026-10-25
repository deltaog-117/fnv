//! The runner's laws, checked over random dialogue graphs and random walks.

use engine::dialogue::DialogueError;
use proptest::prelude::*;
use serde_json::{Value, json};

use crate::support::{dialogue, game, has_flag, key, talk, topic};

const TOPICS: usize = 6;

/// One random topic: links, optional ending, and the flag it needs, checks and sets.
#[derive(Debug, Clone)]
struct Spec {
    next: Vec<usize>,
    ends: bool,
    needs: Option<usize>,
    gate: Option<usize>,
    sets: Option<usize>,
}

fn spec() -> impl Strategy<Value = Spec> {
    (
        prop::collection::vec(0..TOPICS, 0..3),
        any::<bool>(),
        prop::option::of(0..TOPICS),
        prop::option::of(0..TOPICS),
        prop::option::of(0..TOPICS),
    )
        .prop_map(|(next, ends, needs, gate, sets)| Spec {
            // An ending topic has no follow-ups, as the book requires.
            ends: ends && next.is_empty(),
            next,
            needs,
            gate,
            sets,
        })
}

fn flag_condition(index: usize) -> Value {
    json!({"type": "flag", "flag": format!("f{index}")})
}

fn build(specs: &[Spec], start: &[usize]) -> engine::shared::Dialogue {
    let names: Vec<String> = (0..specs.len()).map(|i| format!("t{i}")).collect();
    let topics = specs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut extra = json!({
                "next": s.next.iter().map(|n| &names[*n]).collect::<Vec<_>>(),
                "ends": s.ends,
            });
            if let Some(n) = s.needs {
                extra["conditions"] = json!([flag_condition(n)]);
            }
            if let Some(g) = s.gate {
                extra["check"] = flag_condition(g);
            }
            if let Some(f) = s.sets {
                extra["effects"] = json!([{"type": "set_flag", "flag": format!("f{f}")}]);
            }
            topic(&names[i], extra)
        })
        .collect();
    let start: Vec<&str> = start.iter().map(|i| names[*i].as_str()).collect();
    dialogue("d", &start, topics)
}

/// Walks the dialogue by `picks`, checking the laws at every step, and returns the
/// trace so two runs can be compared.
fn walk(specs: &[Spec], start: &[usize], picks: &[u8]) -> Vec<String> {
    let d = build(specs, start);
    let (mut talk, mut game) = (talk(&d), game());
    let flags = |game: &engine::game::Game| -> Vec<bool> {
        (0..TOPICS)
            .map(|i| has_flag(game, &format!("f{i}")))
            .collect()
    };
    let mut trace = Vec::new();
    for pick in picks {
        let choices = talk.choices(&game);
        if talk.is_over() {
            assert!(choices.is_empty(), "nothing is offered after the end");
            assert_eq!(talk.choose(&mut game, &key("t0")), Err(DialogueError::Over));
            break;
        }
        for choice in &choices {
            let spec = &specs[choice.key.as_str()[1..].parse::<usize>().expect("tN key")];
            assert!(spec.needs.is_none_or(|n| has_flag(&game, &format!("f{n}"))));
            assert_eq!(
                choice.unlocked,
                spec.gate.is_none_or(|g| has_flag(&game, &format!("f{g}")))
            );
        }
        let (before, menu_before) = (flags(&game), choices.clone());
        let index = usize::from(*pick) % (choices.len() + 1);
        if index == choices.len() {
            // Not in the menu: always refused, never changes anything.
            let result = talk.choose(&mut game, &key("zzz"));
            assert!(matches!(result, Err(DialogueError::NotOffered { .. })));
            assert_eq!(flags(&game), before);
            assert_eq!(talk.choices(&game), menu_before);
            trace.push("refused".to_owned());
            continue;
        }
        let chosen = &choices[index];
        let result = talk.choose(&mut game, &chosen.key);
        if chosen.unlocked {
            let reply = result.expect("an offered, unlocked topic with plain effects succeeds");
            assert_eq!(reply.topic, chosen.key);
            assert!(!reply.lines.is_empty());
            assert_eq!(reply.over, talk.is_over());
            trace.push(format!("{} over={}", reply.topic, reply.over));
        } else {
            assert!(matches!(result, Err(DialogueError::Locked { .. })));
            assert_eq!(flags(&game), before);
            assert_eq!(talk.choices(&game), menu_before);
            trace.push(format!("locked {}", chosen.key));
        }
    }
    trace
}

proptest! {
    #[test]
    fn the_laws_hold_on_any_graph(
        specs in prop::collection::vec(spec(), TOPICS..=TOPICS),
        start in prop::collection::vec(0..TOPICS, 1..4),
        picks in prop::collection::vec(any::<u8>(), 0..40),
    ) {
        // The same walk twice gives the same trace: the runner has no hidden state.
        prop_assert_eq!(walk(&specs, &start, &picks), walk(&specs, &start, &picks));
    }
}
