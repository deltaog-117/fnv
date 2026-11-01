//! Fame and infamy per faction and town, reputation titles and karma.
//!
//! [`ReputationBook`] is the read-only list of factions that track reputation,
//! each with the points at which a tier starts and the 4x4 grid of titles.
//! [`Standing`] is what the player has earned: fame and infamy points per faction
//! and one karma number. It implements the shared `StandingView` and
//! `StandingSink` traits, so conditions and effects run against it unchanged.
//!
//! This feature owns these laws and nothing else:
//!
//! - gaining reputation only adds: a gain cannot take fame or infamy back, and each
//!   score stops at [`MAX_POINTS`] so it never overflows. Only forcing a standing
//!   can lower a score;
//! - the tier of a score is how many tier thresholds it has reached, so a higher
//!   score never has a lower tier;
//! - the seven standings of `ReputationLevel` are a pure function of the two tiers,
//!   and forcing a standing with `SetReputation` and reading it back gives the same
//!   standing;
//! - karma stays between [`Karma::MIN`] and [`Karma::MAX`];
//! - an effect aimed at a faction without a model is an error, never a silent
//!   no-op, and a refused effect changes nothing.
//!
//! Prices, quest gates and dialogue that read the standing belong to the features
//! that own them; the consequences the wiki lists per faction are data for them.

mod book;
mod error;
mod karma;
mod state;

pub use book::{FactionModel, ReputationBook, standing};
pub use error::ReputationError;
pub use karma::Karma;
pub use state::{MAX_POINTS, Standing};
