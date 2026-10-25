//! Golden combat scenarios from `tests/scenarios/combat.json`: language-neutral JSON
//! (fighters, scripted rolls, moves, expected state), so any implementation of the
//! game can be checked against the same spec.

use std::fs;
use std::path::Path;

use engine::combat::{Combat, Power, Side, Status};
use serde::Deserialize;

use crate::support::{Plain, Rolls, percent};
use engine::combat::{FlatHitChance, Rules};

#[derive(Deserialize)]
struct File {
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    hit_chance: u32,
    fighters: Vec<FighterSpec>,
    rolls: Vec<u8>,
    steps: Vec<Step>,
    expect: Expect,
}

#[derive(Deserialize)]
struct FighterSpec {
    side: String,
    hp: u32,
    ap: u32,
    #[serde(default)]
    dt: f64,
    #[serde(default)]
    dr: f64,
    #[serde(default)]
    crit_chance: u32,
    #[serde(default)]
    crit_damage: f64,
    damage: f64,
    ap_cost: u32,
}

#[derive(Deserialize)]
struct Step {
    action: String,
    target: Option<usize>,
    power: Option<String>,
    #[serde(default)]
    fails: bool,
}

#[derive(Deserialize)]
struct Expect {
    hp: Vec<u32>,
    status: String,
    round: u32,
    ap_left: u32,
}

fn side(name: &str) -> Side {
    match name {
        "player" => Side::Player,
        "enemy" => Side::Enemy,
        other => panic!("unknown side {other:?} in the scenario file"),
    }
}

#[test]
fn the_combat_scenarios_hold() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scenarios/combat.json");
    let text = fs::read_to_string(path).expect("the scenario file exists");
    let file: File = serde_json::from_str(&text).expect("the scenario file parses");
    assert!(!file.scenarios.is_empty());

    for s in file.scenarios {
        let name = &s.name;
        let fighters = s.fighters.iter().map(|f| {
            let plain = Plain::new(f.hp, f.damage)
                .armor(f.dt, f.dr)
                .crit(f.crit_chance, f.crit_damage)
                .ap(f.ap, f.ap_cost);
            (side(&f.side), plain)
        });
        let mut combat = Combat::new(
            fighters,
            Rules::standard(),
            FlatHitChance(percent(s.hit_chance)),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let ids: Vec<_> = combat.ids().collect();
        let mut dice = Rolls::new(&s.rolls);

        for step in &s.steps {
            let result = match step.action.as_str() {
                "attack" => {
                    let target = ids[step
                        .target
                        .unwrap_or_else(|| panic!("{name}: attack needs a target"))];
                    let power = match step.power.as_deref() {
                        None | Some("normal") => Power::Normal,
                        Some("boosted") => Power::Boosted,
                        Some(other) => panic!("{name}: unknown power {other:?}"),
                    };
                    combat.attack(target, power, &mut dice).map(|_| ())
                }
                "end_turn" => combat.end_turn(),
                other => panic!("{name}: unknown action {other:?}"),
            };
            assert_eq!(
                result.is_err(),
                step.fails,
                "{name}: {step_action} -> {result:?}",
                step_action = step.action
            );
        }

        let hps: Vec<u32> = ids.iter().map(|i| combat.hp(*i).expect("own id")).collect();
        assert_eq!(hps, s.expect.hp, "{name}: hit points");
        let status = match combat.status() {
            Status::Ongoing => "ongoing",
            Status::Won(Side::Player) => "won_player",
            Status::Won(Side::Enemy) => "won_enemy",
        };
        assert_eq!(status, s.expect.status, "{name}: status");
        assert_eq!(combat.round(), s.expect.round, "{name}: round");
        assert_eq!(combat.ap_left(), s.expect.ap_left, "{name}: ap left");
    }
}
