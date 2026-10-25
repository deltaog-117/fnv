//! The NPC laws, checked over random sequences of effects.

use engine::shared::{NpcState, WorldSink, WorldView};
use proptest::prelude::*;

use crate::support::{npc, world};

#[derive(Debug, Clone, Copy)]
enum Op {
    Kill,
    Hostile,
    Friendly,
    Remove,
    Meet,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::Kill),
        Just(Op::Hostile),
        Just(Op::Friendly),
        Just(Op::Remove),
        Just(Op::Meet),
    ]
}

const STATES: [NpcState; 4] = [
    NpcState::Alive,
    NpcState::Dead,
    NpcState::Hostile,
    NpcState::Met,
];

proptest! {
    #[test]
    fn the_laws_hold_after_every_effect(ops in prop::collection::vec(op(), 0..40)) {
        let doc = npc("doc");
        let mut world = world();
        for op in ops {
            let dead = world.npc_is(&doc, NpcState::Dead);
            let gone = STATES.iter().all(|state| !world.npc_is(&doc, *state));
            let _ = match op {
                Op::Kill => world.kill_npc(&doc),
                Op::Hostile => world.set_hostile(&doc),
                Op::Friendly => world.set_friendly(&doc),
                Op::Remove => world.remove_npc(&doc),
                Op::Meet => world.meet(&doc),
            };

            // The dead stay dead or are removed; the removed never come back.
            if dead {
                prop_assert!(
                    world.npc_is(&doc, NpcState::Dead)
                        || STATES.iter().all(|state| !world.npc_is(&doc, *state))
                );
            }
            if gone {
                prop_assert!(STATES.iter().all(|state| !world.npc_is(&doc, *state)));
            }
            // Alive and dead exclude each other, and hostile implies alive.
            prop_assert!(
                !(world.npc_is(&doc, NpcState::Alive) && world.npc_is(&doc, NpcState::Dead))
            );
            if world.npc_is(&doc, NpcState::Hostile) {
                prop_assert!(world.npc_is(&doc, NpcState::Alive));
            }
            // The other NPC is never touched.
            prop_assert!(world.npc_is(&npc("sunny"), NpcState::Alive));
        }
    }

    #[test]
    fn a_refused_effect_changes_nothing(ops in prop::collection::vec(op(), 0..20)) {
        let doc = npc("doc");
        let mut world = world();
        for op in ops {
            let before: Vec<bool> = STATES.iter().map(|s| world.npc_is(&doc, *s)).collect();
            let result = match op {
                Op::Kill => world.kill_npc(&doc),
                Op::Hostile => world.set_hostile(&doc),
                Op::Friendly => world.set_friendly(&doc),
                Op::Remove => world.remove_npc(&doc),
                Op::Meet => world.meet(&doc),
            };
            if result.is_err() {
                let after: Vec<bool> = STATES.iter().map(|s| world.npc_is(&doc, *s)).collect();
                prop_assert_eq!(after, before);
            }
        }
    }
}
