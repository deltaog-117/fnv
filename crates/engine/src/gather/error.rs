//! Why a find list could not be built, or a pickup was refused.

use thiserror::Error;

use crate::shared::{EffectError, FindId, ItemId};

/// A find list could not be built, or a pickup was refused. Nothing changes when
/// one of these is returned.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GatherError {
    /// Two finds share an id.
    #[error("find id {0} appears twice")]
    DuplicateFind(FindId),
    /// A find lists the same item twice.
    #[error("find {find} lists {item} twice")]
    DuplicateItem {
        /// The find.
        find: FindId,
        /// The repeated item.
        item: ItemId,
    },
    /// The player has no hit points left.
    #[error("the player is down")]
    Down,
    /// No such find.
    #[error("unknown find {0}")]
    UnknownFind(FindId),
    /// The player already took it.
    #[error("{0} has already been taken")]
    AlreadyTaken(FindId),
    /// The pack refused the items.
    #[error(transparent)]
    Effect(#[from] EffectError),
}
