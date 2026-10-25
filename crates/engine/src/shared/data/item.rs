//! Item records: weapons, armor, ammo and consumables.
//!
//! Each item file holds one kind, so each kind is its own struct. The fields
//! every kind shares are written once, in `item_record!`.

use serde::Deserialize;

use crate::shared::{AmmoFamilyId, Dlc, ItemId, Skill};

/// Which item file a record came from (`kind` in `item.schema.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// A weapon.
    Weapon,
    /// Armor or clothing.
    Armor,
    /// Ammunition.
    Ammo,
    /// Food, drink, chems and medicine.
    Consumable,
    /// A weapon modification.
    WeaponMod,
    /// A skill book or magazine.
    SkillBook,
    /// A crafting component or other miscellany.
    Misc,
}

/// How a weapon is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponClass {
    /// A single-shot firearm.
    Gun,
    /// A firearm that fires bursts.
    Automatic,
    /// A hand-loaded weapon.
    Handload,
    /// A melee weapon.
    Melee,
    /// An unarmed weapon such as knuckles.
    Unarmed,
    /// A thrown grenade.
    Grenade,
    /// A placed mine.
    Mine,
    /// Anything else.
    Misc,
}

/// Where armor is worn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorSlot {
    /// Body armor.
    Body,
    /// A helmet or hat.
    Head,
    /// Clothing without armor value.
    Clothing,
}

/// Armor weight class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorClass {
    /// Light armor.
    Light,
    /// Medium armor.
    Medium,
    /// Heavy armor.
    Heavy,
    /// Power armor.
    Power,
}

/// Defines an item struct with the fields every kind shares, followed by the
/// fields given for this kind.
macro_rules! item_record {
    (
        $(#[$meta:meta])*
        $name:ident {
            $($(#[$fmeta:meta])* $field:ident : $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// The item's unique id.
            pub id: ItemId,
            /// Display name.
            pub name: String,
            /// Which kind of item this is.
            pub kind: ItemKind,
            /// The game or add-on the item belongs to.
            pub dlc: Dlc,
            /// True for one-of-a-kind items.
            #[serde(default)]
            pub unique: bool,
            /// Base value in caps.
            pub value: f64,
            /// Weight in pounds.
            pub weight: f64,
            /// The game's editor id.
            pub edid: Option<String>,
            /// The game's form id, eight hex digits.
            pub form_id: Option<String>,
            /// Wiki page the record was imported from.
            pub source: Option<String>,
            $($(#[$fmeta])* pub $field: $ty,)*
        }
    };
}

item_record!(
    /// A weapon.
    Weapon {
        /// How the weapon is used.
        weapon_class: WeaponClass,
        /// The skill that governs the weapon.
        skill: Option<Skill>,
        /// Base damage per shot.
        damage: f64,
        /// Damage on a critical hit.
        crit_damage: Option<f64>,
        /// Multiplier on the critical chance.
        crit_chance_mult: Option<f64>,
        /// Shots per second.
        shots_per_sec: Option<f64>,
        /// Action points one attack costs.
        ap_cost: Option<f64>,
        /// Strength needed to handle it without penalty.
        strength_req: Option<u32>,
        /// Skill needed to handle it without penalty.
        skill_req: Option<u32>,
        /// Condition points when new.
        condition_hp: Option<f64>,
        /// Spread of the shot.
        spread: Option<f64>,
        /// Projectiles per shot.
        projectiles: Option<u32>,
        /// The ammo family it fires.
        ammo_family: Option<AmmoFamilyId>,
        /// The wiki's ammo name, when it did not resolve to a family.
        ammo_text: Option<String>,
        /// Rounds used per shot.
        ammo_per_shot: Option<f64>,
        /// Rounds per magazine.
        clip_size: Option<u32>,
        /// Seconds to reload.
        reload_time: Option<f64>,
        /// Splash damage of an explosive.
        explosion_damage: Option<f64>,
        /// The wiki's description of a special effect.
        effect_text: Option<String>,
        /// The wiki's note on repairing it.
        repair_text: Option<String>,
    }
);

item_record!(
    /// A piece of armor or clothing.
    Armor {
        /// Where it is worn.
        slot: ArmorSlot,
        /// Weight class.
        armor_class: Option<ArmorClass>,
        /// Damage threshold.
        dt: f64,
        /// Damage resistance.
        dr: Option<f64>,
        /// Condition points when new.
        condition_hp: Option<f64>,
        /// The wiki's description of its effects.
        effects_text: Option<String>,
        /// The wiki's note on repairing it.
        repair_text: Option<String>,
    }
);

item_record!(
    /// A type of ammunition.
    Ammo {
        /// The family weapons name in `ammo_family`.
        family: AmmoFamilyId,
        /// The load, for example "armor piercing".
        variant: Option<String>,
        /// Weight under Hardcore rules.
        weight_hardcore: Option<f64>,
    }
);

item_record!(
    /// Food, drink, a chem or medicine.
    Consumable {
        /// The wiki's description of its effects; the source of truth.
        effects_text: String,
        /// The wiki's description under Hardcore rules.
        effects_hardcore_text: Option<String>,
        /// Best-effort parse of simple "+N stat for T seconds" effects.
        #[serde(default)]
        effects: Vec<StatEffect>,
        /// Addiction risk, for chems.
        addiction: Option<Addiction>,
        /// The skill a skill-related consumable affects.
        skill: Option<String>,
    }
);

/// One parsed "+N stat" effect of a consumable.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatEffect {
    /// The stat's name as the wiki writes it.
    pub stat: String,
    /// The change.
    pub amount: f64,
    /// True when the change applies every second.
    #[serde(default)]
    pub per_second: bool,
    /// How long it lasts, in seconds; permanent when absent.
    pub duration_s: Option<f64>,
}

/// The addiction risk of a chem.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Addiction {
    /// Which addiction it causes.
    #[serde(rename = "type")]
    pub kind: String,
    /// Chance of addiction per use, in percent.
    pub chance_percent: Option<f64>,
}
