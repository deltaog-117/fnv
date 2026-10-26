//! Benchmark stub for the numbers the character derives on every read: level,
//! hit points and a skill, from XP and S.P.E.C.I.A.L., and for the work an XP
//! award does: scaling it, paying out the levels it crosses, and a skill point
//! spent.
//!
//! Reads them from a character at several XP totals many times and prints the
//! median time per read of all three, then the median time per award and spend. Run with
//! `cargo bench -p engine --bench character`. Record the number as the baseline; a
//! regression above 5% should fail CI once a comparing harness (for example
//! criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::character::Character;
use engine::shared::{CharacterView, Skill};

const RUNS: usize = 15;
const READS: u32 = 200_000;

fn median_run() -> Duration {
    // Built outside the timed loop: this measures the reads, not the awards. Few
    // enough to stay in cache, as the one character a game reads is.
    let characters: Vec<Character> = (0..32_u32)
        .map(|n| {
            let mut character = Character::default();
            character.grant_xp(n * 6_000);
            character
        })
        .collect();
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for read in 0..READS {
                let character = &characters[read as usize % characters.len()];
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

/// Awards per run of the second measurement: a character climbs from level 1 to
/// the cap, spending a point after each award.
const AWARDS: u32 = 30;
const CLIMBS: u32 = 5_000;

fn median_climb() -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..CLIMBS {
                let mut character = Character::default();
                for _ in 0..AWARDS {
                    character.grant_xp(std::hint::black_box(400));
                    // Refusals are part of the path: the pack runs dry at times.
                    std::hint::black_box(character.spend_skill_point(Skill::Guns).is_ok());
                }
                std::hint::black_box(character.skill_points());
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

/// Nanoseconds per operation with a decimal: the reads take about twenty, and a
/// whole-nanosecond figure would hide a regression of a few percent.
fn per_op(total: Duration, operations: u32) -> f64 {
    total.as_secs_f64() * 1e9 / f64::from(operations)
}

fn main() {
    println!(
        "character reads: {:.1} ns per level + hit points + skill ({READS} per run)",
        per_op(median_run(), READS)
    );
    println!(
        "character growth: {:.1} ns per XP award + skill point spent ({} per run)",
        per_op(median_climb(), CLIMBS * AWARDS),
        CLIMBS * AWARDS
    );
}
