//! Benchmark stub for the numbers the character derives on every read: level,
//! hit points and a skill, from XP and S.P.E.C.I.A.L.
//!
//! Reads them from a character at several XP totals many times and prints the
//! median time per read of all three. Run with
//! `cargo bench -p engine --bench character`. Record the number as the baseline; a
//! regression above 5% should fail CI once a comparing harness (for example
//! criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::character::Character;
use engine::shared::{CharacterView, Skill};

const RUNS: usize = 15;
const READS: u32 = 200_000;

fn median_run() -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for read in 0..READS {
                let mut character = Character::default();
                character.grant_xp(read);
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box((
                    character.level(),
                    character.max_hp(),
                    character.skill(Skill::Guns),
                ));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "character reads: {:?} per level + hit points + skill ({READS} per run)",
        median_run() / READS
    );
}
