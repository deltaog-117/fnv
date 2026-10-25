//! Widgets, theme and input handling used by every screen.
//!
//! Nothing here knows a game rule or a screen: screens depend on this module and
//! never on each other.

pub mod dice;
pub mod input;
pub mod layout;
pub mod log;
pub mod menu;
pub mod theme;
