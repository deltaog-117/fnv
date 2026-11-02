//! However the player walks, time only moves forward by the walks taken, and a
//! refused trip changes nothing.

use engine::shared::Location;
use proptest::prelude::*;

use crate::support::{game, id, location};

const NAMES: [&str; 6] = ["p0", "p1", "p2", "p3", "p4", "p5"];

/// Six places with random links of random lengths (one record per link).
fn world() -> impl Strategy<Value = Vec<Location>> {
    prop::collection::vec((0..NAMES.len(), 0..NAMES.len(), 0.5_f64..200.0), 0..14).prop_map(
        |links| {
            NAMES
                .iter()
                .enumerate()
                .map(|(from, name)| {
                    let out: Vec<(&str, f64)> = links
                        .iter()
                        .filter(|(a, b, _)| *a == from && *b != from)
                        .map(|(_, b, d)| (NAMES[*b], *d))
                        .collect();
                    location(name, &out)
                })
                .collect()
        },
    )
}

proptest! {
    #[test]
    fn a_walk_moves_time_by_exactly_the_trips_taken(
        records in world(), trips in prop::collection::vec(0..NAMES.len(), 0..40),
    ) {
        let mut game = game(&records, "p0");
        let mut expected = game.clock().as_minutes();
        for target in trips {
            let before = game.clone();
            match game.travel(&id(NAMES[target])) {
                Ok(span) => {
                    expected += span.as_minutes();
                    prop_assert_eq!(game.at(), Some(&id(NAMES[target])));
                }
                Err(_) => {
                    prop_assert_eq!(game.at(), before.at());
                    prop_assert_eq!(game.clock(), before.clock());
                }
            }
            prop_assert_eq!(game.clock().as_minutes(), expected);
        }
    }

    #[test]
    fn every_offered_route_can_be_walked_and_walked_back(records in world()) {
        let game = game(&records, "p0");
        for route in game.routes() {
            let mut trial = game.clone();
            let there = trial.travel(&route.to).expect("an offered route can be walked");
            prop_assert_eq!(there, route.span);
            let back = trial.travel(&id("p0")).expect("every link is two-way");
            prop_assert_eq!(back, there, "the walk back takes as long");
        }
    }
}
