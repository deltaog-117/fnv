//! Composition root: assembles each feature's state and implements the view traits
//! the shared evaluator reads. The only module allowed to know every feature.
//!
//! [`Game`] holds the state of each feature built so far. Features that do not
//! exist yet answer reads with the shared defaults and refuse writes, so an
//! effect aimed at them fails loudly instead of vanishing.

mod state;

pub use state::Game;
