//! How long a walk takes.

use std::num::NonZeroU32;

/// Minutes to walk one pixel of the 1024x1024 map.
///
/// The wiki gives no walking speed, so two minutes a pixel is a stand-in: crossing
/// the whole map takes a day and a half. Replace it once the game-data export gives
/// the world's scale.
pub const MINUTES_PER_PIXEL: f64 = 2.0;

/// The minutes a walk of `distance` map pixels takes, rounded up and never zero.
///
/// `None` when the distance is not a finite number above zero. A distance too large
/// for a `u32` of minutes is capped at the largest, so a walk never wraps around.
#[must_use]
pub fn minutes_for(distance: f64) -> Option<NonZeroU32> {
    if !distance.is_finite() || distance <= 0.0 {
        return None;
    }
    let minutes = (distance * MINUTES_PER_PIXEL)
        .ceil()
        .clamp(1.0, f64::from(u32::MAX));
    // Clamped to the range a u32 holds, and at least one.
    NonZeroU32::new(minutes as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_walk_still_takes_a_minute() {
        assert_eq!(minutes_for(0.001).map(NonZeroU32::get), Some(1));
    }

    #[test]
    fn the_goodsprings_cave_is_a_walk_of_seventy_two_minutes() {
        // 35.9 pixels at two minutes each, rounded up.
        assert_eq!(minutes_for(35.9).map(NonZeroU32::get), Some(72));
    }

    #[test]
    fn a_distance_that_is_not_a_positive_number_is_refused() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(minutes_for(bad), None, "{bad}");
        }
    }

    #[test]
    fn an_enormous_distance_is_capped_not_wrapped() {
        assert_eq!(minutes_for(1.0e300).map(NonZeroU32::get), Some(u32::MAX));
    }
}
