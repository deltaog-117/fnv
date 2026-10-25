//! A wrapper that hides a value from logs.

use std::fmt;

/// Holds a value whose `Debug` and `Display` output is masked.
///
/// Log fields that may contain personal or sensitive data (a player's save path,
/// a free-text name) are wrapped in this type, so forgetting to redact is a type
/// error rather than a review finding. Reading the value takes a deliberate call
/// to [`expose`](Self::expose).
#[derive(Clone, PartialEq, Eq)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    /// Wraps `value`.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// The wrapped value. Never pass the result to a logging macro.
    #[must_use]
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_never_shows_the_value() {
        let secret = Redacted::new("player-name-123");
        assert_eq!(format!("{secret}"), "[redacted]");
        assert_eq!(format!("{secret:?}"), "[redacted]");
        assert_eq!(*secret.expose(), "player-name-123");
    }
}
