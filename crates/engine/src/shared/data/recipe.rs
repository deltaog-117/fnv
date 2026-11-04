//! Recipe records: what can be made from what, where and with which skill.

use std::num::NonZeroU32;

use serde::Deserialize;

use crate::shared::{Condition, Effect, ItemId, NonEmpty, RecipeId, Skill, SkillLevel};

/// A kind of crafting station, spelled as `recipes.json` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum StationKind {
    /// A campfire.
    #[serde(rename = "Campfire")]
    Campfire,
    /// A workbench.
    #[serde(rename = "Workbench")]
    Workbench,
    /// A reloading bench.
    #[serde(rename = "Reloading bench")]
    ReloadingBench,
}

impl StationKind {
    /// The name the player sees.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Campfire => "campfire",
            Self::Workbench => "workbench",
            Self::ReloadingBench => "reloading bench",
        }
    }
}

/// Whether a recipe makes its item or takes it apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeMode {
    /// The ingredients make the item.
    Craft,
    /// The item is taken apart into the listed items.
    Breakdown,
}

/// The skill a recipe asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeSkill {
    /// The skill.
    pub skill: Skill,
    /// The level needed.
    pub min: SkillLevel,
}

/// One ingredient line.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ingredient {
    /// The ingredient as the wiki writes it.
    pub item: String,
    /// The item, when the wiki's name matched one.
    pub item_id: Option<ItemId>,
    /// How many.
    pub qty: NonZeroU32,
    /// A tool the wiki marks "not expended": it must be carried, and is not used up.
    #[serde(default)]
    pub kept: bool,
}

/// What making a recipe does to the story, while its conditions hold.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CraftReaction {
    /// Every condition must hold; empty means always.
    #[serde(default)]
    pub when: Vec<Condition>,
    /// Applied in order.
    pub effects: NonEmpty<Effect>,
}

/// A recipe.
///
/// The wiki's table is imperfect: some recipes have no output item, some list
/// ingredients that matched no item, and some name no station. Those load as they
/// are, and the engine's recipe book says why each one cannot be made yet.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// The recipe's unique id.
    pub id: RecipeId,
    /// What it makes.
    pub name: String,
    /// Make or take apart.
    pub mode: RecipeMode,
    /// Where the wiki lists it.
    pub table: Option<String>,
    /// How many it makes; one when absent.
    pub output_qty: Option<NonZeroU32>,
    /// The item it makes.
    pub output_item_id: Option<ItemId>,
    /// The skill it asks for, if any.
    pub skill: Option<RecipeSkill>,
    /// The kind of station it needs.
    pub station: Option<StationKind>,
    /// What it uses up.
    pub ingredients: Vec<Ingredient>,
    /// What making it does to the story.
    #[serde(default)]
    pub on_craft: Vec<CraftReaction>,
}
