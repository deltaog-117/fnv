//! Prices: an item's base value, the player's Barter skill and how many.
//!
//! The multipliers are the two `barter_*_multiplier` formulas in
//! `data/rules/formulas.json`, and a test pins these constants to that file.

use std::num::NonZeroU32;

use crate::shared::{DiscountPercent, SkillLevel};

/// Multiplier on the base value when buying at Barter 0.
pub const BUY_AT_ZERO: f64 = 1.55;
/// Multiplier on the base value when selling at Barter 0.
pub const SELL_AT_ZERO: f64 = 0.45;
/// How far each Barter point moves either multiplier.
pub const PER_POINT: f64 = 0.0045;

/// What the player pays per cap of base value: 1.55 at Barter 0, 1.10 at 100.
#[must_use]
pub fn buy_multiplier(barter: SkillLevel) -> f64 {
    BUY_AT_ZERO - PER_POINT * f64::from(barter.get())
}

/// What the player is paid per cap of base value: 0.45 at Barter 0, 0.90 at 100.
#[must_use]
pub fn sell_multiplier(barter: SkillLevel) -> f64 {
    SELL_AT_ZERO + PER_POINT * f64::from(barter.get())
}

/// Caps to buy `count` of an item worth `value`, never less than one.
///
/// The price of the whole line is rounded, not each item, so a stack of cheap
/// items is not rounded up one by one.
#[must_use]
pub fn buy_total(value: f64, count: NonZeroU32, barter: SkillLevel) -> u32 {
    round_caps(value * f64::from(count.get()) * buy_multiplier(barter)).max(1)
}

/// Caps to buy `count` of an item worth `value` with `percent` taken off the price,
/// never less than one. The cut is applied to the whole line before the single
/// rounding, so it does not round twice.
#[must_use]
pub fn buy_total_with_discount(
    value: f64,
    count: NonZeroU32,
    barter: SkillLevel,
    percent: DiscountPercent,
) -> u32 {
    let kept = f64::from(100 - percent.get()) / 100.0;
    round_caps(value * f64::from(count.get()) * buy_multiplier(barter) * kept).max(1)
}

/// Caps paid for `count` of an item worth `value`; zero when the line is worth
/// less than half a cap.
#[must_use]
pub fn sell_total(value: f64, count: NonZeroU32, barter: SkillLevel) -> u32 {
    round_caps(value * f64::from(count.get()) * sell_multiplier(barter))
}

/// Caps to buy `count` of an item worth `value` at a vendor who deals at base value,
/// never less than one.
#[must_use]
pub fn flat_buy_total(value: f64, count: NonZeroU32) -> u32 {
    round_caps(value * f64::from(count.get())).max(1)
}

/// Caps paid for `count` of an item worth `value` by a vendor who deals at base
/// value; zero when the line is worth less than half a cap.
#[must_use]
pub fn flat_sell_total(value: f64, count: NonZeroU32) -> u32 {
    round_caps(value * f64::from(count.get()))
}

fn round_caps(caps: f64) -> u32 {
    // Clamped to the range a u32 holds, and finite because a value is checked
    // to be finite when the item book is built.
    caps.round().clamp(0.0, f64::from(u32::MAX)) as u32
}
