//! The combat rules against the real data in `data/engine/`.

use std::fs;
use std::path::Path;

use engine::combat::{Points, Rules};
use engine::shared::{GameData, Scope};

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_standard_rules_match_the_curated_formulas() {
    let text = fs::read_to_string(root().join("data/engine/formulas.json")).expect("file exists");
    let file: serde_json::Value = serde_json::from_str(&text).expect("file parses");
    let constants = &file["constants"];
    let from_file = Rules::new(
        constants["dr_cap"].as_f64().expect("dr_cap is a number"),
        constants["min_damage_fraction"]
            .as_f64()
            .expect("min_damage_fraction is a number"),
    )
    .expect("the file's constants are valid");
    assert_eq!(Rules::standard(), from_file);
}

/// Weapons the wiki lists with an action-point cost of 0: NPC, creature and special
/// weapons. A fight needs a cost of at least 1, so the adapter that builds an
/// `AttackProfile` from a weapon must decide what to do with these.
const FREE_WEAPONS: [&str; 9] = [
    "Hologram beam",
    "Laser detonator",
    "Mega robo-scorpion claw",
    "Miss Fortune's Bad Luck Bringer",
    "Mysterious Stranger's .44 Magnum",
    "Rorschach drawing",
    "Securitron 9mm submachinegun",
    "Slave's Burden",
    "X-25 Gatling laser",
];

#[test]
fn every_real_weapon_and_armor_fits_the_combat_types() {
    let data = GameData::load(&root().join("data/engine"), &Scope::Everywhere).expect("data loads");
    let mut free = Vec::new();
    for weapon in data.weapons.iter() {
        assert!(
            Points::try_new(weapon.damage).is_ok(),
            "{}: damage",
            weapon.name
        );
        if weapon.ap_cost.is_some_and(|cost| cost < 1.0) {
            free.push(weapon.name.as_str());
        }
    }
    free.sort_unstable();
    let mut known = FREE_WEAPONS.to_vec();
    known.sort_unstable();
    assert_eq!(
        free, known,
        "weapons with an action-point cost below 1 changed"
    );
    for armor in data.armor.iter() {
        assert!(Points::try_new(armor.dt).is_ok(), "{}: dt", armor.name);
        if let Some(dr) = armor.dr {
            assert!(Points::try_new(dr).is_ok(), "{}: dr", armor.name);
        }
    }
}
