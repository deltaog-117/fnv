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

### Changed
- A quest condition must now carry a `stage` exactly when its state is `stage_at_least` or `stage_is`, and stage numbers cannot be negative (`schemas/common.schema.json`)

### Deprecated
- (None yet)

### Removed
- (None yet)

### Fixed
- (None yet)

### Security
- (None yet)
