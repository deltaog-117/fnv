//! Tests for the data loader: the real `data/engine/` files, and small broken
//! data sets that must fail with an error naming the file and the record.

use std::fs;
use std::path::{Path, PathBuf};

use engine::shared::{
    DialogueId, FactionId, GameData, ItemId, ItemKind, LoadError, LocationId, NpcId, QuestId, Scope,
};
use proptest::prelude::*;

fn real_data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine")
}

fn goodsprings() -> Scope {
    Scope::Around(LocationId::try_new("goodsprings").expect("literal is a valid id"))
}

/// A scratch directory holding a data set with every file present and empty.
/// Cargo gives each test run its own `CARGO_TARGET_TMPDIR`, so no cleanup is
/// needed beyond replacing the directory.
fn empty_data(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("loader")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("dialogue")).expect("scratch directory can be created");
    for file in [
        "locations",
        "quests",
        "npcs",
        "factions",
        "weapons",
        "armor",
        "ammo",
        "consumables",
        "encounters",
        "perks",
        "traits",
    ] {
        write(&dir, &format!("{file}.json"), "[]");
    }
    dir
}

fn write(dir: &Path, file: &str, text: &str) {
    fs::write(dir.join(file), text).expect("scratch file can be written");
}

fn load_error(dir: &Path) -> LoadError {
    GameData::load(dir, &Scope::Everywhere).expect_err("the data set is broken on purpose")
}

#[test]
fn everything_in_data_engine_loads() {
    let data = GameData::load(&real_data(), &Scope::Everywhere).expect("the real data loads");
    assert_eq!(data.weapons.len(), 307);
    assert_eq!(data.armor.len(), 299);
    assert_eq!(data.ammo.len(), 121);
    assert_eq!(data.consumables.len(), 195);
    assert_eq!(data.quests.len(), 256);
    assert_eq!(data.npcs.len(), 880);
    assert_eq!(data.locations.len(), 684);
    assert_eq!(data.factions.len(), 42);
    assert_eq!(data.dialogue.len(), 517);
}

#[test]
fn each_item_file_holds_its_own_kind() {
    let data = GameData::load(&real_data(), &Scope::Everywhere).expect("the real data loads");
    assert!(data.weapons.iter().all(|i| i.kind == ItemKind::Weapon));
    assert!(data.armor.iter().all(|i| i.kind == ItemKind::Armor));
    assert!(data.ammo.iter().all(|i| i.kind == ItemKind::Ammo));
    assert!(
        data.consumables
            .iter()
            .all(|i| i.kind == ItemKind::Consumable)
    );
}

#[test]
fn goodsprings_scope_holds_the_town_and_the_powder_gangers() {
    let data = GameData::load(&real_data(), &goodsprings()).expect("the real data loads");
    let quest = |id: &str| QuestId::try_new(id).expect("literal is a valid id");
    let npc = |id: &str| NpcId::try_new(id).expect("literal is a valid id");
    let dialogue = |id: &str| DialogueId::try_new(id).expect("literal is a valid id");

    assert!(data.quests.contains(&quest("ghost_town_gunfight")));
    assert!(data.quests.contains(&quest("run_goodsprings_run")));
    assert!(!data.quests.contains(&quest("ring_a_ding_ding")));

    for id in [
        "doc_mitchell",
        "sunny_smiles",
        "trudy",
        "joe_cobb",
        "powder_ganger",
    ] {
        assert!(data.npcs.contains(&npc(id)), "{id} should be in scope");
    }
    assert!(!data.npcs.contains(&npc("benny")));

    assert!(data.dialogue.contains(&dialogue("docmitchell")));
    assert!(data.dialogue.contains(&dialogue("gsjoecobb")));

    // Items, factions and locations are not narrowed by the scope.
    assert_eq!(data.weapons.len(), 307);
    assert!(
        data.factions
            .contains(&FactionId::try_new("powder_gangers").expect("valid id"))
    );
    assert_eq!(data.locations.len(), 684);
}

#[test]
fn every_dialogue_file_in_scope_belongs_to_a_scoped_npc() {
    let data = GameData::load(&real_data(), &goodsprings()).expect("the real data loads");
    let linked: Vec<&DialogueId> = data.npcs.iter().flat_map(|n| n.dialogue.iter()).collect();
    assert!(data.dialogue.iter().all(|d| linked.contains(&&d.id)));
}

#[test]
fn a_scope_root_that_does_not_exist_is_reported() {
    let root = LocationId::try_new("atlantis").expect("literal is a valid id");
    let err = GameData::load(&real_data(), &Scope::Around(root)).expect_err("no such place");
    assert!(matches!(err, LoadError::UnknownScopeRoot(_)), "{err}");
}

#[test]
fn a_missing_directory_names_the_missing_file() {
    let err = GameData::load(Path::new("no/such/dir"), &Scope::Everywhere).expect_err("missing");
    assert!(err.to_string().contains("locations.json"), "{err}");
}

#[test]
fn an_unknown_field_names_the_file_and_the_record() {
    let dir = empty_data("unknown_field");
    write(
        &dir,
        "factions.json",
        r#"[{"id":"ncr","name":"NCR","kind":"faction","dlc":"base","colour":"tan"}]"#,
    );
    let message = load_error(&dir).to_string();
    assert!(message.contains("factions.json"), "{message}");
    assert!(message.contains("ncr"), "{message}");
    assert!(message.contains("colour"), "{message}");
}

#[test]
fn a_record_without_an_id_is_named_by_position() {
    let dir = empty_data("no_id");
    write(&dir, "weapons.json", r#"[{"name":"Nameless"}]"#);
    let message = load_error(&dir).to_string();
    assert!(message.contains("weapons.json"), "{message}");
    assert!(message.contains("#0"), "{message}");
}

#[test]
fn a_repeated_id_is_rejected() {
    let dir = empty_data("duplicate");
    let faction = r#"{"id":"ncr","name":"NCR","kind":"faction","dlc":"base"}"#;
    write(&dir, "factions.json", &format!("[{faction},{faction}]"));
    assert!(matches!(load_error(&dir), LoadError::DuplicateId { .. }));
}

#[test]
fn malformed_json_is_reported_with_the_file() {
    let dir = empty_data("malformed");
    write(&dir, "armor.json", "[{");
    let message = load_error(&dir).to_string();
    assert!(message.contains("armor.json"), "{message}");
}

#[test]
fn a_dialogue_topic_may_not_point_outside_its_file() {
    let dir = empty_data("dangling");
    write(
        &dir,
        "dialogue/guard.json",
        r#"{"id":"guard","speaker_name":"Guard","topics":[
            {"key":"greeting","section":"topics","responses":[{"text":"Halt."}],"next":["missing"]}
        ]}"#,
    );
    let err = load_error(&dir);
    assert!(matches!(err, LoadError::DanglingTopic { .. }), "{err}");
}

#[test]
fn a_dialogue_file_must_hold_the_id_its_name_says() {
    let dir = empty_data("mismatch");
    write(
        &dir,
        "dialogue/guard.json",
        r#"{"id":"someone_else","speaker_name":"Guard","topics":[]}"#,
    );
    let err = load_error(&dir);
    assert!(matches!(err, LoadError::DialogueIdMismatch { .. }), "{err}");
}

#[test]
fn a_topic_needs_at_least_one_response() {
    let dir = empty_data("silent");
    write(
        &dir,
        "dialogue/guard.json",
        r#"{"id":"guard","speaker_name":"Guard","topics":[
            {"key":"greeting","section":"topics","responses":[]}
        ]}"#,
    );
    assert!(matches!(load_error(&dir), LoadError::Record { .. }));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Whatever text an item carries as its id, the loader accepts it exactly
    /// when the id type does, and a rejection always names the file.
    #[test]
    fn item_ids_are_accepted_exactly_when_valid(id in "[a-zA-Z0-9_ -]{0,8}") {
        let dir = empty_data("prop_item_id");
        let record = serde_json::json!({
            "id": id, "name": "Thing", "kind": "ammo", "dlc": "base",
            "value": 1, "weight": 1, "family": "ball"
        });
        write(&dir, "ammo.json", &format!("[{record}]"));
        match GameData::load(&dir, &Scope::Everywhere) {
            Ok(data) => {
                prop_assert!(ItemId::try_new(id.clone()).is_ok());
                prop_assert_eq!(data.ammo.len(), 1);
            }
            Err(err) => {
                prop_assert!(ItemId::try_new(id).is_err());
                prop_assert!(err.to_string().contains("ammo.json"));
            }
        }
    }
}
