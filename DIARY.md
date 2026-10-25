# 📘 DIARY.md

## Architectural Decision Log

*A chronological record of decisions, trade-offs and reasoning for fnv-terminal-ai-game. Just as Git tracks code changes, this diary tracks the **why** behind the code.*

---

## 📋 Decision Index

| Date | Decision Area | Choice | Status |
|------|---------------|--------|--------|
| 2026-10-25 | Data source | Fallout Wiki import now; game-file export later | ✅ Confirmed |
| 2026-10-25 | Language and stack | Rust + ratatui (engine/TUI), Python (data tooling) | ✅ Confirmed |
| 2026-10-25 | Scope of the first playable | Goodsprings vertical slice | ✅ Confirmed |
| 2026-10-25 | Engine architecture | Shared contracts + feature-owned state (COA C) | ✅ Confirmed |
| 2026-10-25 | Quality gate | One `scripts/check`, data steps run on change | ✅ Confirmed |
| 2026-10-25 | Rust types and the schema | Schema is the contract; hand-written types checked by contract tests | ✅ Confirmed |
| 2026-10-25 | Data loader | Per-record typed parsing with a location scope | ✅ Confirmed |

---

## 📝 Decision Entries

### Data Source: Fallout Wiki Now, Game Files Later

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The game needs thousands of items, quests, NPCs and dialogue lines before any engine code is useful. Typing them by hand is the largest cost of the whole project, and the user did not have the game installed when planning started.

---

#### Options Considered

**Option A: Import from the Fallout Wiki through its API**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Available immediately <br> • Infoboxes and tables are mostly structured <br> • Cheap: the whole import used well under 1% of the working budget |
| **Disadvantages** | • No quest scripts, dialogue links or skill-check thresholds <br> • Prose notes had to be read and curated by hand <br> • Some pages are messy (recipes, faction infoboxes) |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Good enough to start; cannot supply the game logic itself |

**Option B: Export the game's own data (`FalloutNV.esm` and the DLC files) with xEdit**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • The only complete source: quest stages and conditions, dialogue conditions and links, loot, vendors, game settings <br> • More exact than any wiki |
| **Disadvantages** | • Needs the user's copy of the game, which was unavailable <br> • Must stay local and never be redistributed |
| **Implementation Difficulty** | Medium to hard |
| **Fit with Constraints** | Best long-term, blocked for now |

**Option C: Author everything by hand**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Full control and no licensing question <br> • Original text |
| **Disadvantages** | • Months of typing before a playable exists <br> • Risk of abandoning the project |
| **Implementation Difficulty** | Extreme |
| **Fit with Constraints** | Only for the gaps A and B cannot fill |

---

#### Decision & Rationale

**Chosen Option:** A now, B when the game is available, C only for gaps.

**Reasoning:**

> The wiki import turned a months-long data-entry problem into a few sessions and gave the engine real data to be designed against. It cannot supply game logic, so the quest `conditions` and `effects` and the dialogue graph stay empty until option B, except for the Goodsprings slice, which will be authored by hand.
>
> **Trade-offs accepted:** the imported text is CC BY-SA and must keep attribution if ever distributed; quest consequence rules were read from wiki notes and are not verified in game.

---

#### Implementation Notes

> - One importer per data family (`scripts/fetch_*.py`), each with `list`, `fetch` and `build`.
> - Raw data is never edited by hand, apart from the two curated files `consequences.json` and `formulas.json`.
> - Records keep their `form_id` so the xEdit export can be joined to them later.

---

#### References

- Fallout Wiki MediaWiki API: https://fallout.fandom.com/api.php
- xEdit (FNVEdit) project pages

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Language and Stack: Rust with ratatui

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The engine interprets a vocabulary of conditions and effects that will grow from 14 and 10 variants today to hundreds once the game's own data is imported. It must be a turn-based game with a panel-based terminal interface, built by one developer over what is expected to be years.

---

#### Options Considered

**Option A: Rust + ratatui**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Exhaustive `match` on enums: a new condition kind is flagged everywhere it must be handled <br> • Distinct id types prevent mix-ups <br> • Single native binary <br> • Compiler-driven refactoring |
| **Disadvantages** | • Slower iteration and compile times <br> • ratatui is flexible but needs more hand-built widgets <br> • Schema-to-type codegen handles `oneOf` imperfectly |
| **Implementation Difficulty** | Medium to hard |
| **Fit with Constraints** | Best long-term fit for a growing rule vocabulary |

**Option B: Python + Textual**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Fastest to a playable slice <br> • The data tooling is already Python <br> • Rich TUI toolkit, official LLM SDK |
| **Disadvantages** | • Missed branches fail at runtime, possibly hours into a playthrough <br> • Awkward single-binary packaging |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Best for speed, weaker for a large rules engine |

**Option C: Go + Bubble Tea**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Single binary, quick compiles <br> • Polished TUI toolkit |
| **Disadvantages** | • No sum types, which is the core data shape here <br> • No etiquette file in the playbook |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Weakest at what the project is mostly made of |

C, C++, Zig and TypeScript were also considered and rejected: manual memory handling for a string-heavy interpreter gains nothing at this speed, and their JSON-schema and TUI tooling is weaker (TypeScript was the closest runner-up).

---

#### Decision & Rationale

**Chosen Option:** Rust, with Python kept for data tooling only.

**Reasoning:**

> The user expects to work on this for years, and the rule vocabulary will keep growing. Exhaustive pattern matching and distinct id types make that growth safe. Python stays where it is strong: the importers, the normalizer and the validator.
>
> **Trade-offs accepted:** slower early iteration, and more TUI code to write by hand. Python with Textual remains the fallback if iteration speed becomes the problem. Golden test scenarios stay language-neutral JSON, so any implementation can be checked against the same specification.

---

#### Implementation Notes

> - Cargo workspace on edition 2024 with `crates/engine` (library) and `crates/tui` (binary `fnv-terminal`).
> - Add dependencies together with the code that uses them (`serde`, `thiserror`, `tracing`, `proptest`, `ratatui`, `crossterm`).
> - Rust types for `Condition` and `Effect` must be checked against `schemas/common.schema.json`, not written twice by hand.

---

#### References

- Rust API Guidelines, ratatui and serde documentation

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Scope of the First Playable: Goodsprings

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

A full port is a multi-year project. The first milestone has to prove every core system without requiring the whole game's content.

---

#### Options Considered

**Option A: Goodsprings only**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • The tutorial exercises dialogue, skill checks, quests, combat and reputation <br> • Small enough to author by hand |
| **Disadvantages** | • No travel or multi-faction play yet |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: Goodsprings, Primm, Novac and the main quest to Ring-a-Ding-Ding**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Exercises travel and factions |
| **Disadvantages** | • Several times the authoring work before anything is playable |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Good second milestone |

**Option C: Up to the Strip**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Includes the House/Yes Man branch |
| **Disadvantages** | • Far too large for a first playable |
| **Implementation Difficulty** | Extreme |
| **Fit with Constraints** | Poor |

---

#### Decision & Rationale

**Chosen Option:** A, with turn-based AP combat, dialogue with skill checks, the quest engine and reputation/karma as core systems.

**Reasoning:**

> Leveling and perks, vendors and crafting, companions, save and load, and community mods are planned but follow the core. An LLM mode is a post-v1 goal and must stay constrained by game state.

---

#### Implementation Notes

> - Roadmap priorities live in `ROADMAP.md`.

---

#### References

- `ROADMAP.md`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Engine Architecture: Shared Contracts, Feature-Owned State

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

`Condition` and `Effect` are shared by quests, dialogue, perks and NPC availability, and evaluating a condition needs state from many features. The architecture rules require that deleting a feature still compiles and that `shared/` never imports a feature.

---

#### Options Considered

**Option A: Enums, evaluator and the whole game state in `shared/`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Simplest <br> • One struct the evaluator reads directly |
| **Disadvantages** | • `shared/` becomes a dumping ground for every feature's state <br> • Deleting a feature leaves its state behind |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Breaks the spirit of feature-first design |

**Option B: Open registry of condition and effect kinds, one trait object per kind**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • True delete test <br> • Mods could add new kinds |
| **Disadvantages** | • Loses exhaustive `match`, the reason for choosing Rust <br> • Typos in rule data surface only at load time |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Conflicts with the type-driven design goal |

**Option C: Exhaustive enums and small read-only view traits in `shared/`; features own their state and implement the views; a thin `game` module composes them**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Exhaustive matching stays <br> • Features own their state and never import one another <br> • Default trait answers let a deleted feature compile away <br> • Maps one-to-one onto `common.schema.json` |
| **Disadvantages** | • Enum variants remain for deleted features <br> • `game` must know every feature (its job) <br> • Effects need a command-style sink |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best balance |

---

#### Decision & Rationale

**Chosen Option:** C.

**Reasoning:**

> It keeps the compile-time checking that justified Rust, while meeting the delete test through default trait answers instead of dynamic dispatch. The hundreds of condition kinds expected from the game-data import would be split across several small view traits rather than one large one.
>
> **Trade-offs accepted:** mods cannot add new kinds of condition, only new data using the existing vocabulary. If that ever matters, option B can be revisited without changing the data or schemas.

---

#### Implementation Notes

> - `crates/engine/src/shared/`: ids, `Condition` / `Effect`, view traits, loader, errors.
> - `crates/engine/src/game/`: the composition root, the only module that may know every feature.
> - Features: `character`, `quests`, `dialogue`, `combat`, `reputation`, `inventory`, `world`, `companions`, `saves`, `modding`.

---

#### References

- Feature-first architecture rules; Rust API Guidelines on visibility

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Quality Gate: A Single `scripts/check`

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The project mixes a Python data pipeline and a Rust workspace. Both must stay green, and a failure must not slip through unnoticed.

---

#### Options Considered

**Option A: One script, data steps run only when `data/`, `schemas/` or `scripts/` changed**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • One command for everything <br> • Fast when only Rust changed |
| **Disadvantages** | • Behaviour depends on git state <br> • Everything counts as changed until the first commit |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Good |

**Option B: One script, always run everything**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Simplest and most predictable |
| **Disadvantages** | • About eight seconds of data steps on every run |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Acceptable |

**Option C: Separate scripts for data and Rust**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Each part is runnable alone |
| **Disadvantages** | • Easy to forget one; no single pass/fail signal |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Weaker |

---

#### Decision & Rationale

**Chosen Option:** A, with `CHECK_ALL=1` to force the data steps.

**Reasoning:**

> A single pass/fail command that is silent on success matches the project rule of showing only failures. The data steps regenerate the engine data in a temporary directory and diff it against the committed copy, so a stale `data/engine` cannot be committed unnoticed.
>
> **Trade-offs accepted:** the Python steps need `jsonschema` in a local `.venv`, and benchmarks and heavier integration checks are not part of the routine run.

---

#### Implementation Notes

> - Steps: normalize into a temp directory and diff, validate, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
> - A deliberately stale data file was used to confirm the failure message and exit code.

---

#### References

- `scripts/check`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Keeping the Rust Types and the Schema in Step

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

`schemas/common.schema.json` is the language-neutral definition of ids, `Condition` and `Effect`. The Python tooling validates against it and mod authors will rely on it. The Rust enums must match it exactly, or data could pass the validator and fail in the engine, or the reverse.

---

#### Options Considered

**Option A: The schema stays the source of truth; hand-written Rust types guarded by contract tests**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Ergonomic types: distinct id newtypes, bounded numbers, behaviour where needed <br> • The Python pipeline and the Rust build stay independent <br> • Drift is caught from both directions |
| **Disadvantages** | • Two definitions to maintain, though neither can change alone without a failing test |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: Generate the Rust types from the schema at build time (`typify`)**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • One source of truth |
| **Disadvantages** | • Ids become plain strings, losing `QuestId` vs `NpcId` <br> • `oneOf` produces awkward generated names <br> • Heavy build-time dependency |
| **Implementation Difficulty** | Easy to start, hard to polish |
| **Fit with Constraints** | Gives up the type-driven design goal |

**Option C: Rust is the source of truth; derive the schema with `schemars` and diff it in `scripts/check`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • One definition, newtypes and exhaustiveness preserved |
| **Disadvantages** | • Reverses ownership of a file the Python tools depend on <br> • The data pipeline would need cargo to regenerate it <br> • Every existing schema detail must be reproduced exactly |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Possible later if keeping two definitions becomes a burden |

---

#### Decision & Rationale

**Chosen Option:** A.

**Reasoning:**

> The schema is the neutral contract, and the contract tests are the "checked, not written twice" rule made executable. Three kinds of drift were injected on purpose and each one failed a test: a level removed from the schema, a new condition type added to it, and the stage requirement relaxed again.
>
> **Trade-offs accepted:** every new condition or effect kind needs a Rust variant, a schema branch, an entry in the tag list and a sample strategy. Forgetting one fails a test or, for the `tag()` match, the compiler.

---

#### Implementation Notes

> - `QuestState` carries its stage in the variants that need one, so "stage_at_least without a stage" cannot be built. The schema was tightened to say the same thing, and stage numbers are non-negative.
> - Parsing rejects unknown fields (`deny_unknown_fields`), as the schema does; a typo such as `"minn"` is an error instead of a silent default.
> - `Effect` carries `f64` amounts, so it is `PartialEq` but not `Eq`.
> - Property tests generate arbitrary valid conditions and effects, check them against the schema with the `jsonschema` crate, and read them back. A second set compares every type name and closed set in both directions.
> - `Redacted<T>` masks values in logs. No benchmark was added: this feature has no hot path, and the first one arrives with the evaluator and combat.

---

#### References

- `crates/engine/tests/shared/contract.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Data Loader: Typed Catalogs with a Location Scope

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The engine needs `data/engine/` as typed Rust values before the evaluator, quests or dialogue can exist. The first playable is Goodsprings only, so loading every quest, NPC and dialogue file at startup is wasted work.

---

#### Decision

> Each record is parsed on its own into a typed struct, so an error names the file and the record id. `Scope::Around(location)` loads only the quests, NPCs and dialogue that belong to a location and the places inside it; `Scope::Everywhere` loads all of it. Items, factions and locations always load in full because other records refer to them.
>
> **Trade-offs accepted:** the loader checks only what is visible inside one file or needed to build the catalogs (duplicate ids, dialogue topic links). Cross-file references stay with `scripts/validate.py`, so the rules are not written twice.

---

#### Implementation Notes

> - Record types reject unknown fields, as the schemas do.
> - Tests load the real data, then load small broken data sets and expect an error naming the file and record.
> - The benchmark measures about 25 ms for Goodsprings and 49 ms for everything, so startup cost is not a concern yet.

---

#### References

- `crates/engine/src/shared/load.rs`
- `crates/engine/tests/shared/loader.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

## 🧠 Usage Guidelines

### When to Write an Entry

- **Before committing to a major design choice** – a library, a data model, a dependency.
- **When you revisit a decision** – document the reversal and the reasons.
- **After a significant learning** – so future work avoids the same pitfall.
