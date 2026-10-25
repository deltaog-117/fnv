//! Benchmark stub for the combat hot path: a whole fight resolved attack by attack.
//!
//! Plays a four-fighter fight of sure hits to the end many times and prints the
//! median time per fight. Run with `cargo bench -p engine --bench combat`. Record
//! the number as the baseline; a regression above 5% should fail CI once a
//! comparing harness (for example criterion) replaces this stub.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use engine::combat::{
    AttackProfile, Combat, Combatant, Dice, FlatHitChance, Multiplier, Percent, Points, Power,
    Rules, Side, Status,
};

const RUNS: usize = 15;
const FIGHTS: u32 = 20_000;

struct Grunt;

impl Combatant for Grunt {
    fn max_hp(&self) -> NonZeroU32 {
        NonZeroU32::new(120).expect("120 is not zero")
    }
    fn max_ap(&self) -> u32 {
        65
    }
    fn dt(&self) -> Points {
        Points::try_new(5.0).expect("a valid threshold")
    }
    fn dr(&self) -> Points {
        Points::try_new(10.0).expect("a valid resistance")
    }
    fn crit_chance(&self) -> Percent {
        Percent::try_new(5).expect("a valid chance")
    }
    fn attack(&self) -> AttackProfile {
        AttackProfile {
            ap_cost: NonZeroU32::new(20).expect("20 is not zero"),
            base_damage: Points::try_new(26.0).expect("a valid damage"),
            crit_damage: Points::try_new(13.0).expect("a valid damage"),
            skill: Multiplier::FULL,
            condition: Multiplier::FULL,
            bonus: Points::ZERO,
        }
    }
}

/// A small linear congruential generator: cheap and repeatable, which is all a
/// benchmark needs from dice.
struct Lcg(u64);

impl Dice for Lcg {
    fn percent(&mut self) -> u8 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        u8::try_from((self.0 >> 33) % 100 + 1).unwrap_or(100)
    }
}

fn one_fight(dice: &mut Lcg) -> Status {
    let sides = [Side::Player, Side::Enemy, Side::Player, Side::Enemy];
    let mut combat = Combat::new(
        sides.map(|side| (side, Grunt)),
        Rules::standard(),
        FlatHitChance(Percent::ALWAYS),
    )
    .expect("both sides are present");
    let ids: Vec<_> = combat.ids().collect();
    while combat.status() == Status::Ongoing {
        let me = combat.side(combat.active());
        let foe = ids
            .iter()
            .find(|i| combat.side(**i) != me && combat.is_standing(**i));
        if foe.is_some_and(|foe| combat.attack(*foe, Power::Normal, dice).is_ok()) {
            continue;
        }
        if combat.end_turn().is_err() {
            break;
        }
    }
    combat.status()
}

fn median_run(dice: &mut Lcg) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..FIGHTS {
                // Keep the result alive so the call cannot be optimised away.
                std::hint::black_box(one_fight(dice));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let mut dice = Lcg(1);
    println!(
        "combat fight: {:?} per fight ({FIGHTS} per run)",
        median_run(&mut dice) / FIGHTS
    );
}
