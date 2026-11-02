//! How the map is built from location records.

use engine::travel::{TravelBook, TravelError};

use crate::support::{id, line, location};

#[test]
fn a_link_written_on_one_record_works_in_both_directions() {
    let book = TravelBook::new(&line()).expect("valid");
    assert_eq!(
        book.span_between(&id("a"), &id("b")),
        book.span_between(&id("b"), &id("a"))
    );
    assert!(book.span_between(&id("a"), &id("b")).is_some());
}

#[test]
fn two_places_with_no_link_have_no_walk_between_them() {
    let book = TravelBook::new(&line()).expect("valid");
    assert_eq!(book.span_between(&id("a"), &id("c")), None);
}

#[test]
fn when_both_records_write_the_link_the_shorter_distance_wins() {
    let records = [location("a", &[("b", 50.0)]), location("b", &[("a", 10.0)])];
    let book = TravelBook::new(&records).expect("valid");
    let shorter =
        TravelBook::new(&[location("a", &[("b", 10.0)]), location("b", &[])]).expect("valid");
    assert_eq!(
        book.span_between(&id("a"), &id("b")),
        shorter.span_between(&id("a"), &id("b"))
    );
}

#[test]
fn routes_come_nearest_first() {
    let records = [
        location("hub", &[("far", 90.0), ("near", 5.0), ("mid", 40.0)]),
        location("far", &[]),
        location("mid", &[]),
        location("near", &[]),
    ];
    let book = TravelBook::new(&records).expect("valid");
    let order: Vec<String> = book
        .routes_from(&id("hub"))
        .into_iter()
        .map(|route| route.to.to_string())
        .collect();
    assert_eq!(order, ["near", "mid", "far"]);
}

#[test]
fn a_route_carries_the_name_the_player_sees() {
    let book = TravelBook::new(&line()).expect("valid");
    let routes = book.routes_from(&id("a"));
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].name, "B");
}

#[test]
fn an_unknown_place_has_no_routes_and_no_name() {
    let book = TravelBook::new(&line()).expect("valid");
    assert!(book.routes_from(&id("nowhere")).is_empty());
    assert_eq!(book.name(&id("nowhere")), None);
}

#[test]
fn a_link_to_a_place_that_is_not_there_is_refused() {
    let records = [location("a", &[("ghost", 5.0)])];
    assert_eq!(
        TravelBook::new(&records).err(),
        Some(TravelError::UnknownPlace {
            from: id("a"),
            to: id("ghost")
        })
    );
}

#[test]
fn a_place_that_links_to_itself_is_refused() {
    let records = [location("a", &[("a", 5.0)])];
    assert_eq!(
        TravelBook::new(&records).err(),
        Some(TravelError::SelfLink(id("a")))
    );
}

#[test]
fn a_distance_that_is_not_above_zero_is_refused() {
    for bad in [0.0, -3.0] {
        let records = [location("a", &[("b", bad)]), location("b", &[])];
        assert!(
            matches!(
                TravelBook::new(&records).err(),
                Some(TravelError::BadDistance { .. })
            ),
            "{bad}"
        );
    }
}

#[test]
fn a_repeated_id_is_refused() {
    let records = [location("a", &[]), location("a", &[])];
    assert_eq!(
        TravelBook::new(&records).err(),
        Some(TravelError::DuplicatePlace(id("a")))
    );
}
