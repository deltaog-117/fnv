//! The Barter formulas, pinned to the curated file.

use engine::shared::SkillLevel;
use engine::trade::{buy_multiplier, buy_total, sell_multiplier, sell_total};
use proptest::prelude::*;

use crate::support::count;

fn barter(level: u32) -> SkillLevel {
    SkillLevel::try_new(level).expect("in range")
}

#[test]
fn the_anchor_points_of_the_table() {
    // 155% to buy and 45% to sell at Barter 0, 110% and 90% at Barter 100.
    assert_eq!(buy_total(100.0, count(1), barter(0)), 155);
    assert_eq!(sell_total(100.0, count(1), barter(0)), 45);
    assert_eq!(buy_total(100.0, count(1), barter(100)), 110);
    assert_eq!(sell_total(100.0, count(1), barter(100)), 90);
}

#[test]
fn the_table_rows_that_are_not_typos() {
    // Buy 146, 137, 128, 119 and sell 54, 63, 72 at Barter 20, 40, 60 and 80.
    for (level, buy, sell) in [(20, 146, 54), (40, 137, 63), (60, 128, 72)] {
        assert_eq!(buy_total(100.0, count(1), barter(level)), buy, "{level}");
        assert_eq!(sell_total(100.0, count(1), barter(level)), sell, "{level}");
    }
    assert_eq!(buy_total(100.0, count(1), barter(80)), 119);
}

#[test]
fn a_line_is_rounded_once_not_per_item() {
    // 10 rounds worth 1 cap at Barter 0 sell for 4.5 caps, which rounds to 5;
    // rounding each round first would pay nothing.
    assert_eq!(sell_total(1.0, count(1), barter(0)), 0);
    assert_eq!(sell_total(1.0, count(10), barter(0)), 5);
}

#[test]
fn buying_costs_at_least_one_cap() {
    assert_eq!(buy_total(0.0, count(1), barter(100)), 1);
}

#[test]
fn the_constants_are_the_formulas_in_the_file() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine/formulas.json");
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("file exists"))
            .expect("file parses");
    let expression = |id: &str| {
        file["formulas"]
            .as_array()
            .expect("formulas is a list")
            .iter()
            .find(|f| f["id"] == id)
            .unwrap_or_else(|| panic!("formula {id} exists"))["expression"]
            .clone()
    };
    assert_eq!(expression("barter_buy_multiplier"), "1.55 - 0.0045*barter");
    assert_eq!(expression("barter_sell_multiplier"), "0.45 + 0.0045*barter");
    assert!((buy_multiplier(barter(0)) - 1.55).abs() < 1e-9);
    assert!((sell_multiplier(barter(0)) - 0.45).abs() < 1e-9);
}

proptest! {
    #[test]
    fn more_barter_never_costs_more_or_pays_less(
        value in 0.0_f64..10_000.0, n in 1_u32..200, low in 0_u32..=100, extra in 0_u32..=100,
    ) {
        let high = (low + extra).min(100);
        let (low, high) = (barter(low), barter(high));
        prop_assert!(buy_total(value, count(n), high) <= buy_total(value, count(n), low));
        prop_assert!(sell_total(value, count(n), high) >= sell_total(value, count(n), low));
    }

    #[test]
    fn a_vendor_never_pays_more_than_it_charges(
        value in 0.0_f64..10_000.0, n in 1_u32..200, level in 0_u32..=100,
    ) {
        let level = barter(level);
        prop_assert!(sell_total(value, count(n), level) <= buy_total(value, count(n), level));
    }
}
