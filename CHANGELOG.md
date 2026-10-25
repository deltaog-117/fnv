# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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

### Changed
- `Game` now applies flag, NPC and ending effects instead of refusing them; `Game::new` has an empty roster, so NPC effects on it fail with an unknown-NPC error
- Effects aimed at features that are not built now fail with `EffectError::NotBuilt` instead of `Refused`
- A quest condition must now carry a `stage` exactly when its state is `stage_at_least` or `stage_is`, and stage numbers cannot be negative (`schemas/common.schema.json`)

### Deprecated
- (None yet)

### Removed
- (None yet)

### Fixed
- (None yet)

### Security
- (None yet)
