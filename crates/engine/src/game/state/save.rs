//! Turning a running game into a [`Save`] and back.
//!
//! Saving reads the state out as plain lists, sorted so the same game always gives
//! the same text. Loading never trusts the file: it starts from a game built from the
//! data, then puts each part back through the same checks play uses (the quest log
//! refuses a stage that does not exist, the pack an item it has never heard of, the
//! character a perk it may not have), and finally checks the facts that span parts.
//! Quest rewards are not paid again, since the effects of the stages reached are
//! already in the pack, the standing and the flags the save holds.

use std::collections::HashSet;
use std::num::NonZeroU32;

use super::{Active, ActiveEffects, Deferred, Game};
use crate::addiction::{AddictionKind, Addictions};
use crate::character::{Character, CharacterRecord, SpecialSet};
use crate::clock::GameClock;
use crate::gather::Gathered;
use crate::inventory::{CopyId, Slot};
use crate::poison::Poison;
use crate::radiation::Radiation;
use crate::saves::{
    Save, SaveError, SavedAddiction, SavedCharacter, SavedDeferred, SavedEffect, SavedEnding,
    SavedFate, SavedFired, SavedNpc, SavedPack, SavedPhase, SavedQuest, SavedScore, SavedShop,
    SavedStanding, SavedWeapon, SavedWorld, VERSION,
};
use crate::shared::{
    InventorySink, InventoryView, NpcId, QuestId, QuestSink, QuestStage, QuestStatus, QuestView,
    Rads, Renown, Special, SpecialLevel, StandingSink, StandingView, WorldSink,
};
use crate::world::Fate;
use strum::IntoEnumIterator;

/// Features whose effects a game may have postponed; a save naming any other is
/// from a different game.
const DEFERRABLE_FEATURES: [&str; 1] = ["companions"];

impl Game {
    /// Everything that changes during play, as a record that can be written to disk.
    ///
    /// A fight in progress is not part of the game until it is settled, so a save
    /// holds the state from before the fight began.
    #[must_use]
    pub fn save(&self) -> Save {
        let mut quests: Vec<SavedQuest> = self
            .quests
            .progress()
            .map(|(quest, status, stage)| SavedQuest {
                quest: quest.clone(),
                status: match status {
                    QuestStatus::Completed => SavedPhase::Completed,
                    QuestStatus::Failed => SavedPhase::Failed,
                    QuestStatus::Active | QuestStatus::NotStarted => SavedPhase::Active,
                },
                stage,
            })
            .collect();
        quests.sort_by(|a, b| a.quest.cmp(&b.quest));

        let mut flags: Vec<_> = self.world.flags().cloned().collect();
        flags.sort();
        let mut npcs: Vec<SavedNpc> = self
            .world
            .touched()
            .map(|(npc, touched)| SavedNpc {
                npc: npc.clone(),
                fate: match touched.fate {
                    Fate::Friendly => SavedFate::Friendly,
                    Fate::Hostile => SavedFate::Hostile,
                    Fate::Dead => SavedFate::Dead,
                    Fate::Removed => SavedFate::Removed,
                },
                met: touched.met,
            })
            .collect();
        npcs.sort_by(|a, b| a.npc.cmp(&b.npc));
        let mut endings: Vec<SavedEnding> = self
            .world
            .endings()
            .map(|(ending, open)| SavedEnding {
                ending: ending.clone(),
                open,
            })
            .collect();
        endings.sort_by(|a, b| a.ending.cmp(&b.ending));

        let factions = self
            .standing
            .factions()
            .map(|(faction, _)| SavedScore {
                faction: faction.clone(),
                fame: self.standing.renown_points(faction, Renown::Fame),
                infamy: self.standing.renown_points(faction, Renown::Infamy),
            })
            .filter(|score| score.fame != 0.0 || score.infamy != 0.0)
            .collect();

        let record = self.character.record();
        let character = SavedCharacter {
            special: Special::iter()
                .map(|stat| (stat, record.special.get(stat).get()))
                .collect(),
            xp: record.xp,
            spent: record.spent.into_iter().collect(),
            tagged: record.tagged,
            traits: record.traits,
            perks: record.perks.into_iter().collect(),
            skill_halves: record.skill_halves,
            perk_picks: record.perk_picks,
        };

        let pack = SavedPack {
            caps: self.inventory.caps(),
            items: self
                .inventory
                .stacks()
                .map(|(item, count)| (item.clone(), count.get()))
                .collect(),
            weapons: self
                .inventory
                .weapons()
                .map(|(id, copy)| SavedWeapon {
                    id: id.get(),
                    item: copy.item().clone(),
                    lost: copy.lost(),
                    mods: copy.mods().to_vec(),
                })
                .collect(),
            wielded: self.inventory.wielded().map(CopyId::get),
            weapon: None,
            outfit: self.inventory.equipped(Slot::Outfit).cloned(),
            head: self.inventory.equipped(Slot::Head).cloned(),
        };

        let shops = self
            .market
            .shops()
            .map(|(vendor, shop)| SavedShop {
                vendor: vendor.clone(),
                caps: shop.caps(),
                stock: shop
                    .stock()
                    .map(|(item, count)| (item.clone(), count.get()))
                    .collect(),
            })
            .collect();

        let effects = self
            .effects
            .iter()
            .map(|active| SavedEffect {
                source: active.source.clone(),
                stat: active.stat,
                amount: active.amount,
                until: active.until,
            })
            .collect();

        let mut won: Vec<_> = self.won.iter().cloned().collect();
        won.sort();
        let mut fired: Vec<SavedFired> = self
            .fired
            .iter()
            .map(|(quest, rule)| SavedFired {
                quest: quest.clone(),
                rule: *rule,
            })
            .collect();
        fired.sort_by(|a, b| (&a.quest, a.rule).cmp(&(&b.quest, b.rule)));
        let deferred = self
            .deferred
            .iter()
            .map(|postponed| SavedDeferred {
                quest: postponed.quest.clone(),
                effect: postponed.effect.clone(),
                feature: postponed.feature.to_owned(),
            })
            .collect();

        Save {
            version: VERSION,
            clock_minutes: self.clock.as_minutes(),
            at: self.at.clone(),
            wounds: self.wounds,
            quests,
            world: SavedWorld {
                flags,
                npcs,
                endings,
            },
            standing: SavedStanding {
                karma: self.standing.karma().get(),
                factions,
            },
            character,
            pack,
            shops,
            effects,
            rads: self.radiation.rads().get(),
            poison: self.poison.left(),
            addictions: self
                .addictions
                .iter()
                .map(|(kind, last_dose)| SavedAddiction {
                    kind: kind.to_string(),
                    last_dose,
                })
                .collect(),
            gathered: self.gathered.iter().cloned().collect(),
            won,
            fired,
            deferred,
        }
    }

    /// A game loaded from `save`: this game's data (quests, items, maps, fights)
    /// with the state of the save in place of its own. `self` is only read, so it
    /// can be the game just built from the data.
    ///
    /// # Errors
    ///
    /// [`SaveError::Invalid`] naming the part of the save that cannot be real in
    /// this game: an unknown quest, item, NPC or vendor, a stage that does not exist,
    /// a character no rules could produce, wounds past the maximum, and so on.
    /// `self` is left as it was.
    pub fn restore(&self, save: &Save) -> Result<Self, SaveError> {
        if save.version != VERSION {
            return Err(SaveError::invalid(
                "version",
                format!("expected {VERSION}, got {}", save.version),
            ));
        }
        let mut game = Self {
            book: self.book.clone(),
            quests: crate::quests::QuestLog::new(self.book.clone()),
            world: self.world.emptied(),
            standing: self.standing.emptied(),
            character: self.character.clone(),
            inventory: self.inventory.emptied(),
            arsenal: self.arsenal.clone(),
            weapon_mods: self.weapon_mods.clone(),
            perks: self.perks.clone(),
            market: self.market.refilled(),
            beds: self.beds.clone(),
            recipes: self.recipes.clone(),
            stations: self.stations.clone(),
            finds: self.finds.clone(),
            gathered: Gathered::default(),
            clock: GameClock::from_minutes(save.clock_minutes),
            effects: ActiveEffects::default(),
            radiation: Radiation::default(),
            poison: Poison::default(),
            addictions: Addictions::default(),
            addiction_book: self.addiction_book.clone(),
            map: self.map.clone(),
            at: None,
            wounds: 0,
            won: HashSet::new(),
            fired: HashSet::new(),
            deferred: Vec::new(),
        };
        game.restore_quests(save)?;
        game.restore_world(save)?;
        game.restore_standing(save)?;
        game.restore_character(save, self)?;
        game.restore_pack(save)?;
        game.restore_shops(save)?;
        game.restore_ailments(save)?;
        game.restore_effects(save)?;
        game.restore_place(save, self)?;
        // The time-bound perks settle first, so the cap on the wounds is the maximum
        // the player had at the moment of the save.
        game.settle();
        game.restore_wounds(save)?;
        game.settle();
        game.restore_gathered(save)?;
        game.restore_history(save)?;
        Ok(game)
    }

    fn restore_quests(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |quest: &QuestId, reason: &dyn std::fmt::Display| {
            SaveError::invalid("quests", format!("{quest}: {reason}"))
        };
        let mut seen = HashSet::new();
        for entry in &save.quests {
            if !seen.insert(&entry.quest) {
                return Err(bad(&entry.quest, &"listed twice"));
            }
            let log = &mut self.quests;
            let replayed = match (entry.status, entry.stage) {
                (SavedPhase::Active, None) => log.start_quest(&entry.quest),
                (SavedPhase::Active, Some(stage)) => log.set_stage(&entry.quest, stage),
                (SavedPhase::Completed, None) => log
                    .start_quest(&entry.quest)
                    .and_then(|()| log.complete_quest(&entry.quest)),
                (SavedPhase::Completed, Some(stage)) => log
                    .set_stage(&entry.quest, stage)
                    .and_then(|()| log.complete_quest(&entry.quest)),
                (SavedPhase::Failed, None) => log.fail_quest(&entry.quest),
                (SavedPhase::Failed, Some(stage)) => log
                    .set_stage(&entry.quest, stage)
                    .and_then(|()| log.fail_quest(&entry.quest)),
            };
            replayed.map_err(|error| bad(&entry.quest, &error))?;
            // A stage that finishes a quest cannot belong to one that is still going.
            let status = match entry.status {
                SavedPhase::Active => QuestStatus::Active,
                SavedPhase::Completed => QuestStatus::Completed,
                SavedPhase::Failed => QuestStatus::Failed,
            };
            let stage: Option<QuestStage> = entry.stage;
            if self.quests.quest_status(&entry.quest) != status
                || self.quests.quest_stage(&entry.quest) != stage
            {
                return Err(bad(&entry.quest, &"the status does not fit the stage"));
            }
        }
        Ok(())
    }

    fn restore_world(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("world", what);
        for flag in &save.world.flags {
            self.world.set_flag(flag, true).map_err(|e| bad(&e))?;
        }
        let mut seen: HashSet<&NpcId> = HashSet::new();
        for entry in &save.world.npcs {
            if !seen.insert(&entry.npc) {
                return Err(bad(&format!("{} is listed twice", entry.npc)));
            }
            // Met first, while the NPC is still alive: nobody can be met after.
            if entry.met {
                self.world.meet(&entry.npc).map_err(|e| bad(&e))?;
            }
            match entry.fate {
                SavedFate::Friendly => self.world.set_friendly(&entry.npc),
                SavedFate::Hostile => self.world.set_hostile(&entry.npc),
                SavedFate::Dead => self.world.kill_npc(&entry.npc),
                SavedFate::Removed => self.world.remove_npc(&entry.npc),
            }
            .map_err(|e| bad(&e))?;
        }
        for ending in &save.world.endings {
            if ending.open {
                self.world.unlock_ending(&ending.ending)
            } else {
                self.world.lock_ending(&ending.ending)
            }
            .map_err(|e| bad(&e))?;
        }
        Ok(())
    }

    fn restore_standing(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("standing", what);
        let mut seen = HashSet::new();
        for score in &save.standing.factions {
            if !seen.insert(&score.faction) {
                return Err(bad(&format!("{} is listed twice", score.faction)));
            }
            self.standing
                .add_reputation(&score.faction, score.fame, score.infamy)
                .map_err(|e| bad(&e))?;
        }
        self.standing
            .add_karma(save.standing.karma)
            .map_err(|e| bad(&e))
    }

    fn restore_character(&mut self, save: &Save, template: &Self) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("character", what);
        let saved = &save.character;
        let mut special = SpecialSet::default();
        for stat in Special::iter() {
            let level = saved
                .special
                .get(&stat)
                .ok_or_else(|| bad(&format!("{} is missing", stat.name())))?;
            let level = SpecialLevel::try_new(u32::from(*level)).map_err(|e| bad(&e))?;
            special = special.with(stat, level);
        }
        let record = CharacterRecord {
            special,
            xp: saved.xp,
            spent: saved.spent.iter().map(|(s, n)| (*s, *n)).collect(),
            tagged: saved.tagged.clone(),
            traits: saved.traits.clone(),
            perks: saved.perks.iter().map(|(p, r)| (p.clone(), *r)).collect(),
            skill_halves: saved.skill_halves,
            perk_picks: saved.perk_picks,
        };
        self.character = Character::restore(&record, template.character.rules(), &self.perks)
            .map_err(|e| bad(&e))?;
        Ok(())
    }

    fn restore_pack(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("pack", what);
        let pack = &save.pack;
        self.inventory
            .add_caps(f64::from(pack.caps))
            .map_err(|e| bad(&e))?;
        for saved in &pack.weapons {
            let fitting = self.weapon_mods.fitting(&saved.mods);
            let lost = self
                .arsenal
                .condition_max(&saved.item, &fitting)
                .map_or(0, |max| max.get());
            if saved.lost > lost {
                return Err(bad(&format!(
                    "{} lost {} of {lost} points",
                    saved.item, saved.lost
                )));
            }
            self.inventory
                .restore_weapon(CopyId::from_saved(saved.id), &saved.item, saved.lost)
                .map_err(|e| bad(&e))?;
            for item in &saved.mods {
                if !self.weapon_mods.fits(item, &saved.item) {
                    return Err(bad(&format!("{item} does not fit {}", saved.item)));
                }
            }
            self.inventory
                .restore_mods(CopyId::from_saved(saved.id), &saved.mods)
                .map_err(|e| bad(&e))?;
        }
        for (item, count) in &pack.items {
            let count = NonZeroU32::new(*count)
                .ok_or_else(|| bad(&format!("{item} is carried zero times")))?;
            self.inventory.give_item(item, count).map_err(|e| bad(&e))?;
        }
        if pack.wielded.is_some() && pack.weapon.is_some() {
            return Err(bad(&"the weapon in hand is named twice"));
        }
        if let Some(id) = pack.wielded {
            self.inventory
                .equip_copy(CopyId::from_saved(id))
                .map_err(|e| bad(&e))?;
        }
        for (slot, item) in [
            (Slot::Weapon, &pack.weapon),
            (Slot::Outfit, &pack.outfit),
            (Slot::Head, &pack.head),
        ] {
            let Some(item) = item else { continue };
            let placed = self.inventory.equip(item).map_err(|e| bad(&e))?;
            if placed != slot {
                return Err(bad(&format!("{item} does not go in that slot")));
            }
        }
        Ok(())
    }

    fn restore_shops(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("shops", what);
        let mut seen = HashSet::new();
        for shop in &save.shops {
            if !seen.insert(&shop.vendor) {
                return Err(bad(&format!("{} is listed twice", shop.vendor)));
            }
            let mut stock = Vec::with_capacity(shop.stock.len());
            for (item, count) in &shop.stock {
                let count = NonZeroU32::new(*count)
                    .ok_or_else(|| bad(&format!("{item} is stocked zero times")))?;
                if self.inventory.value(item).is_none() {
                    return Err(bad(&format!("{item} has no base value")));
                }
                stock.push((item.clone(), count));
            }
            self.market
                .set_shop(&shop.vendor, shop.caps, stock)
                .map_err(|e| bad(&e))?;
        }
        Ok(())
    }

    fn restore_effects(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("effects", what);
        let now = self.clock.as_minutes();
        let mut list = Vec::with_capacity(save.effects.len());
        for effect in &save.effects {
            if self.inventory.value(&effect.source).is_none() {
                return Err(bad(&format!("{} is not an item", effect.source)));
            }
            if effect.until <= now {
                return Err(bad(&format!("{} has already worn off", effect.source)));
            }
            list.push(Active {
                source: effect.source.clone(),
                stat: effect.stat,
                amount: effect.amount,
                until: effect.until,
            });
        }
        self.effects = ActiveEffects::from_entries(list);
        self.set_tally();
        Ok(())
    }

    fn restore_ailments(&mut self, save: &Save) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("ailments", what);
        let rads = Rads::try_new(u32::from(save.rads)).map_err(|e| bad(&e))?;
        self.radiation = Radiation::holding(rads);
        if save.poison > Poison::MAX {
            return Err(bad(&format!(
                "{} poison is more than a player can hold",
                save.poison
            )));
        }
        self.poison = Poison::holding(save.poison);
        let now = self.clock.as_minutes();
        let mut held = Vec::with_capacity(save.addictions.len());
        for entry in &save.addictions {
            let kind = AddictionKind::try_new(&entry.kind).map_err(|e| bad(&e))?;
            if kind.as_str() != entry.kind {
                return Err(bad(&format!(
                    "{:?} is not spelled the way the game spells it",
                    entry.kind
                )));
            }
            if !self.addiction_book.knows(&kind) {
                return Err(bad(&format!("no item addicts to {kind}")));
            }
            if entry.last_dose > now {
                return Err(bad(&format!(
                    "the last dose of {kind} is after the save was made"
                )));
            }
            if held
                .iter()
                .any(|(other, _): &(AddictionKind, u64)| *other == kind)
            {
                return Err(bad(&format!("{kind} is listed twice")));
            }
            held.push((kind, entry.last_dose));
        }
        self.addictions = Addictions::from_entries(held);
        Ok(())
    }

    fn restore_place(&mut self, save: &Save, template: &Self) -> Result<(), SaveError> {
        let bad = |what: &dyn std::fmt::Display| SaveError::invalid("place", what);
        match (&save.at, &template.at) {
            (Some(place), _) if self.map.contains(place) => self.at = Some(place.clone()),
            (Some(place), _) => return Err(bad(&format!("{place} is not on the map"))),
            (None, Some(_)) => return Err(bad(&"the save has no place but the game has a map")),
            (None, None) => {}
        }
        Ok(())
    }

    fn restore_wounds(&mut self, save: &Save) -> Result<(), SaveError> {
        let max = self.character.max_hp();
        if save.wounds > max {
            return Err(SaveError::invalid(
                "wounds",
                format!("{} hit points lost, but the maximum is {max}", save.wounds),
            ));
        }
        self.wounds = save.wounds;
        Ok(())
    }

    fn restore_gathered(&mut self, save: &Save) -> Result<(), SaveError> {
        for find in &save.gathered {
            if self.finds.get(find).is_none() {
                return Err(SaveError::invalid(
                    "gathered",
                    format!("unknown find {find}"),
                ));
            }
            self.gathered
                .take(find)
                .map_err(|error| SaveError::invalid("gathered", error))?;
        }
        Ok(())
    }

    fn restore_history(&mut self, save: &Save) -> Result<(), SaveError> {
        for won in &save.won {
            if self.arsenal.encounter(won).is_none() {
                return Err(SaveError::invalid("won", format!("unknown fight {won}")));
            }
            self.won.insert(won.clone());
        }
        for fired in &save.fired {
            if !self.book.contains(&fired.quest) {
                return Err(SaveError::invalid(
                    "fired",
                    format!("unknown quest {}", fired.quest),
                ));
            }
            if fired.rule >= self.book.rules(&fired.quest).len() {
                return Err(SaveError::invalid(
                    "fired",
                    format!("{} has no rule {}", fired.quest, fired.rule),
                ));
            }
            self.fired.insert((fired.quest.clone(), fired.rule));
        }
        for postponed in &save.deferred {
            let feature = DEFERRABLE_FEATURES
                .iter()
                .find(|known| **known == postponed.feature)
                .ok_or_else(|| {
                    SaveError::invalid("deferred", format!("unknown feature {}", postponed.feature))
                })?;
            if !self.book.contains(&postponed.quest) {
                return Err(SaveError::invalid(
                    "deferred",
                    format!("unknown quest {}", postponed.quest),
                ));
            }
            self.deferred.push(Deferred {
                quest: postponed.quest.clone(),
                effect: postponed.effect.clone(),
                feature,
            });
        }
        Ok(())
    }
}
