//! What the player calls each item: the engine knows items by id only.

use std::collections::BTreeMap;

use engine::shared::{GameData, ItemId};

/// Display names for the weapons, armor, ammo and consumables of the loaded data.
#[derive(Debug, Clone, Default)]
pub struct Names(BTreeMap<ItemId, String>);

impl Names {
    /// The names of every item record in `data`.
    #[must_use]
    pub fn from_data(data: &GameData) -> Self {
        let weapons = data.weapons.iter().map(|i| (i.id.clone(), i.name.clone()));
        let armor = data.armor.iter().map(|i| (i.id.clone(), i.name.clone()));
        let ammo = data.ammo.iter().map(|i| (i.id.clone(), i.name.clone()));
        let consumables = data
            .consumables
            .iter()
            .map(|i| (i.id.clone(), i.name.clone()));
        Self(
            weapons
                .chain(armor)
                .chain(ammo)
                .chain(consumables)
                .collect(),
        )
    }

    /// The item's name, or its id when the data has no record for it, so a row is
    /// never blank.
    #[must_use]
    pub fn name(&self, item: &ItemId) -> String {
        self.0
            .get(item)
            .cloned()
            .unwrap_or_else(|| item.as_str().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_without_a_record_is_named_by_its_id() {
        let id = ItemId::try_new("mystery_box").expect("valid id");
        assert_eq!(Names::default().name(&id), "mystery_box");
    }
}
