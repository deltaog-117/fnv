//! Radiation and addiction through `Game`: irradiated food, RadAway, Rad-X, chems
//! that hook the player, withdrawal, and a doctor's cures.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::addiction::{AddictionBook, WITHDRAWAL_MINUTES};
use engine::clock::Span;
use engine::combat::Dice;
use engine::game::Game;
use engine::inventory::{InventoryError, ItemBook, ItemClass};
use engine::quests::QuestBook;
use engine::shared::{
    Ailment, CharacterView, Condition, Effect, InventorySink, InventoryView, ItemId, ItemUse,
    Quest, Rads, Special, apply, evaluate,
};
use proptest::prelude::*;
use serde_json::json;

fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("literal is not zero")
}

fn minutes(n: u32) -> Span {
    Span::minutes(count(n))
}

fn used(value: serde_json::Value) -> ItemUse {
    serde_json::from_value(value).expect("the literal is a valid use")
}

/// Rolls the numbers it was given, and counts how many were asked for.
struct Loaded {
    rolls: Vec<u8>,
    asked: usize,
}

impl Loaded {
    fn new(rolls: &[u8]) -> Self {
        Self {
            rolls: rolls.to_vec(),
            asked: 0,
        }
    }
}

impl Dice for Loaded {
    fn percent(&mut self) -> u8 {
        let roll = self.rolls.get(self.asked).copied().unwrap_or(100);
        self.asked += 1;
        roll
    }
}

/// Dirty water (6 rads), RadAway (-50), Rad-X (+25 resistance for 4 minutes) and
/// Jet (a 20% addiction, +15 action points for 4 minutes). The default character
/// has Endurance 5, so 8% resistance of its own.
fn game() -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let uses = [
        ("dirty_water", json!({"boost": {"rads": 6}})),
        ("radaway", json!({"boost": {"rads": -50}})),
        ("big_dose", json!({"boost": {"rads": 300}})),
        (
            "rad_x",
            json!({"boost": {"effects": [{"stat": "radiation_resistance", "amount": 25, "minutes": 4}]}}),
        ),
        (
            "jet",
            json!({"boost": {"effects": [{"stat": "action_points", "amount": 15, "minutes": 4}]}}),
        ),
    ];
    let items = ItemBook::new(
        uses.iter()
            .map(|(name, _)| (item(name), ItemClass::Consumable, 0.0)),
    )
    .and_then(|book| {
        book.with_uses(
            uses.iter()
                .map(|(name, value)| (item(name), used(value.clone()))),
        )
    })
    .expect("the literal entries are valid");
    let jet = item("jet");
    let addictions = AddictionBook::new([(&jet, "jet", 20.0)]).expect("the literal builds");
    let mut game = Game::new(Arc::new(quests))
        .with_items(Arc::new(items))
        .with_addictions(Arc::new(addictions));
    for name in ["dirty_water", "radaway", "big_dose", "rad_x", "jet"] {
        game.give_item(&item(name), count(10)).expect("in the book");
    }
    game
}

#[test]
fn irradiated_food_adds_rads_after_the_players_own_resistance() {
    let mut game = game();
    assert_eq!(game.radiation_resistance(), 8, "2 * (5 - 1)");
    let used = game
        .use_item_rolling(&item("big_dose"), &mut Loaded::new(&[]))
        .expect("used");
    // 300 rads with 8% kept out is 276, which is one tier of sickness.
    assert_eq!(used.rads_in, 276);
    assert_eq!(game.rads().get(), 276);
    assert_eq!(game.radiation_tier(), 1);
}

#[test]
fn rad_x_keeps_more_out_until_it_wears_off() {
    let mut game = game();
    game.use_item(&item("rad_x")).expect("used");
    assert_eq!(game.radiation_resistance(), 33);
    game.use_item(&item("dirty_water")).expect("used");
    assert_eq!(game.rads().get(), 4, "6 rads, a third kept out, rounded");
    game.wait(minutes(4)).expect("time passes");
    assert_eq!(game.radiation_resistance(), 8);
}

#[test]
fn radaway_takes_rads_out_and_is_kept_when_there_are_none() {
    let mut game = game();
    assert_eq!(
        game.use_item(&item("radaway")),
        Err(InventoryError::NotIrradiated)
    );
    assert_eq!(
        game.inventory().item_count(&item("radaway")),
        10,
        "not spent"
    );
    game.use_item(&item("big_dose")).expect("used");
    let used = game
        .use_item_rolling(&item("radaway"), &mut Loaded::new(&[]))
        .expect("used");
    assert_eq!(used.rads_out, 50);
    assert_eq!(game.rads().get(), 276 - 50);
    assert_eq!(game.inventory().item_count(&item("radaway")), 9);
}

#[test]
fn sickness_costs_endurance_and_agility_and_a_cure_gives_them_back() {
    let mut game = game();
    let (endurance, agility, hp) = (
        game.character().special(Special::Endurance).get(),
        game.character().special(Special::Agility).get(),
        game.character().max_hp(),
    );
    game.use_item(&item("big_dose")).expect("used");
    assert_eq!(
        game.character().special(Special::Endurance).get(),
        endurance - 1
    );
    assert_eq!(
        game.character().special(Special::Agility).get(),
        agility - 1
    );
    assert!(game.character().max_hp() < hp);
    assert!(game.is_alive());
    apply(
        &Effect::Cure {
            ailment: Ailment::Radiation,
        },
        &mut game,
    )
    .expect("cured");
    assert_eq!(game.rads(), Rads::NONE);
    assert_eq!(
        game.character().special(Special::Endurance).get(),
        endurance
    );
    assert_eq!(game.character().max_hp(), hp);
}

#[test]
fn waiting_does_not_clear_radiation() {
    let mut game = game();
    game.use_item(&item("big_dose")).expect("used");
    game.wait(minutes(60 * 24 * 7)).expect("time passes");
    assert_eq!(game.rads().get(), 276);
}

#[test]
fn a_roll_at_or_below_the_chance_hooks_the_player() {
    let mut hooked = game();
    let used = hooked
        .use_item_rolling(&item("jet"), &mut Loaded::new(&[20]))
        .expect("used");
    assert_eq!(used.addicted.as_ref().map(|k| k.as_str()), Some("jet"));
    assert!(hooked.addictions().any() && evaluate(&Condition::Addicted {}, &hooked));

    let mut lucky = game();
    let used = lucky
        .use_item_rolling(&item("jet"), &mut Loaded::new(&[21]))
        .expect("used");
    assert_eq!(used.addicted, None);
    assert!(!lucky.addictions().any() && !evaluate(&Condition::Addicted {}, &lucky));
}

#[test]
fn a_chem_used_without_dice_never_hooks_the_player() {
    let mut game = game();
    for _ in 0..5 {
        game.use_item(&item("jet")).expect("used");
    }
    assert!(!game.addictions().any());
}

#[test]
fn an_addicted_player_is_not_rolled_for_again_and_the_last_dose_moves() {
    let mut game = game();
    let mut dice = Loaded::new(&[1]);
    game.use_item_rolling(&item("jet"), &mut dice)
        .expect("used");
    game.wait(minutes(100)).expect("time passes");
    let used = game
        .use_item_rolling(&item("jet"), &mut dice)
        .expect("used");
    assert_eq!(dice.asked, 1, "no second roll");
    assert_eq!(used.addicted, None);
    let (_, last) = game.addictions().iter().next().expect("still addicted");
    assert_eq!(last, game.clock().as_minutes());
}

#[test]
fn withdrawal_costs_strength_and_agility_until_it_wears_off_or_is_cured() {
    let mut game = game();
    let (strength, agility) = (
        game.character().special(Special::Strength).get(),
        game.character().special(Special::Agility).get(),
    );
    game.use_item_rolling(&item("jet"), &mut Loaded::new(&[1]))
        .expect("used");
    assert_eq!(
        game.character().special(Special::Strength).get(),
        strength - 1
    );
    assert_eq!(
        game.character().special(Special::Agility).get(),
        agility - 1
    );

    let mut cured = game.clone();
    apply(
        &Effect::Cure {
            ailment: Ailment::Addiction,
        },
        &mut cured,
    )
    .expect("cured");
    assert!(!cured.addictions().any());
    assert_eq!(cured.character().special(Special::Strength).get(), strength);

    let lasts = u32::try_from(WITHDRAWAL_MINUTES).expect("fits");
    game.wait(minutes(lasts - 1)).expect("time passes");
    assert!(game.addictions().any());
    game.wait(minutes(1)).expect("time passes");
    assert!(!game.addictions().any());
    assert_eq!(game.character().special(Special::Strength).get(), strength);
}

#[test]
fn radiation_conditions_read_the_rads_held() {
    let mut game = game();
    let at_least = |min| Condition::Radiation {
        min: Rads::try_new(min).expect("in range"),
    };
    assert!(evaluate(&at_least(0), &game));
    assert!(!evaluate(&at_least(1), &game));
    game.use_item(&item("big_dose")).expect("used");
    assert!(evaluate(&at_least(276), &game));
    assert!(!evaluate(&at_least(277), &game));
}

#[test]
fn a_save_keeps_the_rads_and_the_addictions() {
    let template = game();
    let mut played = template.clone();
    played.use_item(&item("big_dose")).expect("used");
    played
        .use_item_rolling(&item("jet"), &mut Loaded::new(&[1]))
        .expect("used");
    played.wait(minutes(30)).expect("time passes");
    let save = played.save();
    let loaded = template.restore(&save).expect("the save loads");
    assert_eq!(loaded.rads(), played.rads());
    assert_eq!(loaded.addictions(), played.addictions());
    assert_eq!(loaded.character().max_hp(), played.character().max_hp());
    assert_eq!(loaded.save(), save, "saving it again changes nothing");
}

#[test]
fn a_save_with_an_unknown_or_future_addiction_is_refused() {
    let template = game();
    let mut played = template.clone();
    played
        .use_item_rolling(&item("jet"), &mut Loaded::new(&[1]))
        .expect("used");
    let mut unknown = played.save();
    unknown.addictions[0].kind = "nothing".to_owned();
    assert!(template.restore(&unknown).is_err());
    let mut future = played.save();
    future.addictions[0].last_dose = future.clock_minutes + 1;
    assert!(template.restore(&future).is_err());
    let mut twice = played.save();
    twice.addictions.push(twice.addictions[0].clone());
    assert!(template.restore(&twice).is_err());
    let mut raw = played.save();
    raw.rads = 1000;
    assert!(template.restore(&raw).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// However the player eats, drinks, doses and waits, they are never killed by
    /// the radiation or the withdrawal, the rads stay in range, and once all the
    /// effects are over the numbers return exactly.
    #[test]
    fn no_mix_of_doses_and_time_kills_the_player_or_breaks_the_numbers(
        steps in prop::collection::vec((0u8..6, 1u8..=100, 1u32..2_000), 1..24),
    ) {
        let mut game = game();
        for (what, roll, wait) in steps {
            let name = ["dirty_water", "radaway", "big_dose", "rad_x", "jet", "jet"][usize::from(what)];
            let _ = game.use_item_rolling(&item(name), &mut Loaded::new(&[roll]));
            let _ = game.wait(minutes(wait));
            prop_assert!(game.is_alive(), "something killed the player");
            prop_assert!(game.rads().get() <= Rads::MAX);
            prop_assert!(game.hp() <= game.character().max_hp());
        }
        apply(&Effect::Cure { ailment: Ailment::Radiation }, &mut game).expect("cured");
        apply(&Effect::Cure { ailment: Ailment::Addiction }, &mut game).expect("cured");
        let lasts = u32::try_from(WITHDRAWAL_MINUTES).expect("fits") + 10;
        game.wait(minutes(lasts)).expect("time passes");
        prop_assert_eq!(game.character().special(Special::Endurance).get(), 5);
        prop_assert_eq!(game.character().special(Special::Strength).get(), 5);
    }
}
