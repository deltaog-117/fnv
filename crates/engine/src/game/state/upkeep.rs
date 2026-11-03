//! What the player does to keep going: repair a weapon, craft, pick things up.
//!
//! Each action runs on a copy of the game and replaces it only when every step
//! worked, so a refusal changes nothing.

use std::num::NonZeroU32;

use super::Game;
use crate::combat::Percent;
use crate::craft::{CraftError, Formula};
use crate::gather::GatherError;
use crate::repair::{RepairError, WEAPON_REPAIR_KIT, after_kit, after_merge, vendor_price};
use crate::shared::{
    CharacterView, FindId, InventorySink, InventoryView, ItemId, LocationId, RecipeId, Skill,
    SkillLevel, StationId, VendorId, apply_all, evaluate,
};

/// What a vendor asks to mend a weapon, and where it would leave the weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairQuote {
    /// Caps the repair costs.
    pub price: u32,
    /// How much of the weapon's condition is left afterwards.
    pub condition: Percent,
}

/// What a craft made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crafted {
    /// The item.
    pub item: ItemId,
    /// How many.
    pub count: NonZeroU32,
}

impl Game {
    /// The weapon's new condition points and the points it has lost, when it wears.
    fn wear_of(&self, weapon: &ItemId) -> Result<(NonZeroU32, u32), RepairError> {
        let max = self
            .arsenal
            .condition_max(weapon)
            .ok_or_else(|| RepairError::DoesNotWear(weapon.clone()))?;
        let lost = self.wear.get(weapon).copied().unwrap_or(0).min(max.get());
        Ok((max, lost))
    }

    /// Records the points a weapon has lost; a whole weapon leaves no record.
    fn set_lost(&mut self, weapon: &ItemId, lost: u32) {
        if lost == 0 {
            self.wear.remove(weapon);
        } else {
            self.wear.insert(weapon.clone(), lost);
        }
    }

    /// The condition left after `lost` of `max` points are gone.
    fn left_percent(max: NonZeroU32, lost: u32) -> Percent {
        crate::combat::Condition::new(max, lost).percent()
    }

    /// The weapons the player carries that wear, with how much of each is left.
    #[must_use]
    pub fn worn_weapons(&self) -> Vec<(ItemId, Percent)> {
        self.inventory
            .stacks()
            .filter_map(|(item, _)| Some((item.clone(), self.weapon_condition(item)?)))
            .collect()
    }

    /// Mends a carried weapon with a weapon repair kit, which is used up. The weapon
    /// is lifted to the Repair skill's share of its condition and never made worse.
    /// Returns what is left of its condition.
    ///
    /// # Errors
    ///
    /// [`RepairError`] when the player is down, the item does not wear or is not
    /// carried, there is no kit, or the repair would change nothing.
    pub fn repair_with_kit(&mut self, weapon: &ItemId) -> Result<Percent, RepairError> {
        self.check_repairer(weapon)?;
        let (max, lost) = self.wear_of(weapon)?;
        let skill = self.character.skill(Skill::Repair);
        let after = after_kit(max, lost, skill);
        if after == lost {
            return Err(RepairError::NothingToGain {
                item: weapon.clone(),
                skill: skill.get(),
            });
        }
        let kit = kit_id();
        let mut trial = self.clone();
        require(&trial, &kit, 1)?;
        trial.inventory.take_item(&kit, NonZeroU32::MIN)?;
        trial.set_lost(weapon, after);
        *self = trial;
        Ok(Self::left_percent(max, after))
    }

    /// Merges two carried copies of one weapon into a single, better one: one copy
    /// is used up. Both copies share one wear record, so the pair is as worn as the
    /// record says. Returns what is left of the remaining copy's condition.
    ///
    /// # Errors
    ///
    /// [`RepairError`] when the player is down, the item does not wear, fewer than
    /// two copies are carried, or the merge would change nothing.
    pub fn merge_weapons(&mut self, weapon: &ItemId) -> Result<Percent, RepairError> {
        self.check_repairer(weapon)?;
        let (max, lost) = self.wear_of(weapon)?;
        let skill = self.character.skill(Skill::Repair);
        let after = after_merge(max, lost, lost, skill);
        if after == lost {
            return Err(RepairError::NothingToGain {
                item: weapon.clone(),
                skill: skill.get(),
            });
        }
        let mut trial = self.clone();
        require(&trial, weapon, 2)?;
        trial.inventory.take_item(weapon, NonZeroU32::MIN)?;
        trial.set_lost(weapon, after);
        *self = trial;
        Ok(Self::left_percent(max, after))
    }

    fn check_repairer(&self, weapon: &ItemId) -> Result<(), RepairError> {
        if !self.is_alive() {
            return Err(RepairError::Down);
        }
        require(self, weapon, 1)
    }

    /// The merchants at `place` who mend weapons for caps and are willing to trade.
    /// None while the player is down.
    pub fn repairers_at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a VendorId, &'a str)> {
        let open = self.is_alive();
        self.market
            .book()
            .at(place)
            .filter(move |(_, info)| {
                open && info.repair_skill().is_some() && self.vendor_will_trade(info.npc())
            })
            .map(|(id, info)| (id, info.name()))
    }

    /// What a vendor asks to mend a weapon, and where it would leave the weapon.
    ///
    /// # Errors
    ///
    /// [`RepairError`] when the vendor is unknown, does not repair or will not
    /// trade, the item does not wear or is not carried, it has no price, or the
    /// vendor could not improve it.
    pub fn repair_quote(
        &self,
        vendor: &VendorId,
        weapon: &ItemId,
    ) -> Result<RepairQuote, RepairError> {
        let info = self
            .market
            .book()
            .get(vendor)
            .ok_or_else(|| RepairError::UnknownVendor(vendor.clone()))?;
        let skill = info
            .repair_skill()
            .ok_or_else(|| RepairError::NoService(vendor.clone()))?;
        if !self.is_alive() {
            return Err(RepairError::Down);
        }
        if !self.vendor_will_trade(info.npc()) {
            return Err(RepairError::Unavailable(info.npc().clone()));
        }
        require(self, weapon, 1)?;
        let (max, lost) = self.wear_of(weapon)?;
        let after = after_kit(max, lost, skill);
        if after == lost {
            return Err(RepairError::NothingToGain {
                item: weapon.clone(),
                skill: skill.get(),
            });
        }
        let value = self
            .inventory
            .value(weapon)
            .ok_or_else(|| RepairError::NoPrice(weapon.clone()))?;
        Ok(RepairQuote {
            price: vendor_price(value, max, lost, after),
            condition: Self::left_percent(max, after),
        })
    }

    /// Has a vendor mend a carried weapon for caps, up to the vendor's own Repair
    /// skill. The player and the vendor change together or not at all.
    ///
    /// # Errors
    ///
    /// The same as [`Game::repair_quote`], and [`RepairError::PlayerShort`] when the
    /// player cannot pay.
    pub fn repair_at(
        &mut self,
        vendor: &VendorId,
        weapon: &ItemId,
    ) -> Result<RepairQuote, RepairError> {
        let quote = self.repair_quote(vendor, weapon)?;
        let have = self.inventory.caps();
        if have < quote.price {
            return Err(RepairError::PlayerShort {
                have,
                need: quote.price,
            });
        }
        let skill = self
            .market
            .book()
            .get(vendor)
            .and_then(|info| info.repair_skill())
            .unwrap_or_default();
        let (max, lost) = self.wear_of(weapon)?;
        let mut trial = self.clone();
        trial.inventory.add_caps(-f64::from(quote.price))?;
        trial
            .market
            .earn(vendor, quote.price)
            .map_err(|_| RepairError::UnknownVendor(vendor.clone()))?;
        trial.set_lost(weapon, after_kit(max, lost, skill));
        *self = trial;
        Ok(quote)
    }

    /// The crafting stations at `place`. None while the player is down.
    pub fn stations_at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a StationId, &'a str)> {
        let open = self.is_alive();
        self.stations
            .at(place)
            .filter(move |_| open)
            .map(|(id, info)| (id, info.name()))
    }

    /// The recipes the game can make at a station, in id order.
    pub fn recipes_at<'a>(&'a self, station: &StationId) -> Vec<(&'a RecipeId, &'a Formula)> {
        self.stations
            .get(station)
            .map(|info| self.recipes.for_kind(info.kind()).collect())
            .unwrap_or_default()
    }

    /// Whether the player could make the recipe at the station right now.
    ///
    /// # Errors
    ///
    /// [`CraftError`] naming the first thing in the way.
    pub fn can_craft(&self, station: &StationId, recipe: &RecipeId) -> Result<(), CraftError> {
        if !self.is_alive() {
            return Err(CraftError::Down);
        }
        let info = self
            .stations
            .get(station)
            .ok_or_else(|| CraftError::UnknownStation(station.clone()))?;
        let formula = self
            .recipes
            .get(recipe)
            .ok_or_else(|| CraftError::UnknownRecipe(recipe.clone()))?;
        if formula.kind() != info.kind() {
            return Err(CraftError::WrongStation {
                recipe: recipe.clone(),
                needs: formula.kind(),
                at: info.kind(),
            });
        }
        if let Some((skill, need)) = formula.skill() {
            let have = self.character.skill(skill);
            if have < need {
                return Err(CraftError::SkillTooLow {
                    recipe: recipe.clone(),
                    skill,
                    need: need.get(),
                    have: have.get(),
                });
            }
        }
        for (item, need) in formula.inputs() {
            let have = self.inventory.item_count(item);
            if have < need.get() {
                return Err(CraftError::Missing {
                    item: item.clone(),
                    have,
                    need: need.get(),
                });
            }
        }
        Ok(())
    }

    /// Makes a recipe at a station: the ingredients leave the pack, the product
    /// arrives and the recipe's reactions run, together or not at all.
    ///
    /// # Errors
    ///
    /// [`CraftError`] when the recipe or station is unknown, the kinds differ, the
    /// skill is too low, an ingredient is missing, or a reaction fails.
    pub fn craft(&mut self, station: &StationId, recipe: &RecipeId) -> Result<Crafted, CraftError> {
        self.can_craft(station, recipe)?;
        let recipes = std::sync::Arc::clone(&self.recipes);
        let formula = recipes
            .get(recipe)
            .ok_or_else(|| CraftError::UnknownRecipe(recipe.clone()))?;
        let mut trial = self.clone();
        for (item, need) in formula.inputs() {
            trial.inventory.take_item(item, need)?;
        }
        let (item, count) = formula.output();
        trial.inventory.give_item(item, count)?;
        for reaction in formula.reactions() {
            if reaction
                .when
                .iter()
                .all(|condition| evaluate(condition, &trial))
            {
                apply_all(reaction.effects.as_slice(), &mut trial)?;
            }
        }
        *self = trial;
        Ok(Crafted {
            item: item.clone(),
            count,
        })
    }

    /// The finds at `place` that the player has not taken. None while the player is
    /// down.
    pub fn finds_at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a FindId, &'a str)> {
        let open = self.is_alive();
        self.finds
            .at(place)
            .filter(move |(id, _)| open && !self.gathered.has_taken(id))
            .map(|(id, info)| (id, info.name()))
    }

    /// Picks up a find: the items arrive in the pack and the find is marked taken,
    /// together or not at all.
    ///
    /// # Errors
    ///
    /// [`GatherError`] when the player is down, the find is unknown, it was already
    /// taken, or the pack refuses an item.
    pub fn gather(&mut self, find: &FindId) -> Result<(), GatherError> {
        if !self.is_alive() {
            return Err(GatherError::Down);
        }
        let info = self
            .finds
            .get(find)
            .ok_or_else(|| GatherError::UnknownFind(find.clone()))?;
        let items: Vec<_> = info
            .items()
            .map(|(item, count)| (item.clone(), count))
            .collect();
        let mut trial = self.clone();
        trial.gathered.take(find)?;
        for (item, count) in items {
            trial.inventory.give_item(&item, count)?;
        }
        *self = trial;
        Ok(())
    }

    /// Whether the player has taken this find.
    #[must_use]
    pub fn has_gathered(&self, find: &FindId) -> bool {
        self.gathered.has_taken(find)
    }

    /// The Repair skill's ceiling for the player now, as a percent of a new weapon.
    #[must_use]
    pub fn repair_ceiling(&self) -> SkillLevel {
        self.character.skill(Skill::Repair)
    }
}

/// The weapon repair kit's item id.
fn kit_id() -> ItemId {
    ItemId::try_new(WEAPON_REPAIR_KIT)
        .unwrap_or_else(|_| unreachable!("the kit constant is a valid id"))
}

/// Refuses when the player carries fewer than `need` of an item.
fn require(game: &Game, item: &ItemId, need: u32) -> Result<(), RepairError> {
    let have = game.inventory.item_count(item);
    if have < need {
        return Err(RepairError::NotCarried {
            item: item.clone(),
            have,
            need,
        });
    }
    Ok(())
}
