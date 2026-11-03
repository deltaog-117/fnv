//! The recipes the game can make, read-only, and why the others cannot be.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::error::CraftError;
use crate::shared::{
    CraftReaction, GameData, ItemId, Recipe, RecipeId, RecipeMode, Skill, SkillLevel, StationKind,
};

/// A recipe the game can make: every part of it is known.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    name: String,
    kind: StationKind,
    skill: Option<(Skill, SkillLevel)>,
    inputs: Vec<(ItemId, NonZeroU32)>,
    output: (ItemId, NonZeroU32),
    reactions: Vec<CraftReaction>,
}

impl Formula {
    /// What it makes, as the player reads it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The kind of station it needs.
    #[must_use]
    pub fn kind(&self) -> StationKind {
        self.kind
    }

    /// The skill and level it asks for, if any.
    #[must_use]
    pub fn skill(&self) -> Option<(Skill, SkillLevel)> {
        self.skill
    }

    /// What it uses up, each item once.
    pub fn inputs(&self) -> impl Iterator<Item = (&ItemId, NonZeroU32)> {
        self.inputs.iter().map(|(item, count)| (item, *count))
    }

    /// What it makes, and how many.
    #[must_use]
    pub fn output(&self) -> (&ItemId, NonZeroU32) {
        (&self.output.0, self.output.1)
    }

    /// What making it does to the story.
    #[must_use]
    pub fn reactions(&self) -> &[CraftReaction] {
        &self.reactions
    }
}

/// Why a recipe is not a [`Formula`] yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unavailable {
    /// Taking an item apart is not built.
    Breakdown,
    /// The wiki names no station for it.
    NoStation,
    /// The wiki's table gives no item it makes.
    NoOutput,
    /// The item it makes is not in the game's item book.
    UnknownOutput(ItemId),
    /// An ingredient matched no item; the text is the wiki's.
    UnmatchedIngredient(String),
    /// An ingredient is not in the game's item book.
    UnknownIngredient(ItemId),
    /// The wiki lists no ingredients.
    NoIngredients,
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Breakdown => f.write_str("taking things apart is not built"),
            Self::NoStation => f.write_str("the wiki names no station for it"),
            Self::NoOutput => f.write_str("the wiki gives no item it makes"),
            Self::UnknownOutput(item) => write!(f, "{item} is not an item the game has"),
            Self::UnmatchedIngredient(text) => write!(f, "no item matches {text:?}"),
            Self::UnknownIngredient(item) => write!(f, "{item} is not an item the game has"),
            Self::NoIngredients => f.write_str("the wiki lists no ingredients"),
        }
    }
}

/// Every recipe, sorted once at load into those the game can make and those it
/// cannot yet.
#[derive(Debug, Clone, Default)]
pub struct RecipeBook {
    formulas: BTreeMap<RecipeId, Formula>,
    unavailable: BTreeMap<RecipeId, Unavailable>,
}

impl RecipeBook {
    /// Builds the book from the loaded data. `knows` says whether an item exists in
    /// the game's item book.
    ///
    /// # Errors
    ///
    /// The same as [`RecipeBook::new`].
    pub fn from_data(data: &GameData, knows: impl Fn(&ItemId) -> bool) -> Result<Self, CraftError> {
        Self::new(data.recipes.iter(), knows)
    }

    /// Builds the book from recipe records.
    ///
    /// # Errors
    ///
    /// [`CraftError::DuplicateRecipe`] when an id repeats.
    pub fn new<'a>(
        records: impl IntoIterator<Item = &'a Recipe>,
        knows: impl Fn(&ItemId) -> bool,
    ) -> Result<Self, CraftError> {
        let mut book = Self::default();
        for record in records {
            let id = record.id.clone();
            if book.formulas.contains_key(&id) || book.unavailable.contains_key(&id) {
                return Err(CraftError::DuplicateRecipe(id));
            }
            match formula(record, &knows) {
                Ok(formula) => book.formulas.insert(id, formula).map(drop),
                Err(reason) => book.unavailable.insert(id, reason).map(drop),
            };
        }
        Ok(book)
    }

    /// The formula with this id, if the game can make it.
    #[must_use]
    pub fn get(&self, recipe: &RecipeId) -> Option<&Formula> {
        self.formulas.get(recipe)
    }

    /// The formulas that need a station of this kind, in id order.
    pub fn for_kind(&self, kind: StationKind) -> impl Iterator<Item = (&RecipeId, &Formula)> {
        self.formulas
            .iter()
            .filter(move |(_, formula)| formula.kind == kind)
    }

    /// Every recipe the game cannot make yet, with the reason, in id order.
    pub fn unavailable(&self) -> impl Iterator<Item = (&RecipeId, &Unavailable)> {
        self.unavailable.iter()
    }

    /// How many recipes the game can make.
    #[must_use]
    pub fn len(&self) -> usize {
        self.formulas.len()
    }

    /// Whether the game can make no recipe.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }
}

/// The formula for a recipe, or the first reason it is not one.
fn formula(record: &Recipe, knows: &impl Fn(&ItemId) -> bool) -> Result<Formula, Unavailable> {
    if record.mode == RecipeMode::Breakdown {
        return Err(Unavailable::Breakdown);
    }
    let kind = record.station.ok_or(Unavailable::NoStation)?;
    let output = record.output_item_id.clone().ok_or(Unavailable::NoOutput)?;
    if !knows(&output) {
        return Err(Unavailable::UnknownOutput(output));
    }
    if record.ingredients.is_empty() {
        return Err(Unavailable::NoIngredients);
    }
    let mut inputs: Vec<(ItemId, NonZeroU32)> = Vec::with_capacity(record.ingredients.len());
    for line in &record.ingredients {
        let item = line
            .item_id
            .clone()
            .ok_or_else(|| Unavailable::UnmatchedIngredient(line.item.clone()))?;
        if !knows(&item) {
            return Err(Unavailable::UnknownIngredient(item));
        }
        // An item listed twice is one ingredient of the combined amount.
        match inputs.iter_mut().find(|(known, _)| known == &item) {
            Some((_, count)) => *count = count.saturating_add(line.qty.get()),
            None => inputs.push((item, line.qty)),
        }
    }
    Ok(Formula {
        name: record.name.clone(),
        kind,
        skill: record.skill.map(|need| (need.skill, need.min)),
        inputs,
        output: (output, record.output_qty.unwrap_or(NonZeroU32::MIN)),
        reactions: record.on_craft.clone(),
    })
}
