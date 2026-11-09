//! Companions through `Game`, over the real data: who will join, the party limit,
//! the perk a companion grants, going home when the player's standing turns, and
//! fighting beside the player.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use engine::character::PerkBook;
use engine::combat::{Dice, Side};
use engine::companions::CompanionBook;
use engine::game::{Arsenal, Game};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::saves::Save;
use engine::shared::{
    CompanionId, Condition, Effect, EffectError, GameData, InventorySink, ItemId, NpcId, Scope,
    apply_all, evaluate,
};
use engine::world::WorldBook;

struct Fixture {
    companions: Arc<CompanionBook>,
    template: Game,
}

static FIXTURE: LazyLock<Fixture> = LazyLock::new(|| {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("the quests build"));
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    let items = Arc::new(ItemBook::from_data(&data).expect("the item book builds"));
    let arsenal = Arc::new(Arsenal::from_data(&data).expect("the arsenal builds"));
    let reputation =
        Arc::new(ReputationBook::from_data(&data).expect("the reputation book builds"));
    let companions = Arc::new(CompanionBook::from_data(&data).expect("the companions build"));
    let template = Game::with_world(quests, world)
        .with_items(items)
        .with_arsenal(arsenal)
        .with_reputation(reputation)
        .with_perks(Arc::new(PerkBook::base_game(&data)))
        .with_companions(Arc::clone(&companions));
    Fixture {
        companions,
        template,
    }
});

fn game() -> Game {
    FIXTURE.template.clone()
}

fn who(name: &str) -> CompanionId {
    CompanionId::try_new(name.to_owned()).expect("literal is a valid id")
}

fn effect(json: serde_json::Value) -> Effect {
    serde_json::from_value(json).expect("the literal is a valid effect")
}

fn join(game: &mut Game, name: &str) -> Result<(), EffectError> {
    let wanted = effect(serde_json::json!({"type": "companion_join", "companion": name}));
    apply_all(&[wanted], game).map_err(|error| error.source)
}

fn leave(game: &mut Game, name: &str) -> Result<(), EffectError> {
    let wanted = effect(serde_json::json!({"type": "companion_leave", "companion": name}));
    apply_all(&[wanted], game).map_err(|error| error.source)
}

fn set_standing(game: &mut Game, faction: &str, level: &str) {
    apply_all(
        &[effect(serde_json::json!(
            {"type": "set_reputation", "faction": faction, "level": level}
        ))],
        game,
    )
    .expect("the faction tracks reputation");
}

fn active(game: &Game, name: &str) -> bool {
    let asked: Condition = serde_json::from_value(
        serde_json::json!({"type": "companion", "companion": name, "active": true}),
    )
    .expect("the literal is a valid condition");
    evaluate(&asked, game)
}

#[test]
fn every_companion_in_the_data_builds_and_names_a_real_person() {
    let book = &FIXTURE.companions;
    assert_eq!(book.iter().count(), 8);
    for companion in book.iter() {
        assert_eq!(
            book.of_npc(&companion.loadout.npc),
            Some(&companion.id),
            "{}",
            companion.id
        );
    }
}

#[test]
fn someone_who_joins_is_in_the_party_and_a_condition_can_ask() {
    let mut game = game();
    assert!(!active(&game, "boone"));
    join(&mut game, "boone").expect("Boone joins a player he has no quarrel with");
    assert_eq!(game.party(), [who("boone")]);
    assert!(active(&game, "boone"));
    leave(&mut game, "boone").expect("and goes home when asked");
    assert!(game.party().is_empty());
    assert!(!active(&game, "boone"));
}

#[test]
fn nobody_unknown_joins_and_nobody_leaves_who_never_came() {
    let mut game = game();
    assert!(matches!(
        join(&mut game, "nobody"),
        Err(EffectError::Refused { .. })
    ));
    assert!(matches!(
        leave(&mut game, "boone"),
        Err(EffectError::Refused { .. })
    ));
}

#[test]
fn the_party_holds_two_and_a_third_is_refused() {
    let mut game = game();
    join(&mut game, "boone").expect("first");
    join(&mut game, "raul_tejada").expect("second");
    let third = join(&mut game, "rex");
    assert!(
        matches!(&third, Err(EffectError::Refused { reason, .. }) if reason.contains("full")),
        "{third:?}"
    );
    assert_eq!(game.party().len(), 2);
}

#[test]
fn a_companion_who_dislikes_the_players_standing_will_not_join() {
    let mut game = game();
    set_standing(&mut game, "new_california_republic", "shunned");
    for name in ["boone", "cass"] {
        let refused = join(&mut game, name);
        assert!(
            matches!(&refused, Err(EffectError::Refused { reason, .. }) if reason.contains("will not")),
            "{name}: {refused:?}"
        );
    }
    join(&mut game, "raul_tejada").expect("Raul has no quarrel with the NCR");
}

#[test]
fn the_legion_turns_away_boone_and_arcade_but_not_cass() {
    let mut game = game();
    set_standing(&mut game, "caesar_s_legion", "accepted");
    assert!(join(&mut game, "boone").is_err());
    assert!(join(&mut game, "arcade_israel_gannon").is_err());
    join(&mut game, "cass").expect("the Legion does not bother Cass");
}

#[test]
fn a_companion_goes_home_when_the_players_standing_turns_against_them() {
    let mut game = game();
    join(&mut game, "veronica").expect("joins");
    join(&mut game, "cass").expect("joins");
    set_standing(&mut game, "brotherhood_of_steel", "vilified");
    assert_eq!(game.party(), [who("cass")], "Veronica leaves, Cass stays");
    set_standing(&mut game, "new_california_republic", "shunned");
    assert!(game.party().is_empty(), "and then Cass");
}

#[test]
fn a_companion_who_dies_leaves_the_party_and_cannot_rejoin() {
    let mut game = game();
    join(&mut game, "boone").expect("joins");
    let npc = NpcId::try_new("boone".to_owned()).expect("literal is a valid id");
    apply_all(&[Effect::KillNpc { npc }], &mut game).expect("Boone dies");
    assert!(game.party().is_empty());
    assert!(matches!(
        join(&mut game, "boone"),
        Err(EffectError::Refused { reason, .. }) if reason.contains("dead")
    ));
}

fn stimpak_heal(mut game: Game) -> u32 {
    let stimpak = ItemId::try_new("stimpak".to_owned()).expect("literal is a valid id");
    InventorySink::give_item(&mut game, &stimpak, NonZeroU32::MIN).expect("given");
    let mut game = game.with_wounds(100);
    game.use_item(&stimpak).expect("the stimpak works")
}

#[test]
fn arcade_makes_healing_work_twenty_percent_better_only_while_he_is_with_the_player() {
    let plain = stimpak_heal(game());
    let mut with_arcade = game();
    join(&mut with_arcade, "arcade_israel_gannon").expect("Arcade joins");
    let better = stimpak_heal(with_arcade.clone());
    assert!(better > plain, "{better} > {plain}");
    leave(&mut with_arcade, "arcade_israel_gannon").expect("and leaves");
    assert_eq!(stimpak_heal(with_arcade), plain, "his perk goes with him");
}

struct Miss;

impl Dice for Miss {
    fn percent(&mut self) -> u8 {
        100
    }
}

#[test]
fn a_companion_fights_beside_the_player() {
    let gunfight = engine::shared::EncounterId::try_new("ghost_town_gunfight".to_owned())
        .expect("literal is a valid id");
    let stage = |quest: &str, stage: u32| {
        effect(serde_json::json!({"type": "set_stage", "quest": quest, "stage": stage}))
    };
    let setup = |mut game: Game| {
        apply_all(
            &[
                effect(serde_json::json!({"type": "give_item", "item": "9mm_pistol"})),
                effect(serde_json::json!({"type": "give_item", "item": "9mm_round", "count": 60})),
                effect(serde_json::json!({"type": "give_item", "item": "leather_armor"})),
                stage("ghost_town_gunfight", 5),
                stage("ghost_town_gunfight", 10),
                stage("ghost_town_gunfight", 60),
                stage("ghost_town_gunfight", 65),
            ],
            &mut game,
        )
        .expect("the setup applies");
        let item = |text: &str| ItemId::try_new(text.to_owned()).expect("a valid id");
        game.equip(&item("9mm_pistol")).expect("carried");
        game.equip(&item("leather_armor")).expect("carried");
        game
    };
    let on_the_players_side = |game: &Game| {
        let mut battle = game.start_battle(&gunfight).expect("the fight starts");
        battle.open(&mut Miss);
        let combat = battle.combat();
        combat
            .ids()
            .filter(|&id| combat.side(id) == Some(Side::Player))
            .count()
    };
    let alone = setup(game());
    let mut accompanied = setup(game());
    join(&mut accompanied, "boone").expect("Boone joins");
    assert_eq!(
        on_the_players_side(&accompanied),
        on_the_players_side(&alone) + 1
    );
}

#[test]
fn the_party_survives_saving_and_loading_and_a_stranger_in_a_save_is_refused() {
    let mut game = game();
    join(&mut game, "arcade_israel_gannon").expect("joins");
    join(&mut game, "cass").expect("joins");
    let text = game.save().to_json().expect("a save writes");
    let save = Save::from_json(&text).expect("a written save reads");
    let loaded = FIXTURE.template.restore(&save).expect("it loads");
    assert_eq!(loaded.party(), game.party());
    assert_eq!(loaded.save(), game.save());
    let stimpak = ItemId::try_new("stimpak".to_owned()).expect("literal is a valid id");
    assert_eq!(
        loaded.character().heal_percent(&stimpak),
        game.character().heal_percent(&stimpak),
        "Arcade's perk is back too"
    );
    assert_eq!(game.character().heal_percent(&stimpak), 20);

    let mut strange = game.save();
    strange.party = vec![who("a_stranger")];
    assert!(FIXTURE.template.restore(&strange).is_err());
    let mut crowded = game.save();
    crowded.party = vec![who("boone"), who("cass"), who("rex")];
    assert!(FIXTURE.template.restore(&crowded).is_err());
}

#[test]
fn a_version_five_save_loads_with_nobody_in_the_party() {
    let mut value: serde_json::Value =
        serde_json::from_str(&game().save().to_json().expect("a save writes")).expect("json");
    let object = value.as_object_mut().expect("a save is an object");
    object.remove("party");
    object.insert("version".to_owned(), serde_json::Value::from(5));
    let save = Save::from_json(&value.to_string()).expect("a version 5 save upgrades");
    let loaded = FIXTURE.template.restore(&save).expect("and loads");
    assert!(loaded.party().is_empty());
}
