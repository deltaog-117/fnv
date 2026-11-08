//! The poison held, and how fast it comes out.

/// Poison points that leave the blood, and cost a hit point each, every minute. A
/// stand-in: the wiki gives poison in hit points per second for a few seconds, and
/// the clock counts whole minutes, so the points are paid out at this rate.
pub const DRAIN_PER_MINUTE: u16 = 5;

/// How much poison gets in past `resistance` percent, rounded to the nearest point.
/// Resistance above 100 counts as 100, so it never lets a negative number in.
#[must_use]
pub fn taken_in(amount: u16, resistance: u8) -> u16 {
    let open = u32::from(100 - resistance.min(100));
    let through = (u32::from(amount) * open + 50) / 100;
    // `through` is at most `amount`, which fits.
    u16::try_from(through).unwrap_or(amount)
}

/// The poison in the player's blood, in hit points still to be lost.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Poison {
    left: u16,
}

impl Poison {
    /// The most poison a player can hold.
    pub const MAX: u16 = 999;

    /// A player holding `points`, never more than [`Poison::MAX`]; used to put a
    /// saved game back, after the save has been checked to be in range.
    #[must_use]
    pub fn holding(points: u16) -> Self {
        Self {
            left: points.min(Self::MAX),
        }
    }

    /// The points still in the blood.
    #[must_use]
    pub fn left(&self) -> u16 {
        self.left
    }

    /// Whether any poison is held.
    #[must_use]
    pub fn is_poisoned(&self) -> bool {
        self.left > 0
    }

    /// Takes in `amount` points after `resistance` percent of them are kept out, and
    /// returns how many got in. The total stops at [`Poison::MAX`], and what is
    /// returned counts only what was really added.
    pub fn absorb(&mut self, amount: u16, resistance: u8) -> u16 {
        let before = self.left;
        self.left = before
            .saturating_add(taken_in(amount, resistance))
            .min(Self::MAX);
        self.left - before
    }

    /// Takes out up to `amount` points, and returns how many came out.
    pub fn remove(&mut self, amount: u16) -> u16 {
        let before = self.left;
        self.left = before.saturating_sub(amount);
        before - self.left
    }

    /// Ends the poison, whatever its size.
    pub fn clear(&mut self) {
        self.left = 0;
    }

    /// Lets `minutes` pass, and returns how many points came out of the blood: at
    /// most [`DRAIN_PER_MINUTE`] for each minute, and never more than were held.
    pub fn elapse(&mut self, minutes: u64) -> u16 {
        let due = minutes.saturating_mul(u64::from(DRAIN_PER_MINUTE));
        let drained = u16::try_from(due).unwrap_or(u16::MAX).min(self.left);
        self.left -= drained;
        drained
    }
}
