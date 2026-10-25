//! A list that always holds at least one element.

use serde::{Deserialize, Serialize};

use super::error::ModelError;

/// A `Vec` that cannot be empty, used where an empty list would be meaningless
/// (for example an `all` condition with nothing to check).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "Vec<T>",
    into = "Vec<T>",
    bound(
        serialize = "T: Clone + Serialize",
        deserialize = "T: Deserialize<'de>"
    )
)]
pub struct NonEmpty<T>(Vec<T>);

impl<T> NonEmpty<T> {
    /// Builds the list from a `Vec`.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::Empty`] when `items` is empty.
    pub fn try_new(items: Vec<T>) -> Result<Self, ModelError> {
        if items.is_empty() {
            Err(ModelError::Empty { what: "list" })
        } else {
            Ok(Self(items))
        }
    }

    /// The elements, in order.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// Iterates over the elements.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }
}

impl<T> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = ModelError;

    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        Self::try_new(items)
    }
}

impl<T> From<NonEmpty<T>> for Vec<T> {
    fn from(list: NonEmpty<T>) -> Self {
        list.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_list() {
        assert!(NonEmpty::<u8>::try_new(Vec::new()).is_err());
    }

    #[test]
    fn keeps_order() {
        let list = NonEmpty::try_new(vec![3, 1, 2]).unwrap();
        assert_eq!(list.as_slice(), [3, 1, 2]);
    }
}
