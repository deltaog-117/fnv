# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Leveling in `crates/engine/src/character/`: skill points at every level-up, spent one at a time with `Character::spend_skill_point`; three tagged skills (+15); a perk pick every second level; up to two traits; `Character::create` for a starting character. Every skill stays between 0 and 100, and nothing is spent that was not paid out
- `PerkBook` (the perks and traits that exist, and who may take them) and `ProgressError`, `CreationError` and `Unmet`, which say why a spend, pick or creation is refused
- `Modifier` in `shared/modifier.rs` (skill points per level, XP percent, carry weight, hit points, action points, all skills, one skill), mirrored by `$defs/modifier` in `common.schema.json`; `Perk`, `Trait`, `PerkRequirements` and `TraitId` records; player-facing `Skill::name` and `Special::name`
- `data/perks/modifiers.json`, merged into the perk and trait records by `normalize.py`: Educated, Swift Learner, Strong Back, Action Boy/Girl and Life Giver, plus the traits Good Natured and Skilled. A perk or trait is listed only when all of its text is modeled; the others can be taken and change nothing yet
- `Game::with_perks`, `Game::perks`, `Game::spend_skill_point` and `Game::choose_perk`
- Tests: `tests/character/progression.rs`, `tests/game/progress.rs`, property tests over random level-ups, real-data checks against the perk and trait files, and contract tests that fail when the Rust types and the schema drift apart; character benchmarks
- Trudy's dialogue in the Goodsprings slice: the Cobb argument and the Ringo follow-ups, the NCR, Legion and Dam news, the robot talk, and a Speech 25 or Sneak 25 enlistment (the flag `trudy_joined_militia`) open from Sunny's agreement until Ringo's countdown, with the failed lines shown while the skill is short
- Dialogue scenarios take an optional `special` map, so a golden scenario can raise an attribute to pass a skill check; eight Trudy scenarios
- Ammunition in combat: a weapon with a `Magazine` (clip size, rounds per shot, reload cost, rounds carried) loses rounds with every shot, hit or miss; an empty weapon refuses the shot with `CombatError::OutOfAmmo` and rolls no dice. `Combat::reload` refills the clip from the reserve for action points, `Combat::rounds` reads the loaded and reserve counts, and `Combatant::magazine` defaults to none, so existing fighters are unaffected
- Game side: weapons take their clip, rounds per shot and ammo family from the item data; the player's rounds come from the pack (the plain load of the family first, then any other the pack holds), foes have an endless supply, and `finish_battle` takes the rounds fired out of the pack. `Battle::reload`, `Fighter::ammo_item` and `BattleError::Ammo`
- Foes reload when empty and keep shooting in the same turn; one with no rounds left passes
- TUI fight screen: a Reload action, the loaded and reserve rounds beside the player's hit points, and "out of ammo" on a locked Shoot row
- Tests: `tests/combat/ammo.rs` (clip, reserve, reload, refusals, enemy reloads, a property test that rounds are bounded, conserved and untouched by a refused move), game-level checks of the pack before, during and after a fight, and a TUI test of shooting and reloading
- Goodsprings quest chain in dialogue: Ringo's graph (offer help, then "Let's do this" at stage 60) and three new Sunny topics ("Let's go" at stage 65, the campfire lesson and the errand to Trudy), in `data/dialogue/slice_goodsprings.json`
- Nine golden dialogue scenarios in `tests/scenarios/goodsprings_dialogue.json` for the gunfight stages 5 to 65 and the campfire quest
- Combat wiring in `crates/engine/src/game/battle.rs`: `Arsenal` (checks every weapon, armor piece and encounter once at load), `Fighter` (a `Combatant` built from the character, the equipped weapon and armor, or an NPC record through the same formulas), `Battle`, `BattleError` and `Outcome`. Weapons with no action-point cost are left out, so they cannot be wielded in a fight
- `Game::hp`, `max_hp` via the character, `is_alive`, `heal`, `encounters_at`, `start_battle`, `finish_battle` and `Game::with_arsenal`. Wounds are stored as hit points lost, so hit points can never pass the maximum; settling a fight is atomic, so failing winning effects change nothing
- `combat::enemy_turns` (the enemy side shoots the first player-side fighter still standing, then passes) and `Combat::with_hp` (a fighter that starts wounded) with `CombatError::HpAboveMax`
- `Character::at_level`, so NPCs take action points and critical chance from the same formulas as the player
- Encounter data: `data/combat/encounters.json` (hand-authored, with the Ghost Town Gunfight), `schemas/encounter.schema.json`, an `Encounter` record and `EncounterId`, copied to `data/engine/encounters.json` by `normalize.py` and cross-checked (places, NPCs, weapons, armor, quests) by `validate.py`
- TUI combat screen (shoot a chosen foe or end the turn, with the result of every attack in the log), a game-over screen, a "Fight" row in the exploration menu while an encounter is on offer, and a clock-seeded dice source
- Tests: `enemy_turns` and `with_hp` cases plus a property test; the Goodsprings gunfight through `Game` over the real data (offer, fighters' numbers, win, loss, wounds, settle-before-over, a property test over random fights); headless TUI tests for the fight flow
- Character in `crates/engine/src/character/`: `Character` (S.P.E.C.I.A.L. plus XP earned) implementing `CharacterView` and `CharacterSink`. The level is derived from XP, so the two cannot disagree; hit points, action points, carry weight and each skill's starting value come from the curated formulas. `Level` (1 to 50) and `CharacterRules` (XP table, level cap, action-point cap)
- Inventory in `crates/engine/src/inventory/`: `ItemBook` (the 922 weapons, armor, ammo and consumables that exist) and `Inventory` (stacks, whole-number bottle caps, and weapon, outfit and head slots) implementing `InventoryView` and `InventorySink`. An equipped item is always carried; taking its last copy unequips it
- `Game::with_items`, `Game::with_character`, `Game::equip`, `Game::unequip`, `Game::is_encumbered`, `Game::character` and `Game::inventory`
- `shared::whole_amount`, which reads an XP or caps amount as a finite whole number
- Character and inventory tests: one case per rule, property tests (levels never fall as XP grows; the pack matches a plain model over random effects and a refusal changes nothing), real-data checks of the formulas, constants and XP table against `formulas.json` and `levels.json`, effects through `Game`, and two benchmark stubs
- Status line in the TUI shows the level, hit points and the bottle caps
- Raw game data imported from the Fallout Wiki, one importer per family (`scripts/fetch_*.py`): items, quests, NPCs, dialogue transcripts, locations and map markers, perks, traits, factions, reputation tables, creatures, stats, levels, challenges, skill books, recipes and ending slides
- Hand-curated quest consequence rules (`data/quests/consequences.json`) and engine formulas (`data/rules/formulas.json`)
- Twelve JSON Schemas in `schemas/`, including the shared `condition` / `effect` vocabulary
- `scripts/normalize.py`, which turns the raw data into engine-ready data in `data/engine/` with stable ids and cross-references, and a derived travel graph over the map markers
- `scripts/validate.py`, which validates every record against its schema, checks references between files, and self-tests the formulas against the wiki's worked examples
- Cargo workspace on edition 2024 with a headless `engine` library and a `tui` binary (`fnv-terminal`), organised by game capability
- Workspace lints: `unsafe` forbidden, `unwrap` denied outside tests
- `scripts/check`: one command that regenerates and validates the data, then runs rustfmt, clippy and the tests
- `ROADMAP.md` and a decision log in `DIARY.md`
- Typed data model in `crates/engine/src/shared/`: distinct id types, bounded skill and S.P.E.C.I.A.L. levels, a non-empty list, a `Redacted` log wrapper, and the `Condition` and `Effect` enums, with unit tests, property tests and two-way contract tests against `schemas/common.schema.json`
- Data loader in `crates/engine/src/shared/`: typed records for items, quests, NPCs, dialogue, factions and locations, and a `GameData` loader with a `Scope` (everywhere, or around one location such as Goodsprings). Errors name the file and the record id; duplicate ids and broken dialogue topic links are rejected
- New id types `LocationId`, `DialogueId`, `TopicKey` and `AmmoFamilyId`
- Loader tests against the real data and against deliberately broken data sets, and a load benchmark
- Condition/effect evaluator in `crates/engine/src/shared/`: `evaluate` for conditions, `apply` and `apply_all` for effects, six read-only view traits with default answers (`QuestView`, `StandingView`, `CharacterView`, `InventoryView`, `CompanionView`, `WorldView`), six write-side sink traits, `EmptyWorld`, `EffectError` and `BatchError`
- `apply_all` rejects a batch that overspends items or caps, or carries a NaN or infinite amount, before changing anything
- `Default` for `SkillLevel` and `SpecialLevel` (the lowest allowed value)
- Evaluator tests: examples for each condition, property tests for the laws of `all`/`any`/`not`, comparison ordering, effect round trips and batch behaviour, and an evaluation benchmark stub
- Quest engine in `crates/engine/src/quests/`: `QuestBook` (validated quest stages and rules) and `QuestLog` (player progress implementing `QuestView` and `QuestSink`). Quests go `not started -> active -> completed | failed`, finished quests never reopen, stages only move forward, and a `finish` stage completes the quest
- Quest tests: lifecycle cases, property tests over random effect sequences, a walk of every staged quest in the real data, and a playthrough benchmark stub
- Game state in `crates/engine/src/game/`: `Game` holds the quest log and implements all six view and six sink traits, so `evaluate` and `apply_all` run against it. Views of unbuilt features give the shared defaults; effects aimed at them are refused with a reason naming the missing feature
- Game tests: quest effects and conditions through `Game`, a property test for the refusals, a walk of every staged quest through `apply_all`, and a batch benchmark stub
- World state in `crates/engine/src/world/`: `WorldBook` (the NPC roster) and `WorldState` (story flags, NPC fates, ending locks) implementing `WorldView` and `WorldSink`. An NPC is alive (hostile or not), dead or removed, so a dead hostile NPC cannot exist; an NPC outside the roster is an error
- `Game::with_world` builds a game over a populated NPC roster, and `Game::meet` records meeting an NPC
- World tests: one case per rule, property tests over random effect sequences, world effects through `Game`, and a benchmark stub
- Goodsprings quest slice: `data/quests/slice_goodsprings.json` (hand-curated stage rewards and two rule gates) merged into `data/engine/quests.json` by `normalize.py`; stages gain an optional `on_enter` effect list in `quest.schema.json`
- `Game` applies a stage's `on_enter` effects the first time the stage is reached and fires a quest's consequence rules, once each, when the quest changes; a change that fails is undone as a whole
- `Game::deferred()` and `EffectError::NotBuilt`: rewards for unbuilt features (caps, XP, items) are kept instead of lost
- `validate.py` checks the quests, factions, items and NPCs named in authored conditions and effects
- Language-neutral golden scenarios in `tests/scenarios/goodsprings.json`, automation tests with a property test, and a plain-versus-automated benchmark
- Dialogue runner in `crates/engine/src/dialogue/`: `DialogueBook` (topic graph checked once: unique ids and keys, links resolve, an ending has no follow-ups) and `Conversation` (opens at the start topics; a topic with no follow-ups returns there; only topics whose conditions hold are offered; a failing `check` lists the option but locks it; a topic's effects apply as a unit or not at all; an ending topic closes the talk). It runs over any `State + Clone`, so it does not import `game`
- `start` (dialogue) and `ends` (topic) fields in `dialogue.schema.json`, the Rust records and the loader, which rejects a `start` link to a missing topic
- Goodsprings dialogue slice: `data/dialogue/slice_goodsprings.json` (hand-authored start menus, follow-ups, conditions and quest effects for Doc Mitchell and Sunny Smiles) merged into `data/engine/dialogue/` by `normalize.py`; `validate.py` checks its links and the quests and NPCs it names
- Dialogue tests: one case per rule, book rejection cases, a property test over random graphs and walks, the real data, a reachability check for authored topics, seven golden scenarios in `tests/scenarios/goodsprings_dialogue.json`, and a menu benchmark stub
- Combat core in `crates/engine/src/combat/`: `Combat` (a turn-based fight between two sides with action points, rounds and a win check), the `Combatant` trait (a fighter's numbers, read through a small trait), the damage pipeline from `formulas.json` (weapon damage, then resistance capped at 85, then threshold with a 20% floor), and bounded `Points`, `Multiplier` and `Percent` types. Hit chance sits behind the `HitModel` trait with a flat stand-in, and chance is rolled through a `Dice` trait, so a fight is deterministic for a given sequence of rolls
- Combat tests: one case per rule, property tests (armor never adds damage or blocks a fifth, more armor never lets more through, random play keeps the fight consistent, a fight of sure hits always ends), twelve golden scenarios in `tests/scenarios/combat.json`, a check of the rules against `formulas.json`, and a fight benchmark stub
- TUI v0 in `crates/tui/` (`fnv-terminal`, built on `ratatui`): a persistent layout (status line, scene, log, active-quest panel, key footer), an exploration screen listing who to talk to in Goodsprings, and a dialogue screen that drives the engine's `Conversation`. A new game starts the opening quest; locked topics are shown dimmed. Run it from the project root with `cargo run -p tui`
- TUI shared pieces: key-to-command mapping, a bounded menu cursor, a word-wrapped log, the layout split (with a "terminal too small" message below 70x22) and a Pip-Boy green theme
- TUI tests: property tests for the menu, the log and the layout, and headless `App` tests over the real Goodsprings data using ratatui's `TestBackend`

### Changed
- Doc's starting kit (`ain_t_that_a_kick_in_the_head`, stage 50) also gives 30 rounds of 9mm, since the pistol cannot shoot without them; `tests/scenarios/goodsprings.json` states them
- The test helper that plays a fight reloads when the weapon runs empty
- The Ghost Town Gunfight is offered at stage 65 ("Join up with Sunny") instead of stage 10, so it no longer opens as soon as Sunny agrees; the optional stages 20 to 50 are skipped until Trudy, Chet, Doc and Easy Pete are authored
- `GameData` also loads `encounters.json`, and a location-scoped load keeps the NPC records of the foes of encounters at that location. The exploration screen offers the fights on offer after the people; the player starts in leather armor and takes the 9mm pistol in hand when a fight starts (stand-ins until there is an inventory screen)
- XP, caps and item rewards now apply to the game instead of being postponed into `Game::deferred()`, which keeps only rewards for unbuilt features (reputation, karma, companions). A reward that names an item outside the item book fails and the whole change is undone
- `tests/scenarios/goodsprings.json` states the caps, XP and items each scenario must leave behind instead of listing deferred effects
- `Game` now applies flag, NPC and ending effects instead of refusing them; `Game::new` has an empty roster, so NPC effects on it fail with an unknown-NPC error
- Effects aimed at features that are not built now fail with `EffectError::NotBuilt` instead of `Refused`
- A quest condition must now carry a `stage` exactly when its state is `stage_at_least` or `stage_is`, and stage numbers cannot be negative (`schemas/common.schema.json`)

### Deprecated
- (None yet)

### Removed
- (None yet)

### Fixed
- The TUI log wrapping property test generated words that cannot fit beside the two-column hanging indent of the narrowest pane, so it failed on rare inputs; words are now at most 6 letters there, and a unit test pins that a longer word is split without losing letters

### Security
- (None yet)
