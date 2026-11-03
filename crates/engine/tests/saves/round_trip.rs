//! Saving and loading give back the same game.

use engine::clock::Span;
use engine::game::Game;
use engine::saves::Save;
use engine::shared::{CharacterView, InventoryView, Skill, apply_all};
use proptest::prelude::*;
use strum::IntoEnumIterator;

use super::support::{count, effect, fight_to_the_end, item, played, stage, template, vendor};

/// Saves, writes to text, reads it back and loads it into a fresh game.
fn through_text(game: &Game) -> Game {
    let text = game.save().to_json().expect("a save writes");
    let save = Save::from_json(&text).expect("a written save reads");
    template().restore(&save).expect("a written save loads")
}

#[test]
fn a_new_game_loads_back_as_itself() {
    let game = template();
    assert_eq!(through_text(&game).save(), game.save());
}

#[test]
fn a_played_game_loads_back_as_itself() {
    let game = played();
    let loaded = through_text(&game);
    assert_eq!(loaded.save(), game.save());
    assert_eq!(loaded.hp(), game.hp());
    assert_eq!(loaded.clock(), game.clock());
    assert_eq!(loaded.at(), game.at());
    assert_eq!(loaded.inventory().caps(), game.inventory().caps());
    assert_eq!(loaded.character().xp(), game.character().xp());
    assert_eq!(
        loaded.character().skill(Skill::Guns),
        game.character().skill(Skill::Guns)
    );
    assert_eq!(
        loaded.active_effects().count(),
        game.active_effects().count()
    );
}

#[test]
fn the_played_game_really_has_a_history_worth_saving() {
    let game = played();
    let save = game.save();
    assert!(!save.quests.is_empty());
    assert!(!save.world.npcs.is_empty());
    assert!(!save.wear.is_empty(), "the pistol wore");
    assert!(!save.won.is_empty(), "the gunfight was won");
    assert_eq!(save.gathered.len(), 2, "both plants were picked");
    assert!(!save.effects.is_empty(), "buffout is running");
    assert!(!save.standing.factions.is_empty());
    assert!(save.wounds > 0 || save.clock_minutes > 0);
    assert!(save.character.xp > 0);
}

#[test]
fn saving_twice_gives_the_same_text() {
    let game = played();
    assert_eq!(
        game.save().to_json().expect("writes"),
        game.save().to_json().expect("writes")
    );
}

#[test]
fn loading_pays_no_reward_a_second_time() {
    let game = played();
    let loaded = through_text(&game);
    assert_eq!(loaded.inventory().caps(), game.inventory().caps());
    assert_eq!(loaded.character().xp(), game.character().xp());
    assert_eq!(loaded.deferred().len(), game.deferred().len());
}

#[test]
fn a_loaded_game_goes_on_exactly_like_the_original() {
    let mut original = played();
    let mut loaded = through_text(&original);
    for game in [&mut original, &mut loaded] {
        game.wait(Span::minutes(count(90))).expect("time passes");
        game.heal(1_000_000);
        game.sell(&vendor("chet"), &item("stimpak"), count(1)).ok();
        apply_all(
            &[effect(serde_json::json!({"type": "xp", "amount": 700}))],
            game,
        )
        .expect("experience is granted");
    }
    assert_eq!(loaded.save(), original.save());
}

#[test]
fn a_worn_weapon_stays_worn_after_loading() {
    let game = played();
    let pistol = item("9mm_pistol");
    let before = game.weapon_condition(&pistol);
    assert!(before.is_some_and(|left| left.get() < 100));
    assert_eq!(through_text(&game).weapon_condition(&pistol), before);
}

#[test]
fn loading_into_a_game_leaves_that_game_alone() {
    let template = template();
    let before = template.save();
    template.restore(&played().save()).expect("the save loads");
    assert_eq!(template.save(), before);
}

#[test]
fn loading_replaces_the_state_the_template_had() {
    let mut busy = template();
    apply_all(
        &[
            stage("ghost_town_gunfight", 5),
            effect(serde_json::json!({"type": "caps", "amount": 999})),
        ],
        &mut busy,
    )
    .expect("the effects apply");
    let loaded = busy.restore(&template().save()).expect("the save loads");
    assert_eq!(loaded.save(), template().save());
}

/// One thing the player can do; the ones that do not apply are refused and ignored,
/// as the front end would refuse them.
#[derive(Debug, Clone)]
enum Op {
    Wait(u32),
    Caps(i32),
    Stage(u8),
    Flag(u8),
    Xp(u32),
    Karma(i32),
    Reputation(u32),
    Buy(u32),
    Sell(u32),
    Use(u8),
    Drop(u8),
    Equip(u8),
    Sleep(u32),
    Travel(u8),
    Spend(u8),
    Perk(u8),
    Fight,
    Heal,
    Gather(u8),
    Craft,
    Kit,
    Merge,
    RepairAtChet,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (1..600u32).prop_map(Op::Wait),
        (-300..600i32).prop_map(Op::Caps),
        (0..8u8).prop_map(Op::Stage),
        (0..4u8).prop_map(Op::Flag),
        (0..2_000u32).prop_map(Op::Xp),
        (-400..400i32).prop_map(Op::Karma),
        (0..120u32).prop_map(Op::Reputation),
        (1..6u32).prop_map(Op::Buy),
        (1..4u32).prop_map(Op::Sell),
        (0..3u8).prop_map(Op::Use),
        (0..3u8).prop_map(Op::Drop),
        (0..3u8).prop_map(Op::Equip),
        (1..9u32).prop_map(Op::Sleep),
        (0..3u8).prop_map(Op::Travel),
        (0..13u8).prop_map(Op::Spend),
        (0..6u8).prop_map(Op::Perk),
        Just(Op::Fight),
        Just(Op::Heal),
        (0..2u8).prop_map(Op::Gather),
        Just(Op::Craft),
        Just(Op::Kit),
        Just(Op::Merge),
        Just(Op::RepairAtChet),
    ]
}

const STAGES: [u32; 8] = [5, 10, 15, 20, 60, 65, 70, 40];
const FLAGS: [&str; 4] = [
    "trudy_joined_militia",
    "easy_pete_joined_militia",
    "chet_supplied_militia",
    "doc_mitchell_gave_cobb_supplies",
];
const ITEMS: [&str; 3] = ["stimpak", "buffout", "9mm_pistol"];

fn play(game: &mut Game, ops: &[Op]) {
    for step in ops {
        match step {
            Op::Wait(minutes) => {
                game.wait(Span::minutes(count(*minutes))).ok();
            }
            Op::Caps(amount) => {
                apply_all(
                    &[effect(
                        serde_json::json!({"type": "caps", "amount": f64::from(*amount)}),
                    )],
                    game,
                )
                .ok();
            }
            Op::Stage(i) => {
                apply_all(
                    &[stage("ghost_town_gunfight", STAGES[usize::from(*i)])],
                    game,
                )
                .ok();
            }
            Op::Flag(i) => {
                apply_all(
                    &[effect(
                        serde_json::json!({"type": "set_flag", "flag": FLAGS[usize::from(*i)]}),
                    )],
                    game,
                )
                .ok();
            }
            Op::Xp(amount) => {
                apply_all(
                    &[effect(
                        serde_json::json!({"type": "xp", "amount": f64::from(*amount)}),
                    )],
                    game,
                )
                .ok();
            }
            Op::Karma(amount) => {
                apply_all(
                    &[effect(
                        serde_json::json!({"type": "karma", "amount": f64::from(*amount)}),
                    )],
                    game,
                )
                .ok();
            }
            Op::Reputation(points) => {
                apply_all(
                    &[effect(serde_json::json!({
                        "type": "reputation", "faction": "goodsprings",
                        "fame": f64::from(*points), "infamy": f64::from(*points / 2),
                    }))],
                    game,
                )
                .ok();
            }
            Op::Buy(n) => {
                game.buy(&vendor("chet"), &item("9mm_round"), count(*n))
                    .ok();
            }
            Op::Sell(n) => {
                game.sell(&vendor("chet"), &item("stimpak"), count(*n)).ok();
            }
            Op::Use(i) => {
                game.use_item(&item(ITEMS[usize::from(*i)])).ok();
            }
            Op::Drop(i) => {
                game.drop_item(&item(ITEMS[usize::from(*i)]), count(1)).ok();
            }
            Op::Equip(i) => {
                game.equip(&item(ITEMS[usize::from(*i)])).ok();
            }
            Op::Sleep(hours) => {
                let bed = game
                    .beds_at(
                        &engine::shared::LocationId::try_new("doc_mitchell_s_house").expect("id"),
                    )
                    .next()
                    .map(|(id, _)| id.clone());
                if let Some(bed) = bed {
                    game.sleep(&bed, Span::hours(count(*hours))).ok();
                }
            }
            Op::Travel(i) => {
                let routes = game.routes();
                if let Some(route) = routes.get(usize::from(*i) % routes.len().max(1)) {
                    game.travel(&route.to.clone()).ok();
                }
            }
            Op::Spend(i) => {
                let skill = Skill::iter().nth(usize::from(*i));
                if let Some(skill) = skill {
                    game.spend_skill_point(skill).ok();
                }
            }
            Op::Perk(i) => {
                let open: Vec<_> = game
                    .perks()
                    .open_to(game.character())
                    .into_iter()
                    .map(|perk| perk.id.clone())
                    .collect();
                if let Some(perk) = open.get(usize::from(*i) % open.len().max(1)) {
                    game.choose_perk(perk).ok();
                }
            }
            Op::Fight => {
                let id = engine::shared::EncounterId::try_new("ghost_town_gunfight").expect("id");
                if let Ok(mut battle) = game.start_battle(&id) {
                    fight_to_the_end(&mut battle);
                    game.finish_battle(&battle).ok();
                }
            }
            Op::Heal => game.heal(1_000_000),
            Op::Gather(i) => {
                let find = ["goodsprings_broc_flower", "goodsprings_xander_root"][usize::from(*i)];
                game.gather(&engine::shared::FindId::try_new(find).expect("id"))
                    .ok();
            }
            Op::Craft => {
                let campfire =
                    engine::shared::StationId::try_new("goodsprings_campfire").expect("id");
                let powder = engine::shared::RecipeId::try_new("healing_powder_craft").expect("id");
                game.craft(&campfire, &powder).ok();
            }
            Op::Kit => {
                apply_all(
                    &[effect(
                        serde_json::json!({"type": "give_item", "item": "weapon_repair_kit"}),
                    )],
                    game,
                )
                .ok();
                game.repair_with_kit(&item("9mm_pistol")).ok();
            }
            Op::Merge => {
                game.merge_weapons(&item("9mm_pistol")).ok();
            }
            Op::RepairAtChet => {
                game.repair_at(&vendor("chet"), &item("9mm_pistol")).ok();
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn any_game_loads_back_as_itself_and_goes_on_the_same(
        before in prop::collection::vec(op(), 0..14),
        after in prop::collection::vec(op(), 0..14),
    ) {
        let mut game = template();
        play(&mut game, &before);

        let save = game.save();
        let text = save.to_json().expect("a save writes");
        let read = Save::from_json(&text).expect("a written save reads");
        prop_assert_eq!(&read, &save);

        let mut loaded = template().restore(&read).expect("a save from a real game loads");
        prop_assert_eq!(loaded.save(), save);

        play(&mut game, &after);
        play(&mut loaded, &after);
        prop_assert_eq!(loaded.save(), game.save());
    }
}
