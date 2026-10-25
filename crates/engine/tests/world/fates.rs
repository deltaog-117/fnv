//! Each rule of the world state, one small case at a time.

use engine::shared::{EffectError, NpcState, WorldSink, WorldView};

use crate::support::{ending, flag, npc, world};

#[test]
fn an_untouched_npc_is_alive_and_nothing_else() {
    let world = world();
    assert!(world.npc_is(&npc("doc"), NpcState::Alive));
    for state in [NpcState::Dead, NpcState::Hostile, NpcState::Met] {
        assert!(!world.npc_is(&npc("doc"), state), "{state:?}");
    }
}

#[test]
fn an_npc_outside_the_roster_is_in_no_state_and_effects_on_it_fail() {
    let mut world = world();
    assert!(!world.npc_is(&npc("ghost"), NpcState::Alive));
    let error = world.kill_npc(&npc("ghost")).expect_err("unknown npc");
    assert!(matches!(error, EffectError::Unknown { kind: "npc", .. }));
    assert!(!world.npc_is(&npc("ghost"), NpcState::Dead));
}

#[test]
fn killing_is_idempotent_and_ends_life_and_hostility() {
    let mut world = world();
    world.set_hostile(&npc("doc")).expect("alive");
    world.kill_npc(&npc("doc")).expect("alive");
    world.kill_npc(&npc("doc")).expect("again changes nothing");
    assert!(world.npc_is(&npc("doc"), NpcState::Dead));
    assert!(!world.npc_is(&npc("doc"), NpcState::Alive));
    assert!(!world.npc_is(&npc("doc"), NpcState::Hostile));
}

#[test]
fn only_the_living_change_attitude() {
    let mut world = world();
    world.set_hostile(&npc("doc")).expect("alive");
    assert!(world.npc_is(&npc("doc"), NpcState::Hostile));
    assert!(world.npc_is(&npc("doc"), NpcState::Alive));
    world.set_friendly(&npc("doc")).expect("alive");
    assert!(!world.npc_is(&npc("doc"), NpcState::Hostile));

    world.kill_npc(&npc("sunny")).expect("alive");
    for result in [
        world.set_hostile(&npc("sunny")),
        world.set_friendly(&npc("sunny")),
    ] {
        assert!(matches!(result, Err(EffectError::Refused { .. })));
    }
}

#[test]
fn a_removed_npc_is_gone_not_dead() {
    let mut world = world();
    world.remove_npc(&npc("doc")).expect("known");
    world
        .remove_npc(&npc("doc"))
        .expect("again changes nothing");
    for state in [
        NpcState::Alive,
        NpcState::Dead,
        NpcState::Hostile,
        NpcState::Met,
    ] {
        assert!(!world.npc_is(&npc("doc"), state), "{state:?}");
    }
    assert!(matches!(
        world.kill_npc(&npc("doc")),
        Err(EffectError::Refused { .. })
    ));
}

#[test]
fn removing_a_dead_npc_clears_the_body() {
    let mut world = world();
    world.kill_npc(&npc("doc")).expect("alive");
    world.remove_npc(&npc("doc")).expect("known");
    assert!(!world.npc_is(&npc("doc"), NpcState::Dead));
}

#[test]
fn meeting_is_remembered_after_death_but_not_after_removal() {
    let mut world = world();
    world.meet(&npc("doc")).expect("alive");
    assert!(world.npc_is(&npc("doc"), NpcState::Met));
    world.kill_npc(&npc("doc")).expect("alive");
    assert!(world.npc_is(&npc("doc"), NpcState::Met));
    assert!(matches!(
        world.meet(&npc("doc")),
        Err(EffectError::Refused { .. })
    ));
    world.remove_npc(&npc("doc")).expect("known");
    assert!(!world.npc_is(&npc("doc"), NpcState::Met));
}

#[test]
fn flags_set_and_clear() {
    let mut world = world();
    assert!(!world.flag(&flag("f")));
    world.set_flag(&flag("f"), true).expect("any flag");
    world.set_flag(&flag("f"), true).expect("again");
    assert!(world.flag(&flag("f")));
    world.set_flag(&flag("f"), false).expect("any flag");
    world
        .set_flag(&flag("g"), false)
        .expect("clearing an unset flag");
    assert!(!world.flag(&flag("f")));
}

#[test]
fn an_ending_keeps_the_last_word() {
    let mut world = world();
    assert_eq!(world.ending_reachable(&ending("e")), None);
    world.unlock_ending(&ending("e")).expect("accepted");
    assert_eq!(world.ending_reachable(&ending("e")), Some(true));
    world.lock_ending(&ending("e")).expect("accepted");
    assert_eq!(world.ending_reachable(&ending("e")), Some(false));
}

#[test]
fn a_clone_is_independent() {
    let mut world = world();
    let copy = world.clone();
    world.kill_npc(&npc("doc")).expect("alive");
    world.set_flag(&flag("f"), true).expect("any flag");
    assert!(copy.npc_is(&npc("doc"), NpcState::Alive));
    assert!(!copy.flag(&flag("f")));
}
