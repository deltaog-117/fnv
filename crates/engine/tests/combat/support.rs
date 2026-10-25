//! A plain fighter, scripted dice and a fight builder shared by the tests.

use std::num::NonZeroU32;

use engine::combat::{
    AttackProfile, Combat, Combatant, Dice, FlatHitChance, Magazine, Multiplier, Percent, Points,
    Rules, Side,
};

/// A fighter made of plain numbers, the way a test (or a monster) would be.
#[derive(Debug, Clone)]
pub struct Plain {
    pub hp: u32,
    pub ap: u32,
    pub dt: f64,
    pub dr: f64,
    pub crit: u32,
    pub damage: f64,
    pub crit_damage: f64,
    pub ap_cost: u32,
    pub magazine: Option<Magazine>,
}

impl Plain {
    /// A fighter with `hp` hit points and a weapon of `damage` that costs 20 AP of 65.
    pub fn new(hp: u32, damage: f64) -> Self {
        Self {
            hp,
            ap: 65,
            dt: 0.0,
            dr: 0.0,
            crit: 0,
            damage,
            crit_damage: 0.0,
            ap_cost: 20,
            magazine: None,
        }
    }

    /// A weapon that holds `clip` rounds, uses one per shot, costs 5 AP to reload,
    /// and has `stock` rounds in all (`None` is endless).
    pub fn rounds(mut self, clip: u32, stock: Option<u32>) -> Self {
        self.magazine = Some(Magazine {
            clip: NonZeroU32::new(clip).expect("tests give a clip"),
            per_shot: NonZeroU32::MIN,
            reload_ap: NonZeroU32::new(5).expect("literal is not zero"),
            stock,
        });
        self
    }

    pub fn armor(mut self, dt: f64, dr: f64) -> Self {
        self.dt = dt;
        self.dr = dr;
        self
    }

    pub fn crit(mut self, chance: u32, damage: f64) -> Self {
        self.crit = chance;
        self.crit_damage = damage;
        self
    }

    pub fn ap(mut self, ap: u32, cost: u32) -> Self {
        self.ap = ap;
        self.ap_cost = cost;
        self
    }
}

impl Combatant for Plain {
    fn max_hp(&self) -> NonZeroU32 {
        NonZeroU32::new(self.hp).expect("tests give every fighter hit points")
    }

    fn max_ap(&self) -> u32 {
        self.ap
    }

    fn dt(&self) -> Points {
        Points::try_new(self.dt).expect("valid test threshold")
    }

    fn dr(&self) -> Points {
        Points::try_new(self.dr).expect("valid test resistance")
    }

    fn crit_chance(&self) -> Percent {
        Percent::try_new(self.crit).expect("valid test crit chance")
    }

    fn attack(&self) -> AttackProfile {
        AttackProfile {
            ap_cost: NonZeroU32::new(self.ap_cost).expect("tests give attacks a cost"),
            base_damage: Points::try_new(self.damage).expect("valid test damage"),
            crit_damage: Points::try_new(self.crit_damage).expect("valid test crit damage"),
            skill: Multiplier::FULL,
            condition: Multiplier::FULL,
            bonus: Points::ZERO,
        }
    }

    fn magazine(&self) -> Option<Magazine> {
        self.magazine
    }
}

/// Dice that play back a fixed list of rolls and say so when it runs out.
pub struct Rolls {
    rolls: Vec<u8>,
    at: usize,
}

impl Rolls {
    pub fn new(rolls: &[u8]) -> Self {
        Self {
            rolls: rolls.to_vec(),
            at: 0,
        }
    }

    pub fn left(&self) -> usize {
        self.rolls.len() - self.at
    }
}

impl Dice for Rolls {
    fn percent(&mut self) -> u8 {
        let roll = *self
            .rolls
            .get(self.at)
            .expect("the test scripted enough rolls");
        self.at += 1;
        roll
    }
}

/// Dice that repeat a list of rolls forever, for random fights.
pub struct Cycle {
    rolls: Vec<u8>,
    at: usize,
}

impl Cycle {
    pub fn new(rolls: Vec<u8>) -> Self {
        assert!(!rolls.is_empty(), "a cycle needs at least one roll");
        Self { rolls, at: 0 }
    }
}

impl Dice for Cycle {
    fn percent(&mut self) -> u8 {
        let roll = self.rolls[self.at % self.rolls.len()];
        self.at += 1;
        roll
    }
}

pub fn percent(value: u32) -> Percent {
    Percent::try_new(value).expect("valid test percent")
}

/// A fight with the standard rules where every attack has `hit` percent to land.
pub fn fight(fighters: Vec<(Side, Plain)>, hit: u32) -> Combat<Plain, FlatHitChance> {
    Combat::new(fighters, Rules::standard(), FlatHitChance(percent(hit)))
        .expect("the test gave each side a fighter")
}

/// One player against one enemy.
pub fn duel(player: Plain, enemy: Plain, hit: u32) -> Combat<Plain, FlatHitChance> {
    fight(vec![(Side::Player, player), (Side::Enemy, enemy)], hit)
}
