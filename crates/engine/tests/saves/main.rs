//! Integration tests for save games, over the real Goodsprings data.
//!
//! `round_trip` plays (a scripted session and random ones), saves, loads and checks
//! the loaded game is the same game; `refusals` tampers with saves and checks each is
//! refused by part; `files` covers versions, the JSON form and the files on disk.

mod files;
mod refusals;
mod round_trip;
mod support;
