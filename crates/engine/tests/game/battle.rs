//! A whole fight through `Game`, over the real Goodsprings data: the encounter is
//! offered by quest state, the fighters come from the character, the pack and the
//! NPC records, and the wounds and the winnings come back.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use engine::character::{Character, CharacterRules, Level, PerkBook, SpecialSet};
use engine::combat::{CombatError, Combatant, Dice, Flight, Power, Side, Status};
use engine::game::{Arsenal, Battle, BattleError, Game, Outcome};
use engine::inventory::{CopyId, ItemBook};
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::shared::{
    CharacterView, Effect, EncounterId, GameData, InventorySink, InventoryView, ItemId, LocationId,
    NpcId, NpcState, PerkId, QuestId, QuestStatus, QuestView, Scope, Skill, Special, SpecialLevel,
    TraitId, WorldView, apply_all,
};
use engine::world::WorldBook;
use proptest::prelude::*;

struct Fixture {
    quests: Arc<QuestBook>,
    world: Arc<WorldBook>,
    items: Arc<ItemBook>,
    arsenal: Arc<Arsenal>,
    reputation: Arc<ReputationBook>,
    perks: PerkBook,
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
        perks: PerkBook::base_game(&data),
    }
});

fn place() -> LocationId {
    LocationId::try_new("goodsprings").expect("literal is a valid id")
}

/// Starts the gunfight and plays whatever the foes do before the player's first turn,
/// with dice that make every opening shot miss.
fn start(game: &Game) -> Battle {
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    battle.open(&mut Always(100));
    battle
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
    let bare = start(&game);
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
    let dressed = start(&game);
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
    let battle = start(&game);
    let mut expected = vec![("Joe Cobb".to_owned(), 50, 5.0)];
    expected.extend(std::iter::repeat_n(
        ("Powder Ganger".to_owned(), 30, 0.0),
        5,
    ));
    assert_eq!(foes_of(&battle), expected);
}

/// What fights on the player's side besides the player: name and hit points.
fn allies_of(battle: &Battle) -> Vec<(String, u32)> {
    let combat = battle.combat();
    combat
        .ids()
        .filter(|&id| combat.side(id) == Some(Side::Player) && id != battle.player())
        .map(|id| {
            let ally = combat.combatant(id).expect("an ally");
            (ally.name().to_owned(), ally.max_hp().get())
        })
        .collect()
}

#[test]
fn only_chets_supplies_thin_the_attackers_and_the_other_helpers_fight_beside_the_player() {
    let mut game = armed_game();
    help_from(&mut game, &HELPERS);
    let battle = start(&game);
    // Cobb and four gangers: the supplies keep one away. Trudy, Easy Pete and the two
    // settlers Trudy enlisted with her shoot.
    assert_eq!(foes_of(&battle).len(), 5);
    assert_eq!(
        allies_of(&battle),
        [
            ("Sunny Smiles".to_owned(), 30),
            ("Easy Pete".to_owned(), 25),
            ("Trudy".to_owned(), 35),
            ("Goodsprings settler".to_owned(), 25),
            ("Goodsprings settler".to_owned(), 25),
        ]
    );
}

#[test]
fn each_helper_has_their_own_effect_on_the_fight() {
    let unhelped = start(&armed_game());
    assert_eq!(foes_of(&unhelped).len(), 6);
    assert_eq!(allies_of(&unhelped), [("Sunny Smiles".to_owned(), 30)]);
    // Trudy's Speech or Sneak check enlists the settlers with her, so her flag brings
    // three people; the others bring one or none.
    for (helper, foes, ally, brought) in [
        ("trudy_joined_militia", 6, Some("Trudy"), 3),
        ("easy_pete_joined_militia", 6, Some("Easy Pete"), 1),
        ("chet_supplied_militia", 5, None, 0),
    ] {
        let mut game = armed_game();
        help_from(&mut game, &[helper]);
        let battle = start(&game);
        assert_eq!(foes_of(&battle).len(), foes, "{helper}");
        let allies = allies_of(&battle);
        assert_eq!(allies.len(), 1 + brought, "{helper}");
        assert_eq!(
            allies.iter().any(|(name, _)| Some(name.as_str()) == ally),
            ally.is_some()
        );
    }
}

#[test]
fn a_dead_ally_does_not_join_the_fight() {
    let mut game = armed_game();
    help_from(&mut game, &["trudy_joined_militia"]);
    apply_all(
        &[effect(
            serde_json::json!({"type": "kill_npc", "npc": "trudy"}),
        )],
        &mut game,
    )
    .expect("trudy dies");
    let names: Vec<_> = allies_of(&start(&game))
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        names,
        ["Sunny Smiles", "Goodsprings settler", "Goodsprings settler"],
        "the settlers were enlisted with Trudy and fight on without her"
    );
}

#[test]
fn allies_shoot_for_themselves_while_the_player_passes() {
    let mut game = armed_game();
    help_from(&mut game, &["trudy_joined_militia"]);
    let mut battle = start(&game);
    let allies: Vec<_> = {
        let combat = battle.combat();
        combat
            .ids()
            .filter(|&id| combat.side(id) == Some(Side::Player) && id != battle.player())
            .collect()
    };
    let shots = battle
        .end_turn(&mut Always(100))
        .expect("the player passes");
    assert!(
        shots.iter().any(|shot| allies.contains(&shot.attacker)),
        "an ally took a turn"
    );
    assert!(
        shots
            .iter()
            .all(|shot| battle.combat().side(shot.attacker) != battle.combat().side(shot.target)),
        "nobody shoots their own side"
    );
}

#[test]
fn the_fight_is_lost_when_the_player_falls_even_if_an_ally_still_stands() {
    let game = armed_game();
    // One hit point left, so the first shot that lands finishes the player.
    let max = game.character().max_hp();
    let game = game.with_wounds(max - 1);
    let mut battle = start(&game);
    play(&mut battle, &mut Always(1), true);
    assert_eq!(battle.combat().status(), Status::Won(Side::Enemy));
    let sunny_up = battle
        .combat()
        .ids()
        .filter(|&id| id != battle.player() && battle.combat().side(id) == Some(Side::Player))
        .any(|id| battle.combat().is_standing(id));
    assert!(sunny_up, "Sunny was still standing when the player fell");
    assert_eq!(game.clone().finish_battle(&battle), Ok(Outcome::Defeat));
}

#[test]
fn winning_pays_the_quest_kills_joe_cobb_and_keeps_the_hit_points_left() {
    let mut game = dressed_game();
    help_from(&mut game, &HELPERS);
    let mut battle = start(&game);
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
    let mut battle = start(&game);
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
    let battle = start(&game);
    assert_eq!(game.finish_battle(&battle), Err(BattleError::Unfinished));
    assert_eq!(game.hp(), game.character().max_hp());
    assert_eq!(game.caps(), 0);
}

#[test]
fn a_wounded_player_starts_the_next_fight_wounded() {
    let mut game = armed_game();
    let mut first = start(&game);
    // Lose on purpose, then heal part of the way and look at the next start.
    play(&mut first, &mut Always(1), true);
    game.finish_battle(&first).expect("the fight settles");
    game.heal(30);
    assert_eq!(game.hp(), 30);
    // Nothing is offered while down, but once healed the fight resumes at 30.
    let second = start(&game);
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
        let mut battle = start(&game);
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
            Outcome::Fled => {
                prop_assert_eq!(status, Status::Fled);
                prop_assert!(game.is_alive());
                prop_assert!(!game.encounters_at(&place()).is_empty());
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
    let battle = start(&game);
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
    let mut battle = start(&game);
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
    let mut battle = start(&game);
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
    let mut battle = start(&game);
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
        let battle = start(game);
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

/// Makes `shots` attacks that all miss, while the foes' fire always hits, so the
/// fight is lost and a win never takes the encounter off offer.
fn miss_and_lose(battle: &mut Battle, shots: u32) {
    for _ in 0..shots {
        if battle.combat().status() != Status::Ongoing {
            return;
        }
        let target = battle
            .combat()
            .ids()
            .find(|&id| {
                battle.combat().side(id) == Some(Side::Enemy) && battle.combat().is_standing(id)
            })
            .expect("a foe stands while the fight goes on");
        let missed = battle.attack(target, Power::Normal, &mut Always(100));
        if missed.is_err() && battle.reload().is_err() {
            battle.end_turn(&mut Always(1)).expect("the fight goes on");
        }
    }
    play(battle, &mut Always(1), true);
}

#[test]
fn a_new_weapon_is_whole_and_wears_with_every_shot_fired() {
    let mut game = dressed_game();
    let pistol = wielded(&game);
    assert_eq!(game.copy_condition(pistol).map(|p| p.get()), Some(100));

    let mut battle = start(&game);
    miss_and_lose(&mut battle, 1);
    assert_eq!(battle.shots(), 1);
    assert_eq!(battle.combat().status(), Status::Won(Side::Enemy));
    game.finish_battle(&battle).expect("the fight settles");
    let left = game
        .copy_condition(pistol)
        .map(|p| p.get())
        .expect("it wears");
    assert!(left < 100, "one shot should show: {left}");
}

#[test]
fn a_worn_weapon_hits_for_less_and_the_wear_comes_into_the_next_fight() {
    let mut game = dressed_game();
    let pistol = wielded(&game);
    let fresh = {
        let fight = start(&game);
        let you = fight
            .combat()
            .combatant(fight.player())
            .expect("the player");
        you.attack().condition.get()
    };
    assert_eq!(fresh, 1.0);

    for _ in 0..40 {
        let mut battle = start(&game);
        miss_and_lose(&mut battle, 20);
        game.finish_battle(&battle).expect("the fight settles");
        game.heal(1_000_000);
    }
    let left = game
        .copy_condition(pistol)
        .map(|p| p.get())
        .expect("it wears");
    assert!(left < 100);
    let worn = start(&game);
    let you = worn.combat().combatant(worn.player()).expect("the player");
    assert!(you.attack().condition.get() < 1.0);
    assert!(you.attack().condition.get() >= 0.5);
}

#[test]
fn only_a_weapon_that_wears_has_a_condition() {
    let game = armed_game();
    assert_eq!(game.copy_condition(CopyId::from_saved(u32::MAX)), None);
}

/// The weapon in hand.
fn wielded(game: &Game) -> CopyId {
    game.inventory().wielded().expect("a weapon is in hand")
}

#[test]
fn only_the_weapon_that_was_fought_with_wears() {
    let mut game = dressed_game();
    InventorySink::give_item(&mut game, &item("9mm_pistol"), NonZeroU32::MIN)
        .expect("a spare is given");
    let spare = game
        .inventory()
        .copies_of(&item("9mm_pistol"))
        .find(|copy| Some(*copy) != game.inventory().wielded())
        .expect("a spare pistol");
    let mut battle = start(&game);
    miss_and_lose(&mut battle, 1);
    game.finish_battle(&battle).expect("the fight settles");
    assert_eq!(game.copy_condition(spare).map(|p| p.get()), Some(100));
    assert!(
        game.copy_condition(wielded(&game))
            .is_some_and(|left| left.get() < 100)
    );
}

#[test]
fn running_away_leaves_the_fight_unwon_with_the_wounds_and_the_wear() {
    let mut game = dressed_game();
    let mut battle = start(&game);
    let target = battle
        .combat()
        .ids()
        .find(|&id| battle.combat().side(id) == Some(Side::Enemy))
        .expect("a foe");
    battle
        .attack(target, Power::Normal, &mut Always(100))
        .expect("a shot is fired");
    // The lowest roll beats any chance the game allows, which is never below 5%.
    let (flight, enemy_turns) = battle.flee(&mut Always(1)).expect("the run is tried");
    assert_eq!(flight, Flight::Escaped);
    assert!(
        enemy_turns.is_empty(),
        "nobody shoots at someone who got away"
    );
    assert_eq!(battle.combat().status(), Status::Fled);

    let hp_in_fight = battle.combat().hp(battle.player()).expect("the player");
    assert_eq!(game.finish_battle(&battle), Ok(Outcome::Fled));
    assert_eq!(game.hp(), hp_in_fight, "the wounds stay");
    assert!(
        game.encounters_at(&place())
            .iter()
            .any(|encounter| encounter.id == gunfight()),
        "the fight is still on offer"
    );
    assert!(
        game.copy_condition(wielded(&game))
            .is_some_and(|left| left.get() < 100),
        "the shot wore the weapon"
    );
}

#[test]
fn a_failed_run_hands_the_turn_to_the_foes_and_the_fight_goes_on() {
    let game = dressed_game();
    let mut battle = start(&game);
    // The highest roll fails any chance below 100%.
    let (flight, enemy_turns) = battle.flee(&mut Always(100)).expect("the run is tried");
    assert_eq!(flight, Flight::Blocked);
    assert!(!enemy_turns.is_empty(), "the foes shoot");
    assert_eq!(battle.combat().status(), Status::Ongoing);
    assert_eq!(
        battle.combat().active(),
        battle.player(),
        "it is your turn again"
    );
}

#[test]
fn the_player_has_a_sequence_of_twice_perception_and_a_flee_chance_from_agility() {
    use engine::shared::{CharacterView, Special};
    let game = dressed_game();
    let battle = start(&game);
    let you = battle.combat().combatant(battle.player()).expect("you");
    let perception = u32::from(game.character().special(Special::Perception).get());
    let agility = u32::from(game.character().special(Special::Agility).get());
    assert_eq!(you.sequence(), 2 * perception);
    assert_eq!(
        u32::from(you.flee_chance().get()),
        (25 + 6 * agility).clamp(5, 95)
    );
}

#[test]
fn a_foe_that_acts_first_shoots_before_the_player_gets_a_turn() {
    // Whoever has the higher sequence opens the fight, so `open` plays the foes
    // that come first and then stops at the player.
    let game = dressed_game();
    let mut battle = game.start_battle(&gunfight()).expect("the fight starts");
    let opening = battle.open(&mut Always(100));
    assert_eq!(battle.combat().active(), battle.player());
    let first_is_foe = battle.combat().round() == 1 && !opening.is_empty();
    let you = battle
        .combat()
        .combatant(battle.player())
        .expect("you")
        .sequence();
    let fastest_foe = battle
        .combat()
        .ids()
        .filter(|&id| battle.combat().side(id) == Some(Side::Enemy))
        .filter_map(|id| battle.combat().combatant(id))
        .map(|foe| foe.sequence())
        .max()
        .expect("a foe");
    assert_eq!(first_is_foe, fastest_foe > you);
}

#[test]
fn every_fighter_can_pay_for_a_reload_inside_one_turn() {
    let game = dressed_game();
    let battle = start(&game);
    let combat = battle.combat();
    for id in combat.ids() {
        let fighter = combat.combatant(id).expect("a fighter");
        if let Some(magazine) = fighter.magazine() {
            assert!(
                magazine.reload_ap.get() <= fighter.max_ap(),
                "{} could never reload",
                fighter.name()
            );
        }
    }
}

#[test]
fn the_players_chance_in_the_real_fight_comes_from_skill_and_the_pistols_spread() {
    use engine::combat::{AimedHitChance, HitModel};
    let game = dressed_game();
    let battle = start(&game);
    let combat = battle.combat();
    let you = combat.combatant(battle.player()).expect("the player");
    let foe = combat
        .combatant(combat.ids().nth(1).expect("a foe"))
        .expect("a foe");
    // Guns 15 and a 0.7 spread: 40 + 15 / 2 - 0.7 * 3 = 45.4.
    assert_eq!(AimedHitChance.chance(you, foe).get(), 45);
}

// Perks and traits that change the numbers a fight and a heal read.

/// Five in everything but intelligence, which takes the five spare points, with the
/// given traits.
fn trained(traits: &[&str]) -> Character {
    let special = SpecialSet::default().with(
        Special::Intelligence,
        SpecialLevel::try_new(10).expect("literal is a legal level"),
    );
    let traits: Vec<TraitId> = traits
        .iter()
        .map(|name| TraitId::try_new(*name).expect("literal is a valid id"))
        .collect();
    Character::create(
        special,
        &[Skill::Guns, Skill::Speech, Skill::Sneak],
        &traits,
        CharacterRules::standard(),
        &FIXTURE.perks,
    )
    .expect("a legal character")
}

/// The player's fighter at the start of the gunfight.
fn you_in(game: &Game) -> engine::game::Fighter {
    let battle = start(game);
    battle
        .combat()
        .combatant(battle.player())
        .expect("the player")
        .clone()
}

#[test]
fn kamikaze_adds_action_points_and_takes_threshold_and_resistance_away() {
    let plain = you_in(&dressed_game().with_character(trained(&[])));
    let reckless = you_in(&dressed_game().with_character(trained(&["kamikaze"])));
    assert_eq!(reckless.max_ap(), plain.max_ap() + 10);
    assert_eq!(reckless.dt().get(), (plain.dt().get() - 2.0).max(0.0));
    assert_eq!(reckless.dr().get(), (plain.dr().get() - 15.0).max(0.0));
}

#[test]
fn fast_shot_makes_a_pistol_shot_cheaper_and_less_steady() {
    let plain = you_in(&dressed_game().with_character(trained(&[])));
    let quick = you_in(&dressed_game().with_character(trained(&["fast_shot"])));
    let cost = |fighter: &engine::game::Fighter| fighter.attack().ap_cost.get();
    assert!(
        cost(&quick) < cost(&plain),
        "{} < {}",
        cost(&quick),
        cost(&plain)
    );
    assert!(quick.aim().spread.get() > plain.aim().spread.get());
}

#[test]
fn trigger_discipline_does_the_opposite() {
    let plain = you_in(&dressed_game().with_character(trained(&[])));
    let steady = you_in(&dressed_game().with_character(trained(&["trigger_discipline"])));
    assert!(steady.attack().ap_cost.get() > plain.attack().ap_cost.get());
    assert!(steady.aim().spread.get() < plain.aim().spread.get());
}

#[test]
fn heavy_handed_leaves_a_pistol_alone() {
    let plain = you_in(&dressed_game().with_character(trained(&[])));
    let heavy = you_in(&dressed_game().with_character(trained(&["heavy_handed"])));
    assert_eq!(heavy.attack(), plain.attack());
}

#[test]
fn built_to_destroy_adds_critical_chance_and_wears_the_weapon_faster() {
    let plain = you_in(&dressed_game().with_character(trained(&[])));
    let burning = you_in(&dressed_game().with_character(trained(&["built_to_destroy"])));
    assert_eq!(
        burning.crit_chance().get(),
        plain.crit_chance().get() + 3,
        "three points of critical chance"
    );
    let left_after = |character: Character| {
        let mut game = dressed_game().with_character(character);
        let mut battle = start(&game);
        miss_and_lose(&mut battle, 20);
        game.finish_battle(&battle).expect("the fight settles");
        game.copy_condition(wielded(&game)).expect("it wears").get()
    };
    assert!(left_after(trained(&["built_to_destroy"])) < left_after(trained(&[])));
}

/// A character that has reached level 12 and taken `perk`.
fn with_perk(perk: &str) -> Character {
    let mut character = trained(&[]);
    let rules = CharacterRules::standard();
    let level = Level::try_new(12).expect("a legal level");
    character.grant_xp(rules.xp_for_level(level));
    character
        .choose_perk(
            &FIXTURE.perks,
            &PerkId::try_new(perk).expect("literal is a valid id"),
        )
        .expect("the perk can be taken");
    character
}

/// What one stimpak restores for a player with this character and 100 wounds.
fn stimpak_heals(character: Character) -> u32 {
    let mut game = armed_game().with_character(character).with_wounds(100);
    InventorySink::give_item(&mut game, &item("stimpak"), NonZeroU32::MIN)
        .expect("a stimpak is given");
    game.use_item(&item("stimpak")).expect("the stimpak works")
}

#[test]
fn a_stimpak_heals_more_the_higher_the_medicine_and_more_again_with_fast_metabolism() {
    let plain = trained(&[]);
    let medicine = u32::from(plain.skill(Skill::Medicine).get());
    assert_eq!(
        stimpak_heals(plain),
        (30 * (100 + medicine / 2) + 50) / 100,
        "one percent for every two points of Medicine"
    );
    let metabolic = with_perk("fast_metabolism");
    let medicine = u32::from(metabolic.skill(Skill::Medicine).get());
    assert_eq!(
        stimpak_heals(metabolic),
        (30 * (120 + medicine / 2) + 50) / 100,
        "and twenty percent more with the perk"
    );
}

/// A level 12 character with the Science the perk asks for, who has taken it.
fn nerd_raged() -> Character {
    let mut character = trained(&[]);
    let rules = CharacterRules::standard();
    character.grant_xp(rules.xp_for_level(Level::try_new(12).expect("a legal level")));
    while character.skill(Skill::Science).get() < 50 {
        character
            .spend_skill_point(Skill::Science)
            .expect("the level-up paid enough points");
    }
    character
        .choose_perk(
            &FIXTURE.perks,
            &PerkId::try_new("nerd_rage").expect("literal is a valid id"),
        )
        .expect("the perk can be taken");
    character
}

/// One hit in seven rolls and no critical, so a round wounds the player without
/// finishing them.
struct Sparse(u32);

impl Dice for Sparse {
    fn percent(&mut self) -> u8 {
        self.0 += 1;
        if self.0 % 7 == 1 { 1 } else { 100 }
    }
}

/// The player's fighter in `battle`.
fn you(battle: &Battle) -> engine::game::Fighter {
    battle
        .combat()
        .combatant(battle.player())
        .expect("the player")
        .clone()
}

#[test]
fn nerd_rage_switches_on_in_the_fight_the_moment_health_falls_to_a_fifth() {
    let character = nerd_raged();
    // A quarter of their hit points left: above the perk's 20% line when it opens.
    let wounds = character.max_hp() - character.max_hp() / 4;
    let mut game = dressed_game().with_character(character).with_wounds(wounds);
    let mut battle = start(&game);
    let calm = you(&battle).dt().get();
    let max = you(&battle).max_hp().get();
    let mut raged = false;
    for _ in 0..12 {
        if battle.combat().status() != Status::Ongoing {
            break;
        }
        let Ok(_) = battle.end_turn(&mut Sparse(0)) else {
            break;
        };
        let hp = battle.combat().hp(battle.player()).unwrap_or(0);
        let low = engine::shared::HealthPercent::of(hp, max)
            <= engine::shared::HealthPercent::of(20, 100);
        let expected = if low { calm + 15.0 } else { calm };
        assert_eq!(you(&battle).dt().get(), expected, "at {hp} of {max}");
        raged |= low && battle.combat().status() == Status::Ongoing;
    }
    assert!(
        raged,
        "the fight reached the low-health line with the player up"
    );
    game.finish_battle(&battle).ok();
}

#[test]
fn a_chem_that_raises_damage_threshold_and_damage_reaches_the_fight() {
    let mut game = dressed_game();
    for name in ["datura_hide", "psycho"] {
        InventorySink::give_item(&mut game, &item(name), NonZeroU32::MIN)
            .expect("the chem is given");
    }
    let plain = you_in(&game);
    game.use_item(&item("datura_hide")).expect("the hide works");
    game.use_item(&item("psycho")).expect("psycho works");
    let boosted = you_in(&game);
    assert_eq!(boosted.dt().get(), plain.dt().get() + 1.0);
    assert_eq!(
        boosted.attack().base_damage.get(),
        plain.attack().base_damage.get() * 1.25
    );
}
