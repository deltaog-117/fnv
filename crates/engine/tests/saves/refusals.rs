//! A save that cannot be real in this game is refused, by part, and the game that
//! was to receive it is left alone.

use engine::saves::{
    Save, SaveError, SavedDeferred, SavedEffect, SavedFired, SavedNpc, SavedPhase, SavedQuest,
    SavedShop, SavedWeapon,
};
use engine::shared::{
    BoostStat, Effect, EncounterId, FindId, LocationId, NpcId, PerkId, QuestId, QuestStage, Skill,
    Special, TraitId,
};

use super::support::{item, played, template, vendor};

fn quest(text: &str) -> QuestId {
    QuestId::try_new(text).expect("literal is a valid id")
}

/// Loads a tampered copy of a real save and returns the part named in the refusal.
fn refused_part(tamper: impl FnOnce(&mut Save)) -> &'static str {
    let mut save = played().save();
    tamper(&mut save);
    let before = template().save();
    let game = template();
    match game.restore(&save) {
        Err(SaveError::Invalid { part, .. }) => {
            assert_eq!(game.save(), before, "a refusal must leave the game alone");
            part
        }
        Err(other) => panic!("expected an invalid save, got {other}"),
        Ok(_) => panic!("the tampered save was accepted"),
    }
}

#[test]
fn an_untouched_save_is_accepted() {
    assert!(template().restore(&played().save()).is_ok());
}

#[test]
fn a_quest_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| save.quests.push(SavedQuest {
            quest: quest("no_such_quest"),
            status: SavedPhase::Active,
            stage: None,
        })),
        "quests"
    );
}

#[test]
fn a_stage_that_does_not_exist_is_refused() {
    assert_eq!(
        refused_part(|save| save.quests[0].stage = Some(QuestStage::new(9_999))),
        "quests"
    );
}

#[test]
fn a_quest_listed_twice_is_refused() {
    assert_eq!(
        refused_part(|save| {
            let again = save.quests[0].clone();
            save.quests.push(again);
        }),
        "quests"
    );
}

#[test]
fn a_finished_quest_cannot_claim_to_be_still_going() {
    assert_eq!(
        refused_part(|save| {
            let done = save
                .quests
                .iter_mut()
                .find(|entry| entry.status == SavedPhase::Completed && entry.stage.is_some())
                .expect("the played game finished a quest at a stage");
            done.status = SavedPhase::Active;
        }),
        "quests"
    );
}

#[test]
fn an_npc_the_world_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| save.world.npcs.push(SavedNpc {
            npc: NpcId::try_new("nobody").expect("literal is a valid id"),
            fate: engine::saves::SavedFate::Dead,
            met: false,
        })),
        "world"
    );
}

#[test]
fn an_item_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.pack.items.insert(item("no_such_item"), 1);
        }),
        "pack"
    );
}

#[test]
fn a_stack_of_nothing_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.pack.items.insert(item("stimpak"), 0);
        }),
        "pack"
    );
}

#[test]
fn gear_worn_in_the_wrong_place_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.pack.weapon = Some(item("leather_armor"));
        }),
        "pack"
    );
}

#[test]
fn gear_that_is_not_carried_cannot_be_worn() {
    assert_eq!(
        refused_part(|save| {
            save.pack.items.remove(&item("leather_armor"));
        }),
        "pack"
    );
}

#[test]
fn a_weapon_in_hand_that_is_not_carried_is_refused() {
    assert_eq!(refused_part(|save| save.pack.weapons.clear()), "pack");
}

#[test]
fn a_vendor_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| save.shops.push(SavedShop {
            vendor: vendor("nobody"),
            caps: 1,
            stock: std::collections::BTreeMap::new(),
        })),
        "shops"
    );
}

#[test]
fn stock_with_no_price_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.shops[0].stock.insert(item("no_such_item"), 3);
        }),
        "shops"
    );
}

#[test]
fn wounds_past_the_maximum_are_refused() {
    assert_eq!(refused_part(|save| save.wounds = 1_000_000), "wounds");
}

#[test]
fn a_boost_that_already_ended_is_refused() {
    assert_eq!(
        refused_part(|save| save.effects.push(SavedEffect {
            source: item("buffout"),
            stat: BoostStat::Special(Special::Strength),
            amount: 2,
            until: 0,
        })),
        "effects"
    );
}

#[test]
fn a_boost_from_an_unknown_item_is_refused() {
    assert_eq!(
        refused_part(|save| save.effects.push(SavedEffect {
            source: item("no_such_item"),
            stat: BoostStat::Special(Special::Strength),
            amount: 2,
            until: u64::MAX,
        })),
        "effects"
    );
}

#[test]
fn a_place_off_the_map_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.at = Some(LocationId::try_new("nowhere_at_all").expect("literal is a valid id"));
        }),
        "place"
    );
}

#[test]
fn a_save_without_a_place_does_not_fit_a_game_with_a_map() {
    assert_eq!(refused_part(|save| save.at = None), "place");
}

#[test]
fn wear_on_something_that_is_not_a_weapon_is_refused() {
    assert_eq!(
        refused_part(|save| save.pack.weapons.push(SavedWeapon {
            id: 9_000,
            item: item("leather_armor"),
            lost: 1,
        })),
        "pack"
    );
}

#[test]
fn wear_past_what_a_weapon_has_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.pack.weapons[0].lost = u32::MAX;
        }),
        "pack"
    );
}

#[test]
fn two_weapons_with_one_number_are_refused() {
    assert_eq!(
        refused_part(|save| {
            let copy = save.pack.weapons[0].clone();
            save.pack.weapons.push(copy);
        }),
        "pack"
    );
}

#[test]
fn naming_the_weapon_in_hand_twice_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.pack.wielded = Some(save.pack.weapons[0].id);
            save.pack.weapon = Some(item("9mm_pistol"));
        }),
        "pack"
    );
}

#[test]
fn a_fight_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| save
            .won
            .push(EncounterId::try_new("no_such_fight").expect("literal is a valid id"))),
        "won"
    );
}

#[test]
fn a_rule_that_does_not_exist_is_refused() {
    assert_eq!(
        refused_part(|save| save.fired.push(SavedFired {
            quest: quest("ghost_town_gunfight"),
            rule: 9_999,
        })),
        "fired"
    );
}

#[test]
fn a_postponed_effect_for_an_unknown_feature_is_refused() {
    assert_eq!(
        refused_part(|save| save.deferred.push(SavedDeferred {
            quest: quest("ghost_town_gunfight"),
            effect: Effect::Heal {},
            feature: "time travel".to_owned(),
        })),
        "deferred"
    );
}

#[test]
fn an_attribute_that_is_missing_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.character.special.remove(&Special::Luck);
        }),
        "character"
    );
}

#[test]
fn attributes_adding_up_to_more_than_a_character_can_have_are_refused() {
    assert_eq!(
        refused_part(|save| {
            for level in save.character.special.values_mut() {
                *level = 10;
            }
        }),
        "character"
    );
}

#[test]
fn a_perk_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.character.perks.insert(
                PerkId::try_new("no_such_perk").expect("literal is a valid id"),
                1,
            );
        }),
        "character"
    );
}

#[test]
fn a_trait_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| save
            .character
            .traits
            .push(TraitId::try_new("no_such_trait").expect("literal is a valid id"))),
        "character"
    );
}

#[test]
fn a_perk_pick_nobody_earned_is_refused() {
    assert_eq!(
        refused_part(|save| save.character.perk_picks += 1),
        "character"
    );
}

#[test]
fn too_many_tagged_skills_are_refused() {
    assert_eq!(
        refused_part(|save| {
            save.character.tagged = vec![Skill::Guns, Skill::Barter, Skill::Speech, Skill::Sneak];
        }),
        "character"
    );
}

#[test]
fn a_skill_pushed_past_one_hundred_is_refused() {
    assert_eq!(
        refused_part(|save| {
            save.character.spent.insert(Skill::Guns, 250);
        }),
        "character"
    );
}

#[test]
fn a_find_the_game_does_not_have_is_refused() {
    assert_eq!(
        refused_part(|save| save
            .gathered
            .push(FindId::try_new("no_such_find").expect("literal is a valid id"))),
        "gathered"
    );
}

#[test]
fn a_find_taken_twice_is_refused() {
    assert_eq!(
        refused_part(|save| {
            let again = save.gathered[0].clone();
            save.gathered.push(again);
        }),
        "gathered"
    );
}
