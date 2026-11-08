//! The rads held, and the sickness they cause.

use crate::shared::{BoostStat, BoostTally, Rads, Special};

/// Every this many rads make the player one tier sicker. A stand-in: the wiki's
/// sickness table was not imported.
pub const RADS_PER_TIER: u16 = 200;

/// How many rads get in past `resistance` percent, rounded to the nearest rad.
/// Resistance above 100 counts as 100, so it never lets a negative number in.
#[must_use]
pub fn absorbed(amount: u16, resistance: u8) -> u16 {
    let open = u32::from(100 - resistance.min(100));
    let through = (u32::from(amount) * open + 50) / 100;
    // `through` is at most `amount`, which fits.
    u16::try_from(through).unwrap_or(amount)
}

/// The radiation the player has absorbed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Radiation {
    rads: Rads,
}

impl Radiation {
    /// A player holding `rads`; used to put a saved game back.
    #[must_use]
    pub fn holding(rads: Rads) -> Self {
        Self { rads }
    }

    /// The rads held now.
    #[must_use]
    pub fn rads(&self) -> Rads {
        self.rads
    }

    /// Takes in `amount` rads after `resistance` percent of them are kept out, and
    /// returns how many got in. The total stops at [`Rads::MAX`], and what is
    /// returned counts only what was really added.
    pub fn absorb(&mut self, amount: u16, resistance: u8) -> u16 {
        let before = self.rads;
        self.rads = before.saturating_add(absorbed(amount, resistance));
        self.rads.get() - before.get()
    }

    /// Takes out up to `amount` rads, and returns how many came out.
    pub fn remove(&mut self, amount: u16) -> u16 {
        let before = self.rads;
        self.rads = before.saturating_sub(amount);
        before.get() - self.rads.get()
    }

    /// Takes out every rad.
    pub fn clear(&mut self) {
        self.rads = Rads::NONE;
    }

    /// How sick the player is: 0 is well, and each [`RADS_PER_TIER`] rads add one.
    #[must_use]
    pub fn tier(&self) -> u8 {
        u8::try_from(self.rads.get() / RADS_PER_TIER).unwrap_or(u8::MAX)
    }

    /// What the sickness takes away: one Endurance and one Agility for every tier
    /// (a stand-in for the wiki's table).
    #[must_use]
    pub fn sickness(&self) -> BoostTally {
        let tier = -i16::from(self.tier());
        let mut tally = BoostTally::default();
        if tier != 0 {
            tally.add(BoostStat::Special(Special::Endurance), tier);
            tally.add(BoostStat::Special(Special::Agility), tier);
        }
        tally
    }
}
