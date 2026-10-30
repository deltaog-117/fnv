//! How many units of a stack one trade moves.

use std::num::NonZeroU32;

/// A count the player is choosing, from one up to the size of the stack.
///
/// Both bounds are private and every method keeps the value between them, so the
/// screen cannot ask the engine to move nothing or more than the stack holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity {
    value: NonZeroU32,
    max: NonZeroU32,
}

impl Quantity {
    /// One unit of a stack of `max`.
    #[must_use]
    pub fn new(max: NonZeroU32) -> Self {
        Self {
            value: NonZeroU32::MIN,
            max,
        }
    }

    /// The chosen count.
    #[must_use]
    pub fn get(self) -> NonZeroU32 {
        self.value
    }

    /// The size of the stack.
    #[must_use]
    pub fn max(self) -> NonZeroU32 {
        self.max
    }

    /// Adds `by`, stopping at the whole stack.
    pub fn raise(&mut self, by: u32) {
        let raised = self.value.get().saturating_add(by).min(self.max.get());
        self.value = NonZeroU32::new(raised).unwrap_or(NonZeroU32::MIN);
    }

    /// Takes away `by`, stopping at one.
    pub fn lower(&mut self, by: u32) {
        let lowered = self.value.get().saturating_sub(by).max(1);
        self.value = NonZeroU32::new(lowered).unwrap_or(NonZeroU32::MIN);
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn the_count_never_leaves_one_to_the_stack_size(
            max in 1u32..500,
            steps in proptest::collection::vec((any::<bool>(), 0u32..u32::MAX), 0..40),
        ) {
            let max = NonZeroU32::new(max).expect("at least one");
            let mut quantity = Quantity::new(max);
            for (up, by) in steps {
                if up {
                    quantity.raise(by);
                } else {
                    quantity.lower(by);
                }
                prop_assert!(quantity.get() <= quantity.max());
                prop_assert_eq!(quantity.max(), max);
            }
        }
    }

    #[test]
    fn raising_past_the_stack_stops_at_the_stack() {
        let mut quantity = Quantity::new(NonZeroU32::new(5).expect("nonzero"));
        quantity.raise(10);
        assert_eq!(quantity.get().get(), 5);
        quantity.lower(100);
        assert_eq!(quantity.get().get(), 1);
    }
}
