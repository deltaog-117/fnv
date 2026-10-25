//! A whole fight through `Game`, over the real Goodsprings data: the encounter is
//! offered by quest state, the fighters come from the character, the pack and the
//! NPC records, and the wounds and the winnings come back.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use engine::combat::{Combatant, Dice, Power, Side, Status};
use engine::game::{Arsenal, Battle, BattleError, Game, Outcome};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{
    Effect, EncounterId, GameData, InventoryView, ItemId, LocationId, NpcId, NpcState, QuestId,
    QuestStatus, QuestView, Scope, WorldView, apply_all,
};
use engine::world::WorldBook;
use proptest::prelude::*;

struct Fixture {
    quests: Arc<QuestBook>,
    world: Arc<WorldBook>,
    items: Arc<ItemBook>,
    arsenal: Arc<Arsenal>,
}

static FIXTURE: LazyLock<Fixture> = LazyLock::new(|| {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Around(place())).expect("the engine data loads");
    Fixture {
        quests: Arc::new(QuestBook::new(data.quests.iter()).expect("the quests build")),
        world: Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id))),
        items: Arc::new(ItemBook::from_data(&data).expect("the item book builds")),
        arsenal: Arc::new(Arsenal::from_data(&data).expect("the arsenal builds")),
    }
});

fn place() -> LocationId {
    LocationId::try_new("goodsprings").expect("literal is a valid id")
}

fn gunfight() -> EncounterId {
    EncounterId::try_new("ghost_town_gunfight").expect("literal is a valid id")
}

fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("literal is a valid id")
}

fn effect(json: serde_json::Value) -> Effect {
    serde_json::from_value(json).expect("the literal is a valid effect")
}

fn stage(quest: &str, stage: u32) -> Effect {
    effect(serde_json::json!({"type": "set_stage", "quest": quest, "stage": stage}))
}

/// A new game with a 9mm pistol in hand and Sunny's request taken (stage 10).
fn armed_game() -> Game {
    let fixture = &*FIXTURE;
    let mut game = Game::with_world(Arc::clone(&fixture.quests), Arc::clone(&fixture.world))
        .with_items(Arc::clone(&fixture.items))
        .with_arsenal(Arc::clone(&fixture.arsenal));
    let pistol = item("9mm_pistol");
    apply_all(
        &[
            effect(serde_json::json!({"type": "give_item", "item": "9mm_pistol"})),
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
        ],
        &mut game,
    )
    .expect("the setup effects apply");
    game.equip(&pistol).expect("the pistol is carried");
    game
}

/// The same, wearing leather armor: a fair fight needs it.
fn dressed_game() -> Game {
    let mut game = armed_game();
    apply_all(
        &[effect(
            serde_json::json!({"type": "give_item", "item": "leather_armor"}),
        )],
        &mut game,
    )
    .expect("the armor is given");
    game.equip(&item("leather_armor"))
        .expect("the armor is carried");
    game
}

/// Every roll the same: 1 always hits and always crits, 100 always misses.
struct Always(u8);

impl Dice for Always {
    fn percent(&mut self) -> u8 {
        self.0
    }
}

/// A small linear congruential generator, so a property test can name a fight by
/// one number.
struct Seeded(u64);

impl Dice for Seeded {
    fn percent(&mut self) -> u8 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u8::try_from((self.0 >> 33) % 100 + 1).expect("1 to 100 fits a byte")
    }
}

/// The player shoots the first foe still standing, and passes when out of action
/// points; `pass_only` makes the player never shoot.
fn play(battle: &mut Battle, dice: &mut impl Dice, pass_only: bool) {
    // Every turn either shoots (spending action points) or passes, so a fight with
    // a few fighters ends well inside this bound.
    for _ in 0..2_000 {
        if battle.combat().status() != Status::Ongoing {
            return;
        }
        let target = battle.combat().ids().find(|&id| {
            battle.combat().side(id) == Some(Side::Enemy) && battle.combat().is_standing(id)
        });
        let shot = target
            .filter(|_| !pass_only)
            .and_then(|id| battle.attack(id, Power::Normal, dice).ok());
        if shot.is_none() {
            battle.end_turn(dice).expect("the fight is not over");
        }
    }
    panic!("the fight did not end");
}

#[test]
fn the_arsenal_prepares_every_encounter_in_the_whole_data() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let arsenal = Arsenal::from_data(&data).expect("every encounter is well formed");
    assert_eq!(arsenal.encounters().count(), data.encounters.len());
    assert!(arsenal.encounter(&gunfight()).is_some());
}

#[test]
fn the_gunfight_is_offered_only_once_sunny_has_agreed() {
    let fixture = &*FIXTURE;
    let mut game = Game::with_world(Arc::clone(&fixture.quests), Arc::clone(&fixture.world))
        .with_items(Arc::clone(&fixture.items))
        .with_arsenal(Arc::clone(&fixture.arsenal));
    assert!(game.encounters_at(&place()).is_empty());
    assert!(matches!(
        game.start_battle(&gunfight()),
        Err(BattleError::NotOffered(_))
    ));
    apply_all(
        &[
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
        ],
        &mut game,
    )
    .expect("the quest advances");
    let offered = game.encounters_at(&place());
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].id, gunfight());
    let elsewhere = LocationId::try_new("primm").expect("literal is a valid id");
    assert!(game.encounters_at(&elsewhere).is_empty());
}

#[test]
fn a_player_with_nothing_in_hand_cannot_start_a_fight() {
    let mut game = armed_game();
    game.unequip(engine::inventory::Slot::Weapon);
    assert!(matches!(
        game.start_battle(&gunfight()),
        Err(BattleError::Unarmed)
    ));
}

#[test]
fn the_player_fights_with_the_numbers_of_their_character_weapon_and_armor() {
    let mut game = armed_game();
    apply_all(
        &[effect(
            serde_json::json!({"type": "give_item", "item": "leather_armor"}),
        )],
        &mut game,
    )
    .expect("the armor is given");
    let bare = game.start_battle(&gunfight()).expect("the fight starts");
    let you = bare.combat().combatant(bare.player()).expect("the player");
    assert_eq!(you.name(), "You");
    assert_eq!(you.max_hp().get(), game.character().max_hp());
    assert_eq!(you.max_ap(), game.character().action_points());
    // The 9mm pistol: 16 damage for 17 action points; no armor is worn yet.
    assert_eq!(you.attack().ap_cost.get(), 17);
    assert_eq!(you.attack().base_damage.get(), 16.0);
    assert_eq!(you.dt().get(), 0.0);

    game.equip(&item("leather_armor"))
        .expect("the armor is carried");
    let dressed = game.start_battle(&gunfight()).expect("the fight starts");
    let you = dressed
        .combat()
        .combatant(dressed.player())
        .expect("the player");
    assert_eq!(you.dt().get(), 6.0);
}

#[test]
fn the_foes_come_from_the_npc_records_and_the_authored_loadouts() {
    let game = armed_game();
    let battle = game.start_battle(&gunfight()).expect("the fight starts");
    let combat = battle.combat();
    let foes: Vec<_> = combat
        .ids()
        .filter(|&id| combat.side(id) == Some(Side::Enemy))
        .map(|id| {
            let foe = combat.combatant(id).expect("a foe");
            (foe.name().to_owned(), foe.max_hp().get(), foe.dt().get())
        })
        .collect();
    assert_eq!(
        foes,
        [
            ("Joe Cobb".to_owned(), 50, 5.0),
            ("Powder Ganger".to_owned(), 30, 0.0),
            ("Powder Ganger".to_owned(), 30, 0.0),
        ]
    );
}

#[test]
fn winning_pays_the_quest_kills_joe_cobb_and_keeps_the_hit_points_left() {
    let mut game = dressed_game();
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    play(&mut battle, &mut Always(1), false);
    assert_eq!(battle.combat().status(), Status::Won(Side::Player));
    let hp_left = battle.combat().hp(battle.player()).expect("the player");

    assert_eq!(game.finish_battle(&battle), Ok(Outcome::Victory));
    assert_eq!(game.hp(), hp_left);
    let cobb = NpcId::try_new("joe_cobb").expect("literal is a valid id");
    assert!(game.world().npc_is(&cobb, NpcState::Dead));
    let quest = QuestId::try_new("ghost_town_gunfight").expect("literal is a valid id");
    assert_eq!(game.quest_status(&quest), QuestStatus::Completed);
    // Stage 70 pays 100 caps and 150 experience.
    assert_eq!(game.caps(), 100);
    assert_eq!(game.character().xp(), 150);
    assert!(
        game.encounters_at(&place()).is_empty(),
        "a won fight is not offered again"
    );
}

#[test]
fn losing_leaves_the_player_down_and_the_fight_unwon() {
    let mut game = armed_game();
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    play(&mut battle, &mut Always(1), true);
    assert_eq!(battle.combat().status(), Status::Won(Side::Enemy));

    assert_eq!(game.finish_battle(&battle), Ok(Outcome::Defeat));
    assert_eq!(game.hp(), 0);
    assert!(!game.is_alive());
    assert_eq!(game.caps(), 0);
    assert!(
        game.encounters_at(&place()).is_empty(),
        "nothing is offered to the dead"
    );
    assert!(matches!(
        game.start_battle(&gunfight()),
        Err(BattleError::PlayerDown)
    ));
}

#[test]
fn a_fight_that_is_not_over_cannot_be_settled_and_changes_nothing() {
    let mut game = armed_game();
    let battle = game.start_battle(&gunfight()).expect("the fight starts");
    assert_eq!(game.finish_battle(&battle), Err(BattleError::Unfinished));
    assert_eq!(game.hp(), game.character().max_hp());
    assert_eq!(game.caps(), 0);
}

#[test]
fn a_wounded_player_starts_the_next_fight_wounded() {
    let mut game = armed_game();
    let mut first = game.start_battle(&gunfight()).expect("the fight starts");
    // Lose on purpose, then heal part of the way and look at the next start.
    play(&mut first, &mut Always(1), true);
    game.finish_battle(&first).expect("the fight settles");
    game.heal(30);
    assert_eq!(game.hp(), 30);
    // Nothing is offered while down, but once healed the fight resumes at 30.
    let second = game.start_battle(&gunfight()).expect("the fight starts");
    assert_eq!(second.combat().hp(second.player()), Some(30));
}

#[test]
fn healing_never_passes_the_maximum() {
    let mut game = armed_game();
    game.heal(1_000_000);
    assert_eq!(game.hp(), game.character().max_hp());
}

proptest! {
    #[test]
    fn any_fight_ends_and_settles_consistently(seed in any::<u64>(), pass_only in any::<bool>()) {
        let mut game = dressed_game();
        let max = game.character().max_hp();
        let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
        play(&mut battle, &mut Seeded(seed), pass_only);
        let status = battle.combat().status();
        let outcome = game.finish_battle(&battle).expect("a finished fight settles");

        prop_assert!(game.hp() <= max);
        prop_assert_eq!(Some(game.hp()), battle.combat().hp(battle.player()));
        match outcome {
            Outcome::Victory => {
                prop_assert_eq!(status, Status::Won(Side::Player));
                prop_assert!(game.is_alive());
                prop_assert!(game.encounters_at(&place()).is_empty());
            }
            Outcome::Defeat => {
                prop_assert_eq!(status, Status::Won(Side::Enemy));
                prop_assert_eq!(game.hp(), 0);
                prop_assert_eq!(game.caps(), 0);
            }
        }
    }
}
