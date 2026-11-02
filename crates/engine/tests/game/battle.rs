//! A whole fight through `Game`, over the real Goodsprings data: the encounter is
//! offered by quest state, the fighters come from the character, the pack and the
//! NPC records, and the wounds and the winnings come back.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use engine::combat::{CombatError, Combatant, Dice, Power, Side, Status};
use engine::game::{Arsenal, Battle, BattleError, Game, Outcome};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
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
    reputation: Arc<ReputationBook>,
}

static FIXTURE: LazyLock<Fixture> = LazyLock::new(|| {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Around(place())).expect("the engine data loads");
    Fixture {
        quests: Arc::new(QuestBook::new(data.quests.iter()).expect("the quests build")),
        world: Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id))),
        items: Arc::new(ItemBook::from_data(&data).expect("the item book builds")),
        arsenal: Arc::new(Arsenal::from_data(&data).expect("the arsenal builds")),
        reputation: Arc::new(ReputationBook::from_data(&data).expect("the reputation book builds")),
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

/// A new game with a 9mm pistol in hand and the town ready to fight (stage 65).
fn armed_game() -> Game {
    let fixture = &*FIXTURE;
    let mut game = Game::with_world(Arc::clone(&fixture.quests), Arc::clone(&fixture.world))
        .with_items(Arc::clone(&fixture.items))
        .with_arsenal(Arc::clone(&fixture.arsenal))
        .with_reputation(Arc::clone(&fixture.reputation));
    let pistol = item("9mm_pistol");
    apply_all(
        &[
            effect(serde_json::json!({"type": "give_item", "item": "9mm_pistol"})),
            effect(serde_json::json!({"type": "give_item", "item": "9mm_round", "count": 60})),
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
            stage("ghost_town_gunfight", 60),
            stage("ghost_town_gunfight", 65),
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
        let mut shot = target
            .filter(|_| !pass_only)
            .map(|id| (id, battle.attack(id, Power::Normal, dice)));
        if let Some((id, Err(CombatError::OutOfAmmo { .. }))) = shot
            && battle.reload().is_ok()
        {
            shot = Some((id, battle.attack(id, Power::Normal, dice)));
        }
        if !matches!(shot, Some((_, Ok(_)))) {
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
        .with_arsenal(Arc::clone(&fixture.arsenal))
        .with_reputation(Arc::clone(&fixture.reputation));
    assert!(game.encounters_at(&place()).is_empty());
    assert!(matches!(
        game.start_battle(&gunfight()),
        Err(BattleError::NotOffered(_))
    ));
    apply_all(
        &[
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
            stage("ghost_town_gunfight", 60),
            stage("ghost_town_gunfight", 65),
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

/// The three ways the town can help before the fight; each takes one Powder Ganger
/// out of it.
const HELPERS: [&str; 3] = [
    "trudy_joined_militia",
    "easy_pete_joined_militia",
    "chet_supplied_militia",
];

fn help_from(game: &mut Game, helpers: &[&str]) {
    let flags: Vec<Effect> = helpers
        .iter()
        .map(|flag| effect(serde_json::json!({"type": "set_flag", "flag": flag})))
        .collect();
    apply_all(&flags, game).expect("the flags are set");
}

/// What the foes of a started fight are: name, hit points and damage threshold.
fn foes_of(battle: &Battle) -> Vec<(String, u32, f64)> {
    let combat = battle.combat();
    combat
        .ids()
        .filter(|&id| combat.side(id) == Some(Side::Enemy))
        .map(|id| {
            let foe = combat.combatant(id).expect("a foe");
            (foe.name().to_owned(), foe.max_hp().get(), foe.dt().get())
        })
        .collect()
}

#[test]
fn an_unprepared_town_faces_the_wikis_six_attackers() {
    let game = armed_game();
    let battle = game.start_battle(&gunfight()).expect("the fight starts");
    let mut expected = vec![("Joe Cobb".to_owned(), 50, 5.0)];
    expected.extend(std::iter::repeat_n(
        ("Powder Ganger".to_owned(), 30, 0.0),
        5,
    ));
    assert_eq!(foes_of(&battle), expected);
}

#[test]
fn a_fully_prepared_town_faces_cobb_and_two_gangers() {
    let mut game = armed_game();
    help_from(&mut game, &HELPERS);
    let battle = game.start_battle(&gunfight()).expect("the fight starts");
    assert_eq!(
        foes_of(&battle),
        [
            ("Joe Cobb".to_owned(), 50, 5.0),
            ("Powder Ganger".to_owned(), 30, 0.0),
            ("Powder Ganger".to_owned(), 30, 0.0),
        ]
    );
}

#[test]
fn each_helper_takes_exactly_one_attacker_out_of_the_fight() {
    for helper in HELPERS {
        let mut game = armed_game();
        help_from(&mut game, &[helper]);
        let battle = game.start_battle(&gunfight()).expect("the fight starts");
        assert_eq!(foes_of(&battle).len(), 5, "{helper}");
    }
}

#[test]
fn winning_pays_the_quest_kills_joe_cobb_and_keeps_the_hit_points_left() {
    let mut game = dressed_game();
    help_from(&mut game, &HELPERS);
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

fn rounds_held(game: &Game) -> u32 {
    game.item_count(&item("9mm_round"))
}

#[test]
fn the_player_starts_loaded_from_the_pack_and_foes_have_endless_rounds() {
    let game = armed_game();
    let battle = game.start_battle(&gunfight()).expect("the fight starts");
    let combat = battle.combat();
    let you = combat
        .rounds(battle.player())
        .expect("the pistol has a clip");
    // A 13-round clip, and the 47 left in the pack are the reserve.
    assert_eq!((you.loaded, you.reserve), (13, Some(47)));
    for id in combat.ids().filter(|&id| id != battle.player()) {
        assert_eq!(combat.rounds(id).expect("they carry pistols").reserve, None);
    }
}

#[test]
fn the_rounds_fired_leave_the_pack_when_the_fight_is_settled() {
    let mut game = dressed_game();
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    play(&mut battle, &mut Always(1), false);
    let rounds = battle.combat().rounds(battle.player()).expect("a clip");
    let left = rounds.total().expect("the pack is counted");
    assert!(left < 60, "the fight cost something");

    game.finish_battle(&battle).expect("the fight settles");
    assert_eq!(rounds_held(&game), left);
}

#[test]
fn a_pistol_with_no_rounds_cannot_shoot() {
    let mut game = dressed_game();
    let all = rounds_held(&game);
    apply_all(
        &[effect(
            serde_json::json!({"type": "take_item", "item": "9mm_round", "count": all}),
        )],
        &mut game,
    )
    .expect("the rounds are taken");
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    let foe = battle
        .combat()
        .ids()
        .find(|&id| battle.combat().side(id) == Some(Side::Enemy))
        .expect("a foe");
    assert!(matches!(
        battle.attack(foe, Power::Normal, &mut Always(1)),
        Err(CombatError::OutOfAmmo { loaded: 0, .. })
    ));
    assert_eq!(battle.reload(), Err(CombatError::NothingToReload));
}

#[test]
fn the_pack_is_untouched_until_the_fight_is_settled() {
    let game = dressed_game();
    let before = rounds_held(&game);
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    let foe = battle
        .combat()
        .ids()
        .find(|&id| battle.combat().side(id) == Some(Side::Enemy))
        .expect("a foe");
    battle
        .attack(foe, Power::Normal, &mut Always(100))
        .expect("a shot, wasted");
    assert_eq!(rounds_held(&game), before);
}

/// A dressed player in Run Goodsprings Run at `stage`, with `flags` set.
fn run_game(stage_number: u32, flags: &[&str]) -> Game {
    let mut game = dressed_game();
    apply_all(&[stage("run_goodsprings_run", stage_number)], &mut game)
        .expect("the quest advances");
    help_from(&mut game, flags);
    game
}

fn npc(text: &str) -> NpcId {
    NpcId::try_new(text).expect("literal is a valid id")
}

fn encounter(text: &str) -> EncounterId {
    EncounterId::try_new(text).expect("literal is a valid id")
}

#[test]
fn killing_ringo_is_offered_only_at_stage_ten_and_moves_the_quest_to_fifteen() {
    for (stage_number, offered) in [(10, true), (15, false), (20, false)] {
        let game = run_game(stage_number, &[]);
        let on_offer = game
            .encounters_at(&place())
            .iter()
            .any(|e| e.id == encounter("kill_ringo"));
        assert_eq!(on_offer, offered, "stage {stage_number}");
    }
    let mut game = run_game(10, &[]);
    let mut battle = game
        .start_battle(&encounter("kill_ringo"))
        .expect("the fight starts");
    assert_eq!(foes_of(&battle).len(), 1);
    play(&mut battle, &mut Always(1), false);
    assert_eq!(game.finish_battle(&battle), Ok(Outcome::Victory));
    assert!(game.world().npc_is(&npc("ringo"), NpcState::Dead));
    let quest = QuestId::try_new("run_goodsprings_run").expect("literal is a valid id");
    assert_eq!(game.quest_stage(&quest).map(|s| s.get()), Some(15));
}

#[test]
fn the_militia_fight_waits_for_the_takeover_call() {
    for (stage_number, offered) in [(45, false), (60, true), (70, false)] {
        let game = run_game(stage_number, &[]);
        let on_offer = game
            .encounters_at(&place())
            .iter()
            .any(|e| e.id == encounter("take_over_goodsprings"));
        assert_eq!(on_offer, offered, "stage {stage_number}");
    }
}

#[test]
fn shaking_chet_down_keeps_trudy_out_of_the_militia() {
    let id = encounter("take_over_goodsprings");
    let plain = run_game(60, &[]).start_battle(&id).expect("starts");
    let shaken = run_game(60, &["chet_shaken_down"])
        .start_battle(&id)
        .expect("starts");
    assert_eq!(foes_of(&plain).len(), 3);
    assert_eq!(foes_of(&shaken).len(), 2);
}

#[test]
fn winning_the_militia_fight_finishes_the_quest_and_kills_the_defenders() {
    use engine::shared::{Renown, StandingView};

    let mut game = run_game(60, &["chet_shaken_down"]);
    let mut battle = game
        .start_battle(&encounter("take_over_goodsprings"))
        .expect("the fight starts");
    play(&mut battle, &mut Always(1), false);
    assert_eq!(game.finish_battle(&battle), Ok(Outcome::Victory));
    let quest = QuestId::try_new("run_goodsprings_run").expect("literal is a valid id");
    assert_eq!(game.quest_status(&quest), QuestStatus::Completed);
    for who in ["sunny_smiles", "easy_pete", "trudy"] {
        assert!(game.world().npc_is(&npc(who), NpcState::Dead), "{who}");
    }
    let town = engine::shared::FactionId::try_new("goodsprings").expect("valid id");
    assert!(game.renown_points(&town, Renown::Infamy) >= 3.0);
}

#[test]
fn a_chems_resistance_adds_to_the_armor_in_the_next_fight_and_ends_with_the_chem() {
    use engine::clock::Span;

    let mut game = dressed_game();
    help_from(&mut game, &HELPERS);
    apply_all(
        &[effect(
            serde_json::json!({"type": "give_item", "item": "med_x"}),
        )],
        &mut game,
    )
    .expect("the chem is given");
    let resistance = |game: &Game| {
        let battle = game.start_battle(&gunfight()).expect("the fight starts");
        let you = battle.combat().combatant(battle.player()).expect("you");
        you.dr().get()
    };
    let bare = resistance(&game);
    game.use_item(&item("med_x")).expect("Med-X is used");
    assert_eq!(resistance(&game), bare + 25.0);
    let four = std::num::NonZeroU32::new(4).expect("not zero");
    game.wait(Span::minutes(four)).expect("time passes");
    assert_eq!(resistance(&game), bare);
}
