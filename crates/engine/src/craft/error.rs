//! Why a recipe book could not be built, or a craft was refused.

use thiserror::Error;

use crate::shared::{BatchError, EffectError, ItemId, RecipeId, Skill, StationId, StationKind};

/// A book could not be built, or a craft was refused. Nothing changes when one of
/// these is returned.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CraftError {
    /// Two recipes share an id.
    #[error("recipe id {0} appears twice")]
    DuplicateRecipe(RecipeId),
    /// Two stations share an id.
    #[error("station id {0} appears twice")]
    DuplicateStation(StationId),
    /// The player has no hit points left.
    #[error("the player is down")]
    Down,
    /// No such recipe, or it cannot be made yet.
    #[error("unknown recipe {0}")]
    UnknownRecipe(RecipeId),
    /// No such station.
    #[error("unknown station {0}")]
    UnknownStation(StationId),
    /// The recipe needs another kind of station.
    #[error("{recipe} needs a {needs}, and this is a {at}", needs = needs.name(), at = at.name())]
    WrongStation {
        /// The recipe.
        recipe: RecipeId,
        /// The kind it needs.
        needs: StationKind,
        /// The kind the player is at.
        at: StationKind,
    },
    /// The player's skill is too low.
    #[error("{recipe} needs {skill} {need}, and you have {have}", skill = skill.name())]
    SkillTooLow {
        /// The recipe.
        recipe: RecipeId,
        /// The skill.
        skill: Skill,
        /// The level the recipe asks for.
        need: u8,
        /// The player's level.
        have: u8,
    },
    /// The player lacks an ingredient.
    #[error("you carry {have} of {item}, and the recipe needs {need}")]
    Missing {
        /// The ingredient.
        item: ItemId,
        /// How many the player carries.
        have: u32,
        /// How many the recipe needs.
        need: u32,
    },
    /// The player's side of the craft failed.
    #[error(transparent)]
    Effect(#[from] EffectError),
    /// One of the recipe's reactions failed, so nothing was made.
    #[error(transparent)]
    Reaction(#[from] BatchError),
}
