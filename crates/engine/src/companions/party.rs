//! The people travelling with the player.

use thiserror::Error;

use crate::shared::CompanionId;

/// How many companions travel with the player at once. The real game allows two
/// (one person and one robot or creature); this stand-in does not tell them apart.
pub const PARTY_LIMIT: usize = 2;

/// Why a change to the party was refused. The party is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PartyError {
    /// The party already has [`PARTY_LIMIT`] people.
    #[error("the party is full")]
    Full,
    /// The person is in the party already.
    #[error("{0} is already in the party")]
    AlreadyIn(CompanionId),
    /// The person is not in the party.
    #[error("{0} is not in the party")]
    NotIn(CompanionId),
}

/// The companions with the player, in the order they joined.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Party {
    members: Vec<CompanionId>,
}

impl Party {
    /// Puts back a party read from a save, checking the same laws a join does.
    ///
    /// # Errors
    ///
    /// [`PartyError`] for a repeat or for more than [`PARTY_LIMIT`] people.
    pub fn from_members(
        members: impl IntoIterator<Item = CompanionId>,
    ) -> Result<Self, PartyError> {
        let mut party = Self::default();
        for member in members {
            party.join(&member)?;
        }
        Ok(party)
    }

    /// Adds a person.
    ///
    /// # Errors
    ///
    /// [`PartyError::AlreadyIn`] or [`PartyError::Full`].
    pub fn join(&mut self, who: &CompanionId) -> Result<(), PartyError> {
        if self.contains(who) {
            return Err(PartyError::AlreadyIn(who.clone()));
        }
        if self.members.len() >= PARTY_LIMIT {
            return Err(PartyError::Full);
        }
        self.members.push(who.clone());
        Ok(())
    }

    /// Takes a person out.
    ///
    /// # Errors
    ///
    /// [`PartyError::NotIn`] when they are not there.
    pub fn leave(&mut self, who: &CompanionId) -> Result<(), PartyError> {
        let place = self
            .members
            .iter()
            .position(|member| member == who)
            .ok_or_else(|| PartyError::NotIn(who.clone()))?;
        self.members.remove(place);
        Ok(())
    }

    /// Whether the person is in the party.
    #[must_use]
    pub fn contains(&self, who: &CompanionId) -> bool {
        self.members.contains(who)
    }

    /// The people in the party, in the order they joined.
    #[must_use]
    pub fn members(&self) -> &[CompanionId] {
        &self.members
    }

    /// Whether nobody is with the player.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}
