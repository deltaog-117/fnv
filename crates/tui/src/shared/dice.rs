//! The dice the front end rolls: a small generator seeded once per game.

use engine::combat::Dice;

/// A xorshift generator. Not for security: it only has to look random to a player
/// and be reproducible from its seed in tests.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator from `seed`. Zero would make xorshift stick at zero, so it is
    /// replaced by a fixed non-zero value.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// A generator seeded from the clock, so each game rolls differently.
    #[must_use]
    pub fn from_clock() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        // Folding the high half in keeps the fast-changing low bits and the rest.
        Self::new((nanos ^ (nanos >> 64)) as u64)
    }
}

impl Dice for Rng {
    fn percent(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        u8::try_from(self.0 % 100 + 1).unwrap_or(100)
    }
}

#[cfg(test)]
mod tests {
    use engine::combat::Dice;
    use proptest::prelude::*;

    // Named apart from the glob, which brings in proptest's own `Rng` trait.
    use super::Rng;

    proptest! {
        #[test]
        fn every_roll_is_from_one_to_one_hundred(seed in any::<u64>()) {
            let mut rng = Rng::new(seed);
            prop_assert!((0..200).all(|_| (1..=100).contains(&rng.percent())));
        }

        #[test]
        fn the_same_seed_rolls_the_same_dice(seed in any::<u64>()) {
            let (mut a, mut b) = (Rng::new(seed), Rng::new(seed));
            prop_assert!((0..50).all(|_| a.percent() == b.percent()));
        }
    }

    #[test]
    fn a_zero_seed_still_rolls_different_numbers() {
        let mut rng = Rng::new(0);
        let first = rng.percent();
        assert!((0..20).any(|_| rng.percent() != first));
    }
}
