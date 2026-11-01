//! Builders shared by the reputation tests.

use std::sync::Arc;

use engine::reputation::{ReputationBook, Standing};
use engine::shared::{Faction, FactionId, RenownTier};
use serde_json::{Value, json};

pub fn id(text: &str) -> FactionId {
    FactionId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn tier(value: u32) -> RenownTier {
    RenownTier::try_new(value).expect("literal is a valid tier")
}

/// The wiki's grid, indexed `[infamy tier][fame tier]`.
pub fn grid() -> Value {
    json!([
        ["Neutral", "Accepted", "Liked", "Idolized"],
        [
            "Shunned",
            "Mixed",
            "Smiling Troublemaker",
            "Good-Natured Rascal"
        ],
        ["Hated", "Sneering Punk", "Unpredictable", "Dark Hero"],
        [
            "Vilified",
            "Merciful Thug",
            "Soft-Hearted Devil",
            "Wild Child"
        ]
    ])
}

/// A town with a reputation model.
pub fn modeled(name: &str, thresholds: [u32; 4]) -> Faction {
    serde_json::from_value(json!({
        "id": name, "name": name, "kind": "town", "dlc": "base",
        "reputation": {"thresholds": thresholds, "titles": grid()},
    }))
    .expect("the literal is a valid faction record")
}

/// A faction that tracks no reputation.
pub fn plain(name: &str) -> Faction {
    serde_json::from_value(json!({"id": name, "name": name, "kind": "faction", "dlc": "base"}))
        .expect("the literal is a valid faction record")
}

/// `town` has tiers at 3, 8 and 15 points (Goodsprings' own); `gang` is a faction
/// without a model.
pub fn book() -> Arc<ReputationBook> {
    let records = [modeled("town", [0, 3, 8, 15]), plain("gang")];
    Arc::new(ReputationBook::new(&records).expect("the records build"))
}

pub fn standing() -> Standing {
    Standing::new(book())
}
