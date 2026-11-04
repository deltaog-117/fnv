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
| 2026-10-25 | Quest engine | `QuestBook` definitions + `QuestLog` state machine (COA A); rules fire in `game` | ✅ Confirmed |
| 2026-10-25 | Game state | Concrete `Game` holding the quest log; unbuilt features refuse effects (COA A) | ✅ Confirmed |
| 2026-10-25 | Character and inventory | Both in one step, minimal; `Game` owns them and keeps the view traits as the only contract (COA A) | ✅ Confirmed |
| 2026-10-26 | Leveling, perks and traits | Perk and trait effects are typed `Modifier` data summed once when taken; only fully modeled perks are listed | ✅ Confirmed |
| 2026-10-26 | Level-up screen | Level-up screen only; the creation screen waits for Doc's intro (COA B) | ✅ Confirmed |
| 2026-11-03 | Weapon instances, restocking, fleeing, named saves | Weapon objects with ids, global restock from the clock, one flee roll, a folder of named saves (B, A, A, B) | ✅ Confirmed |

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

### Evaluator: Read Views and Write Sinks per Domain

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

Dialogue options, quest rules and encounters all ask "does this condition hold?", and quest rules and dialogue choices all say "apply these effects". Both must mean the same thing everywhere, and `shared` may not import a feature.

---

#### Decision

> `evaluate` reads state through six small view traits and `apply` writes it through six sink traits, one pair per domain (quests, standing, character, inventory, companions, world). The view methods have default answers, so a test implements only the domain it needs. The sink methods have no defaults, and `apply` is an exhaustive `match`, so a new `Effect` variant is a compile error until its owner handles it. `apply_all` checks the whole list against the read view first (finite amounts, items and caps affordable, counting earlier gives in the same list) and only then applies it.
>
> **Alternatives rejected:** a pure `plan` returning a list of changes (atomic and loggable, but a second enum that mirrors `Effect`; revisit if the LLM mode needs to vet proposed effects), and an evaluator-only module with each feature applying its own effects by tag (smallest, but loses exhaustiveness).
>
> **Trade-offs accepted:** the pre-flight cannot see what only the owning feature knows, such as an unknown id. A sink error mid-batch leaves earlier effects applied, and `BatchError::applied` says how many.

---

#### Implementation Notes

> - Unknown things answer with the default: nothing started, nothing owned, an NPC in no state (`Alive` included).
> - Tests use two stand-in states: a small honest `MemoryState`, and a `HashedView` that answers each question from a hash, for the laws of `all`/`any`/`not`.
> - A property test checks that after any failed batch the state equals "the first `applied` effects applied one by one".
> - The benchmark stub measures about 5 ns per evaluation of a four-clause gate against an empty world. It stops at the first failing clause, so it is a floor, not a typical figure.
> - `game` does not implement the traits yet; each feature will implement its own as it lands.

---

#### References

- `crates/engine/src/shared/view.rs`, `sink.rs`, `eval.rs`
- `crates/engine/tests/shared/evaluator.rs`, `memory.rs`
- `crates/engine/benches/evaluate.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Quest Engine: A Validated Book and a Lifecycle Log

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The evaluator needs something to read quest conditions from and write quest effects to. The quest data has stages and 116 prose consequence rules, but the machine-readable `conditions` and `effects` are still empty.

---

#### Decision

> `QuestBook` holds every quest's stages and rules, checked once when it is built (stages strictly increasing, ids unique). `QuestLog` holds the player's progress and implements `QuestView` and `QuestSink`. The lifecycle is `not started -> active -> completed | failed`; completed and failed are terminal, stages only move forward, and reaching a `finish` stage completes the quest. "Not started" is the absence of a record, so a stored quest cannot claim to be unstarted. Locking a quest out is failing it, because failed is terminal, so there is no separate blocked set.
>
> **Alternatives rejected:** B, the same log plus an authored Goodsprings slice and a rule engine (the next feature, once the log is proven; it needs hand-authored data), and C, a generic flag graph (flexible, but duplicates the `QuestStatus` and `QuestStage` contract that `shared` already fixes).
>
> **Trade-offs accepted:** the consequence rules do nothing yet. Firing them needs the whole game state, so that belongs to `game`; the book only hands them out with `rules`. A failed effect leaves the log unchanged, but `apply_all` on a mixed list is still only as atomic as the sinks.

---

#### Implementation Notes

> - Starting an active or completed quest, and completing a completed one, are no-ops; a failed quest refuses to start or complete. Only an active quest can be completed, but any defined quest can fail before it starts, since rules shut quests out ahead of time.
> - A late fail or lock-out cannot undo a completion.
> - The log shares the book through `Arc`, so cloning copies only the progress.
> - Tests: lifecycle cases, two property tests over random effect sequences (terminal states hold, stages never go back, a refused effect changes nothing), and a walk of all 172 staged quests in the real data.
> - The benchmark stub measures about 1 us for a nine-stage playthrough.

---

#### References

- `crates/engine/src/quests.rs`, `quests/book.rs`, `quests/log.rs`
- `crates/engine/tests/quests/`
- `crates/engine/benches/quests.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Game State: A Concrete `Game` With Honest Placeholders

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

`apply_all` needs one value that implements all six view and all six sink traits. Only the quest log exists; character, inventory, reputation, companions and world are still empty modules. The sink traits have no default bodies on purpose, so `game` has to say something for the features that are not built.

---

#### Decision

> `Game` is a concrete struct that holds the `QuestLog` and delegates the quest view and sink to it. The five unbuilt views keep the shared defaults (nothing owned, nobody met). The unbuilt sinks return `EffectError::Refused` with a reason naming the missing feature, so an effect aimed at them fails loudly instead of vanishing.
>
> **Alternatives rejected:** B, a small real `WorldState` built in the same cycle (it is the next feature's work and deserves its own tests), and C, a generic `Game<Q, S, C, I, W, P>` (heavy generics for a single real implementation; the view and sink traits already give the seam).
>
> **Trade-offs accepted:** until a feature lands, rules and dialogue effects that touch it (caps, flags, items) error. Each placeholder is replaced by a real field when its feature is built.

---

#### Implementation Notes

> - Cloning is cheap because the quest book is shared, so a caller can try a batch on a copy.
> - `apply_all` is not atomic once it passes the pre-flight, so a refusal mid-batch can leave earlier effects applied; the tests only claim that a refused single effect changes nothing.
> - Tests: quest effects and conditions through `Game`, a property test that every unbuilt effect is refused with its own tag, and every staged quest in the real data walked to its first finish stage through `apply_all`.
> - The benchmark stub measures about 500 ns for a five-effect batch on a fresh `Game`.

---

#### References

- `crates/engine/src/game.rs`, `game/state.rs`
- `crates/engine/tests/game/`
- `crates/engine/benches/game.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### World State: Flags and NPC Fates Over a Shared Roster

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

Most quest consequence rules set a flag or kill, anger or remove an NPC, so rule firing and dialogue both need a real world state. `Game` refused every world effect, and the `WorldSink` contract says an effect on an unknown NPC must fail.

---

#### Decision

> `world/` holds `WorldBook`, the read-only roster of NPC ids, and `WorldState`, which implements `WorldView` and `WorldSink` over it. An NPC is `Alive { hostile }`, `Dead` or `Removed`, plus a `met` mark, so a dead hostile NPC cannot be stored. Only NPCs that changed are stored, so cloning stays cheap. `Game` gets a `world` field; `Game::new` keeps its signature with an empty roster, and `Game::with_world` takes a populated one.
>
> **Alternatives rejected:** B, rule firing in the same cycle (the rules only carry prose `when` text, so firing needs structured conditions first), and C, the whole Goodsprings slice at once (too large to review, and the authoring should target a rule engine that already runs).
>
> **Trade-offs accepted:** endings have no catalog yet, so any ending id is accepted and an untouched ending reads as `None`. Flags are free-form, with no registry. `meet` is an inherent method because the sink traits have no "met" effect.

---

#### Implementation Notes

> - A removed NPC is in no state, `Met` included; a dead NPC stays met. Killing the dead and removing the removed change nothing; killing the removed, and changing the attitude of the dead or removed, are refused.
> - A `Game` built with `Game::new` has no NPCs, so NPC effects fail with `Unknown` rather than vanish.
> - Tests: one case per rule, property tests for the NPC laws and for "a refused effect changes nothing", and world effects through `Game`. The batch test pins the existing behavior that `apply_all` leaves earlier effects applied when a later sink refuses.
> - The benchmark stub measures about 275 ns for a four-effect batch on a fresh `WorldState`.

---

#### References

- `crates/engine/src/world.rs`, `world/book.rs`, `world/state.rs`
- `crates/engine/src/game/state.rs`
- `crates/engine/tests/world/`
- `crates/engine/benches/world.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Goodsprings Quest Slice: Authored Effects and Rule Firing

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The quest `conditions` / `effects` arrays were empty, so nothing exercised the evaluator end to end. The first playable is Goodsprings only, and a quest is not playable until reaching a stage can pay out and the consequence rules can fire.

---

#### Decision

> Authored data goes in a hand-curated overlay, `data/quests/slice_goodsprings.json`, which `normalize.py` merges into `data/engine/quests.json` (COA A of three; the others were a standalone slice pack and rules-only). The overlay adds `on_enter` effects to existing stages and `conditions` / `effects` to existing consequence rules, matched by kind and target name, so `consequences.json` stays the prose source of truth. A typo raises instead of dropping a rule. `Game` runs the automation: reaching a new stage applies that stage's `on_enter` effects, and every change to a quest fires that quest's rules whose conditions hold, once each.
>
> **Alternatives rejected:** B, a standalone pack loaded beside `quests.json` (quest truth in two places), and C, rules only (no rewards, so no quest could be finished).
>
> **Trade-offs accepted:** a change to an automated quest is applied on a clone and kept only if every effect it sets off succeeds, which costs about 370 ns per stage change. Rewards for features that are not built (caps, XP, items) are postponed into `Game::deferred()` through the new `EffectError::NotBuilt`, so the reward is kept and every real refusal still fails. Skipping a stage skips its effects. A rule is evaluated only when its own quest changes, not when a flag or a reputation does.

---

#### Implementation Notes

> - Authored: stage rewards for Ain't That a Kick in the Head, Back in the Saddle, Ghost Town Gunfight and They Went That-a-Way, and the two rules that carry a gate (negative Goodsprings reputation locks the gunfight; finishing the main quest before Trudy's quest locks hers).
> - Not authored, on purpose: Goodsprings fame and Powder Gangers infamy (the wiki gives no amounts), the Vault 21 jumpsuit (no item record), skill-tagged starting gear (the vocabulary has no "tagged skill" condition), and quests with no stages (Trudy's Radio Repair, Ringo's Caravan Rules, Victor's two).
> - `validate.py` checks that every quest, faction, item and NPC named in authored data exists and that a `set_stage` names a real stage.
> - Tests: stage payment, skipped stages, rule firing and its quiet case, rollback of a failing change, a property test (any order of moves pays each stage at most once and a failure changes nothing), and ten language-neutral scenarios in `tests/scenarios/goodsprings.json`.
> - The benchmark now reports a plain and an automated quest: about 690 ns and 1.8 µs per batch of five stage changes.

---

#### References

- `data/quests/slice_goodsprings.json`, `scripts/normalize.py`, `scripts/validate.py`, `schemas/quest.schema.json`
- `crates/engine/src/game/state.rs`, `crates/engine/src/quests/book.rs`
- `crates/engine/tests/game/automation.rs`, `crates/engine/tests/game/scenarios.rs`, `tests/scenarios/goodsprings.json`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Dialogue Runner: A Generic Conversation Over an Authored Overlay

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The quest engine and game state run conditions and effects, but nothing let the player talk. The wiki's dialogue is a flat list of topics with no links, conditions or effects, and most Goodsprings quests start in a conversation.

---

#### Decision

> The runner lives in `dialogue/` and is generic over `State + Clone`, so it does not import `game` (COA A of three; the others were a separate per-scene graph format and a flat-menu fallback with a generated draft graph). `DialogueBook` resolves links to positions once and refuses a bad graph; `Conversation` holds only a position (the start, after a topic, or over). The graph is authored in a hand-curated overlay, `data/dialogue/slice_goodsprings.json`, merged by `normalize.py`, the same way as the quest slice. Two optional schema fields were added: `start` on a dialogue (the menu a talk opens with) and `ends` on a topic. A topic with no `next` returns to the start, and if every follow-up is hidden the start is offered, so the player is never stuck.
>
> **Alternatives rejected:** B, a second graph format for gameplay (drifts from `dialogue.schema.json` and discards its `next`), and C, a flat menu with a generated draft graph (152 topics for Doc Mitchell make a flat menu unusable, and the generator is a project of its own).
>
> **Trade-offs accepted:** a topic's effects are applied to a clone and kept only if all succeed, which costs a clone of the state per choice. `check` means "listed but locked while it fails"; a success/failure pair from the transcripts is written as two topics with exclusive `conditions`. Topics whose effects need unbuilt features (paid healing, supplies) are not authored yet, because `apply_all` would refuse them. Every authored link and condition is read from the transcript text and is not verified in game.

---

#### Implementation Notes

> - Authored: the start menus of Doc Mitchell and Sunny Smiles, Doc's "about yourself" chain, Sunny's area chain, Sunny starting Back in the Saddle (stage 4) and moving Ghost Town Gunfight from stage 5 to 10, and two flag-gated follow-ups.
> - No wiki topic carries a skill-check threshold, so no real topic has a `check` yet; the check path is covered by hand-built dialogues.
> - `validate.py` checks `start` and `next` links, that an ending has no `next`, and that quests and NPCs named in conditions and effects exist. A real-data test fails if an authored topic cannot be reached from a start topic.
> - Tests: 24, including a property test over random graphs and walks (refused picks change nothing, an ended talk offers nothing, the same walk gives the same trace) and seven golden scenarios in `tests/scenarios/goodsprings_dialogue.json`.
> - The benchmark stub measures about 300 ns to build Doc Mitchell's menu.

---

#### References

- `crates/engine/src/dialogue.rs`, `dialogue/book.rs`, `dialogue/conversation.rs`
- `data/dialogue/slice_goodsprings.json`, `scripts/normalize.py`, `scripts/validate.py`, `schemas/dialogue.schema.json`
- `crates/engine/tests/dialogue/`, `tests/scenarios/goodsprings_dialogue.json`, `crates/engine/benches/dialogue.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Combat Core: A Fight Over a Small Combatant Trait

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

Combat was an empty stub, and the character and inventory modules it would read from are stubs too. The curated formulas cover weapon damage, resistance, threshold, action points and crit chance, but the wiki gives no hit-chance or spread formula.

---

#### Decision

> `combat/` defines a `Combatant` trait for the numbers it reads (hit points, action points, threshold, resistance, crit chance, weapon) and tracks only what changes in a fight itself (COA C of three; the others were a plain-struct core with no trait, and a full pipeline over the character and inventory views). Hit chance is behind a swappable `HitModel` with a labelled flat stand-in, and chance comes from a `Dice` trait, so a fight is deterministic for a given sequence of rolls and tests script the dice. Numbers are validated once at the edge (`Points`, `Multiplier`, `Percent`, a non-zero hit-point maximum and attack cost), so the fight never re-checks them.
>
> **Alternatives rejected:** A, a core on plain structs (a subset of C that would have to be rewired when `Game` arrives), and B, the full pipeline over `CharacterView` and `InventoryView` (blocked by the stubs, and the hit formula is not in the data).
>
> **Trade-offs accepted:** a critical hit adds the weapon's critical damage to the base before the multipliers; the wiki does not say where it enters, so this is a stand-in. A hit that gets through at all costs at least 1 hit point, so a fight with any hitting weapon ends. Turn order is the order the fighters are given; initiative is not modelled. Weapon condition is an input multiplier, not a changing value, and body parts and spread are not modelled. A move is checked as a whole before anything changes, so a refused attack costs no action points and rolls no dice.

---

#### Implementation Notes

> - Nine real weapons (Hologram beam, Laser detonator, X-25 Gatling laser and others) have an action-point cost of 0 in the data and three have none. A fight needs a cost of at least 1, so the adapter that builds an attack from a weapon has to decide what to do with them. A real-data test pins the exact list so a change shows up.
> - `Rules::standard()` mirrors `constants` in `formulas.json`; a real-data test fails if they drift.
> - Tests: 29, including four property tests and twelve golden scenarios in `tests/scenarios/combat.json`.
> - The benchmark stub measures about 490 ns for a four-fighter fight of sure hits.
> - Not wired into `Game` yet: that needs character and inventory.

---

#### References

- `crates/engine/src/combat.rs`, `combat/stats.rs`, `combat/damage.rs`, `combat/hit.rs`, `combat/fight.rs`
- `crates/engine/tests/combat/`, `tests/scenarios/combat.json`, `crates/engine/benches/combat.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### TUI v0: An Elm-Style App Over the Headless Engine

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The engine has quests, a world, dialogue and combat, but nothing can be played. The roadmap asks for a persistent layout so the player never has to remember commands: scene and log, status, active quest panel and an action footer.

---

#### Decision

> `crates/tui` is an Elm-style app (COA A of three): `App` owns the `Game`, the log and the active `Screen`; keys become a small `Command` enum; `App::update` changes state and `App::render` draws it. Screens (`exploration`, `dialogue`) depend only on `shared` and never on each other; `app.rs` is the one place that knows all of them, as `game` is in the engine. The terminal is touched only in `main.rs`, so tests drive `App` headless with ratatui's `TestBackend`.
>
> **Alternatives rejected:** B, a stack of `Box<dyn Screen>` (dynamic dispatch, and the log and quest panel would have to be threaded through every screen), and C, one event loop with a `match mode` and inline drawing (smallest, but untestable without a terminal and it grows into one function).
>
> **Trade-offs accepted:** v0 covers Goodsprings only. People to talk to are the NPCs located in the settlement or its buildings that have an authored dialogue graph. A new game starts "Ain't That a Kick in the Head" at stage 10, because there is no start-of-game script yet. There is no travel, inventory or combat screen; the footer lists only the keys that work.

---

#### Implementation Notes

> - Dependencies: `ratatui` 0.30 (its re-exported `crossterm`), `anyhow` for the binary, `textwrap`; `proptest` for tests.
> - The log wraps its own text instead of using the paragraph widget's wrapping, because the newest line must always be the last one visible.
> - The quest panel sorts by quest name; the quest log keeps quests in no stable order and the panel would otherwise shuffle between frames.
> - Below 70x22 the layout is replaced by a "terminal too small" message instead of a broken frame.
> - Tests: menu, log and layout properties; input mapping; screen logic; `App` over the real data (first frame, smallest terminal, talking to Doc Mitchell, quit, walk away).
> - No benchmark stub: the front end is a binary crate, which a bench target cannot import, and drawing is not a measured hot path. Revisit if frames get slow.
> - No tracing yet; add it with the first screen that does I/O beyond the keyboard.

---

#### References

- `crates/tui/src/app.rs`, `main.rs`, `shared/`, `exploration/`, `dialogue/`, `quests/`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Character and Inventory: Both Filled In One Step, Minimal

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

`character/` and `inventory/` were empty stubs, so every XP, caps and item reward in the Goodsprings slice was postponed into `Game::deferred()`, combat could not read a hit-point or action-point total, and the paid dialogue topics had nothing to charge.

---

#### Decision

> Build both in one step and keep each minimal (COA A of three; the others were inventory first with the character stubbed, and a JSON-preset character). `Character` stores only the seven S.P.E.C.I.A.L. values and the XP earned; the level, hit points, action points, carry weight and every skill's starting value are derived on demand from the curated formulas, so XP and level cannot disagree. `Inventory` stores item stacks, bottle caps and three equipment slots (weapon, outfit, head) over a shared read-only `ItemBook`. Both implement the existing view and sink traits, and `Game` delegates to them, replacing the "not built" refusals for XP, caps and items.
>
> **Alternatives rejected:** B, inventory now and the character in a second cycle (smaller diffs, but combat needs derived stats and equipped gear together, so `Game` would change twice), and C, a character loaded from a JSON preset (fast, but a throwaway that leveling and creation would replace).
>
> **Trade-offs accepted:** a skill is its starting value from S.P.E.C.I.A.L. alone: no skill points, tagged skills, perks, traits or creation screen yet. The default character has a 5 in everything. Current hit points are not stored (maximum only); they arrive with the combat wiring. Using consumables, repair, crafting and barter are not built.

---

#### Implementation Notes

> - Quest data writes XP and caps as floats. `whole_amount` (in `shared/sink.rs`) accepts only finite whole numbers, so a fractional reward is refused as a data mistake instead of being rounded away.
> - An item must be in the `ItemBook`, built from the weapon, armor, ammo and consumable catalogs (922 items). Weapon mods, skill books and crafting components are not loaded, so they cannot be carried yet.
> - Laws in `Inventory`: a stack is never empty, caps never go below zero, an equipped item is always carried, and taking the last copy unequips it. Weight never blocks a pickup; `Game::is_encumbered` only reports it, for travel and running to read later.
> - `Game::with_world` keeps its signature. `with_items` and `with_character` are builders; a game without an item book refuses every item as unknown, so tests and the TUI pass one.
> - Derived formulas are hand-written, as in combat. A real-data test pins the expression strings in `formulas.json`, the constants, the governing attribute of all 13 skills, and the XP totals of all 50 rows of `levels.json`, so drift fails the build.
> - Golden scenarios no longer list caps, XP and items as deferred: they now state the caps, XP and items that must result. `Game::deferred()` still exists for reputation, karma and companions.
> - The TUI passes the item book and shows level and caps in the status line. There is no inventory or character screen yet.

---

#### References

- `crates/engine/src/character/`, `crates/engine/src/inventory/`, `crates/engine/src/game/state.rs`
- `tests/scenarios/goodsprings.json`, `data/engine/formulas.json`, `data/engine/levels.json`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Combat Wiring: Real Stats From the Item Data, Authored Loadouts and Hit Points

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The combat core fought plain numbers and the game state held no current hit points, so the Powder Gangers could not be fought. The plan was COA B: wire it with a small hand-curated file and leave `ItemBook` alone.

---

#### Decision

> Build the adapter in `game/` (the composition root), so `combat/` stays free of the character and the pack. `Arsenal` is built once from `GameData` and checks every weapon, armor piece and encounter, so a bad encounter fails at load and not mid-fight. `Fighter` is the `Combatant` for the player and for foes. The player's wounds live in `Game` as hit points lost; `Game::start_battle` and `Game::finish_battle` bracket a `Battle`, and a win applies the encounter's effects atomically.
>
> **Departure from COA B:** the weapon and armor numbers are not duplicated in a combat slice, because `weapons.json` and `armor.json` already carry damage, action-point cost, critical damage and thresholds. Only what the wiki lacks is authored: which weapon and armor each foe carries, their hit points, when the fight is offered and what winning does (`data/combat/encounters.json`). `ItemBook` is still untouched.
>
> **Alternatives rejected:** A, extending `ItemBook` with combat stats (a larger refactor for no gain now), and C, keeping the fight inside the TUI (breaks the headless engine, and hit points would not persist or save).
>
> **Trade-offs accepted:** every attack has a flat 65% chance (the real model is unknown); skill and weapon condition do not change damage; there is no ammunition use, fleeing, healing between fights or allies. The foes' hit points are placeholders tuned over 2,000 random fights so a starting character in leather armor wins about three times in four. A foe with no hit points in its record would otherwise get the player's formula (200), which made the fight unwinnable (2% of fights). The nine weapons with no action-point cost are left out of the arsenal.

---

#### Implementation Notes

> - `Combat::with_hp` lets a fighter start wounded (zero is refused, so the fight cannot start over); `enemy_turns` plays the enemy side with one tactic and cannot loop, because every attack spends at least one action point.
> - Foes take action points and critical chance from `Character::at_level`, the formulas the player uses; a missing attribute reads as 5.
> - The loader keeps the NPC records of an encounter's foes in a location-scoped load, so an encounter cannot name an NPC that is not loaded.
> - The gunfight opens when Sunny agrees (stage 10), because stages 20 to 65 are not authored. The player starts in leather armor and takes the 9mm pistol in hand when a fight starts: the pistol arrives at stage 50 of the opening quest, and there is no inventory screen to equip it.
> - Not tested: failing winning effects leaving the game unchanged (the settle step works on a copy, as quest changes do); a test would need a hand-built encounter and `GameData` has no public constructor.

---

#### References

- `crates/engine/src/game/battle.rs`, `crates/engine/src/combat/ai.rs`, `crates/tui/src/combat/`
- `data/combat/encounters.json`, `schemas/encounter.schema.json`, `crates/engine/tests/game/battle.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Goodsprings Gunfight Path: Skip the Optional Stages, Gate the Fight at 65

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

The gunfight opened as soon as Sunny agreed (stage 10) because Ringo had no dialogue and stages 20 to 65 were not authored. Three courses of action were weighed; the user chose B, a playable chain in one small commit.

---

#### Decision

> Author Ringo's graph from the wiki's own topics (offer help sets stage 5, "Sunny's with us" then "Let's do this" sets stage 60) and Sunny's "Let's go" (stage 65), and move the encounter's requirement from stage 10 to stage 65. The optional stages 20 to 50 (Trudy, Chet, Doc, Easy Pete) are skipped, not faked.
>
> **Campfire quest:** Sunny's "Couldn't hurt" starts it (stage 10) and her Trudy errand finishes it (stage 40, which needs stage 30). Stage 30 is "use the campfire", a crafting action that does not exist yet, and the wiki has no dialogue line for it, so the quest cannot be finished in the TUI until crafting lands. It is offered while Back in the Saddle is active, not completed, because that quest's shooting and gecko stages are not playable yet.
>
> **Not done:** Run Goodsprings Run (Joe Cobb's side) and any rewards for the campfire quest, which the wiki gives no figures for.
>
> **Trade-offs accepted:** the gunfight skips its optional preparation, so the town is no better prepared for it; the tests that set the quest to stage 10 now set 60 and 65 as well.

---

#### References

- `data/dialogue/slice_goodsprings.json`, `data/combat/encounters.json`, `tests/scenarios/goodsprings_dialogue.json`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Ammunition: A Magazine on the Combatant, Rounds Settled With the Pack

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

Finishing combat (ROADMAP item 7) was split into slices because it bundles ammunition, weapon wear, initiative, fleeing, healing and the weapons with no action-point cost. Course of action A (mechanics first) was chosen, and its first slice is ammunition and reload.

---

#### Decision

> `Combatant` gains a defaulted `magazine()` (clip, rounds per shot, reload cost, rounds carried); `Combat` tracks the loaded and reserve rounds per fighter, so a weapon without a magazine behaves as before. A shot costs its rounds whether it hits or not, and a shot with too few loaded is refused before any dice are rolled.
>
> **Pack:** the fight starts with the weapon loaded and the rest of the player's rounds as the reserve. The game is not touched until `finish_battle`, which takes the rounds fired out of the pack on a win or a loss; the rounds left in the weapon go back, because no loaded state is kept between fights. Foes have an endless supply, and they reload when empty.
>
> **Which ammunition:** the plain load of the weapon's family, or another load of the family when the pack holds only that.
>
> **Stand-ins:** a reload costs 2 action points for every weapon (`RELOAD_AP` in `game/battle.rs`), because the data gives seconds and no conversion is known. Doc now gives 30 rounds of 9mm with the pistol, a guess to tune. The hologram beam has a clip but no resolved ammo family, so it needs no ammunition.
>
> **Trade-offs accepted:** a fight cannot be abandoned yet, so deducting at settlement and deducting shot by shot give the same result; the enemy's reload shows in the log only as the action points it took.

---

#### References

- `crates/engine/src/combat/fight.rs`, `crates/engine/src/game/battle.rs`, `crates/engine/tests/combat/ammo.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

---

### Trudy's Dialogue: Enlistment by a Flag, Not a Quest Stage

**Date:** 2026-10-25
**Status:** Confirmed

---

#### Context / Background

Authoring the rest of the Goodsprings dialogue (ROADMAP "Goodsprings dialogue content") was split into one NPC per cycle, starting with Trudy because she owns the optional stage 20 of the Ghost Town Gunfight. The wiki gives her topics, the two skill labels (Speech 25, Sneak 25) and her success and failure lines, but no links or conditions.

---

#### Decision

> Trudy's graph is in `data/dialogue/slice_goodsprings.json`. Her chatter is chained from the transcript (the Cobb argument leads to Ringo, the NCR news to the Legion and the Dam, the robot to Victor's history). A Speech 25 or Sneak 25 pick sets the flag `trudy_joined_militia`; each skill has a locked success option (`check`) and a failure line shown only while the skill is short.
>
> **Why a flag and not stage 20:** quest stages only move forward, and stages 20 to 50 are optional steps that can be done in any order. Setting stage 20 after the player has enlisted Chet (30) would be refused, and a refused effect refuses the whole topic. The flag has no such ordering, and the journal can read it later.
>
> **Window:** the enlistment is offered while the gunfight is active, from stage 10 (Sunny agreed) until Ringo's countdown at 60.
>
> **Left out for now:** her radio repair topics (they need the radio quest), the shop topic (vendors are a separate item) and the lines about Victor's death (no companion state yet).
>
> **Trade-offs accepted:** with the default 5s every skill is 15, so neither check can pass until leveling and tagged skills exist; the golden scenarios raise the attribute through a new optional `special` field instead. Links are read from the transcript and are not verified in game.

---

#### References

- `data/dialogue/slice_goodsprings.json`, `tests/scenarios/goodsprings_dialogue.json`, `crates/engine/tests/dialogue/scenarios.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-25 | Initial entry | deltaog-117 |

### Leveling, Perks and Traits: Effects as Typed Modifier Data

**Date:** 2026-10-26
**Status:** Confirmed

---

#### Context / Background

The character had S.P.E.C.I.A.L., XP and a derived level, but nothing to do with a level once earned. Several Goodsprings skill checks (Trudy's Speech 25 and Sneak 25) could not be passed at the default 5s. The wiki records what a perk or trait does only as prose.

---

#### Decision

> Each level-up pays out skill points (`10 + 0.5 * intelligence`, half points carried over) and, every second level, a perk pick. A skill is its starting value plus points spent, 15 if tagged, plus what perks and traits add, always between 0 and 100. The engine refuses a spend or pick that was not paid out, with an error that names the reason.
>
> **Effects as data:** a perk or trait changes a number through typed `modifiers` (skill points per level, XP percent, carry weight, hit points, action points, one skill or all skills), hand-authored in `data/perks/modifiers.json` and merged by `normalize.py`. The modifiers are summed once when a perk or trait is taken, so a derived number reads one field and never walks the perks. The schema and the Rust type are checked against each other by contract tests.
>
> **Only fully modeled perks are listed:** five perks and two traits so far. The rest can be taken and change nothing, and the picker says so, because a half-modeled perk would look like a working one.
>
> **Trade-offs accepted:** perks that change combat numbers wait for those numbers to exist. The player cannot reach any of this yet; the level-up and creation screens are the next step, and `Game` gets its `PerkBook` through `with_perks`, which only the tests call.

---

#### References

- `crates/engine/src/character/`, `crates/engine/src/shared/modifier.rs`, `data/perks/`, `crates/engine/tests/character/progression.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-26 | Initial entry | deltaog-117 |

---

### Level-Up Screen: Spend and Choose Now, Create the Character Later

**Date:** 2026-10-26
**Status:** Confirmed

---

#### Context / Background

The engine could level a character, but the player could not reach any of it. Three options were compared: wire the perk book only (A), add a level-up screen (B), or add that plus a character creation flow (C).

---

#### Decision

> **COA B.** The scene has a "Character" row that opens a screen with two lists: the thirteen skills (choosing one spends a point) and the perks open to the character (choosing one takes it). The screen keeps only its cursor and reads every number from `Game` when it draws, so it cannot show a stale count; every spend and pick goes through `Game`, so a refusal arrives in the engine's own words and is logged.
>
> **Why not creation yet:** the real game opens with Doc Mitchell's intro, which covers creation, so a separate creation flow now risks being rebuilt around it. Tags and traits stay reachable only through `Character::create` until then.
>
> **Perks the game does not model:** they are listed, because hiding them would hide the pick, but the selected one says that taking it changes nothing. That note goes above the description, because a long description would push it out of the pane.
>
> **Scrolling:** at the smallest supported terminal only about ten rows fit in the scene, so a small `window` helper keeps the selected row visible instead of clipping the list.
>
> **Trade-offs accepted:** the screen is reached only from the scene, not mid-conversation or mid-fight; there is no confirmation before spending a point, and no way to take one back.

---

#### References

- `crates/tui/src/character/screen.rs`, `crates/tui/src/shared/menu.rs`, `crates/tui/src/app.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-26 | Initial entry | deltaog-117 |

### Chet's Dialogue: Supplies by Flag, Two Quest Windows

**Date:** 2026-10-26
**Status:** Confirmed

---

#### Context / Background

Chet is the next Goodsprings graph and owns the optional stage 30 of the Ghost Town Gunfight ("convince Chet to open his store's stock"). His transcript also carries the lines for the Powder Ganger side (Run Goodsprings Run, stage 30). The wiki gives the topics, the 1000-cap payment and the Barter 25 and Speech 25 labels, but no links or conditions.

---

#### Decision

> Chet's graph is in `data/dialogue/slice_goodsprings.json`, built on the pattern of Trudy's. Chatter is chained from the transcript (Khans, New Vegas, weapon mods and surplus ammo). The supply talk is offered while the gunfight is active, from stage 10 to Ringo's countdown at 60; paying 1000 caps (a `caps` effect of -1000) or winning the Barter 25 argument sets the flag `chet_supplied_militia`. The Powder Ganger shakedown (Barter 25 or Speech 25) is offered while Run Goodsprings Run is active from stage 20 to 60 and sets `chet_shaken_down`. Each skill has a locked success option and a failure line shown only while the skill is short.
>
> **Why flags and not stages:** the same reason as Trudy's. Stages only move forward and 20 to 50 are optional in any order, so setting 30 after the player reached 40 would be refused, and a refused effect refuses the whole topic.
>
> **Why the shakedown is in now:** it is part of Chet's transcript and costs only gated topics. Run Goodsprings Run cannot start from Joe Cobb's side yet, so the golden scenarios put the quest at stage 20 by hand.
>
> **Left out for now:** the shop topics and greeting (vendors are a separate item) and the leather armor and ammo as items; in the wiki the armor equips the militia, not the player.
>
> **Trade-offs accepted:** a player with fewer than 1000 caps sees the payment option and is refused on picking it, because there is no condition on caps; the wiki labels the Barter check on the walkthrough page only, so the skill is read from there; links are read from the transcript and are not verified in game.

---

#### References

- `data/dialogue/slice_goodsprings.json`, `tests/scenarios/goodsprings_dialogue.json`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-26 | Initial entry | deltaog-117 |


---

### Character Creation: A Panel Screen Before the First Scene

**Date:** 2026-10-28
**Status:** Confirmed

---

#### Context / Background

`Character::create` existed, but the game always started with the default character (a 5 in everything, nothing tagged, no trait). Three layouts were compared: a one-page form (A), a four-step wizard (B), and presets with an optional customize screen (C). The player asked for a Fallout-style menu in which each part is its own panel that can be opened and changed, with a button to go on.

---

#### Decision

> **COA A, with panels.** One screen holds four panels: S.P.E.C.I.A.L., Skills (tag three), Traits (up to two) and a Done button, plus a Derived box with hit points, action points and carry weight. Left/Right (or Tab) moves between panels, Enter opens one, Esc closes it. Inside S.P.E.C.I.A.L. the arrows lower and raise a value; inside Skills and Traits, Enter toggles the row. The screen is drawn over the whole terminal, outside the persistent layout, because three panels do not fit in the scene region at the smallest size.
>
> **Invalid choices are not reachable:** the points left are derived from the spread (40 minus its total), a raise is refused at zero points left, tags stop at three and traits at two. The Done button stays locked until every point is spent and three skills are tagged, and says what is missing. A property test presses random keys and checks that the limits hold and that whenever the screen yields a character the engine accepted it.
>
> **Where it starts:** `App::with_creation` puts the screen first and `main` uses it; `App::new` still starts in the scene, so the existing tests are unchanged. Beginning swaps the character into the game that `App::new` built; the starting armor and the opening quest are kept.
>
> **Derived numbers:** they are computed from the spread and the tag bonus alone. Traits that change a number are applied by `Character::create` only, so the screen says "Your traits adjust these numbers once you begin" instead of showing a figure that could be wrong. A trait the game does not model yet says so in its description box.
>
> **Input:** `Command` gained `Left` and `Right` (arrows, `h`/`l`, Tab and Shift-Tab). There is no mouse support yet, so "click" is Enter.
>
> **Trade-offs accepted:** no name, age or sex (the game does not model them); no presets; no way to return to creation once the game begins; the skill list scrolls at the smallest terminal.

---

#### References

- `crates/tui/src/character/creation.rs`, `crates/tui/src/shared/input.rs`, `crates/tui/src/app.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-28 | Initial entry | deltaog-117 |

---

### Easy Pete and Joe Cobb: Two Doors into Run Goodsprings Run

**Date:** 2026-10-28
**Status:** Confirmed

---

#### Context / Background

Easy Pete owns the optional stage 50 of the Ghost Town Gunfight (dynamite, an Explosives 25 check). Joe Cobb is the giver of Run Goodsprings Run, and Chet's shakedown topics were waiting for that quest to be startable. The wiki gives both transcripts and one label (`< Speech 20 >` on Cobb's prison question), but no links or conditions; the Explosives 25 check comes from the quest walkthrough. The approach was mechanical (the pattern of Trudy and Chet), so no COAs were needed.

---

#### Decision

> Both graphs are in `data/dialogue/slice_goodsprings.json`.
>
> **Easy Pete:** chatter is chained from the transcript (name, prospecting, the NCR, the Legion, Hoover Dam). The dynamite talk is offered while the gunfight is active, from stage 10 to Ringo's countdown at 60; an Explosives 25 claim sets the flag `easy_pete_joined_militia`, and a character who is short on the skill hears the refusal and may ask again. The question about Joe Cobb is open until the gunfight is completed. Like Trudy and Chet, the outcome is a flag and not stage 50, because the optional stages come in any order and a stage only moves forward.
>
> **Joe Cobb, starting the quest:** "What say I help you take over this town?" offers three pitches that all lead to the same deal, and the deal (`start_quest` plus stage 10) is the only thing that starts Run Goodsprings Run. A player who already agreed to help Ringo (gunfight at stage 5 or later) hears his refusal in the transcript's sense: the pitches are hidden and only "Forget I said anything" remains.
>
> **Joe Cobb, the middle of the quest:** stage 15 ("tell Cobb that Ringo is dead") is set by Ringo's death, which no feature can cause yet, so the report ("Ringo's dead. What now?") is gated on stage 15 and moves the quest to 20, where Chet's shakedown opens. "I already took care of Ringo" is the other door: it is offered only when the quest has not started and Ringo is already dead, and it starts the quest at stage 15. "Let's take over Goodsprings" is open from stage 20 and sets stage 60, whether or not the optional prep was done. The spare-supplies hand-over sets stage 45 and needs the flag `doc_mitchell_gave_cobb_supplies`.
>
> **Why a new flag for Doc's supplies:** Doc Mitchell's trick (Medicine 25 in the transcript) is not authored yet, and a flag set by his topic later is the same mechanism Chet uses. Until then the hand-over is unreachable in the game and covered by a scenario that sets the flag.
>
> **Scenario runner change:** `scenarios.rs` now builds the game with the NPC roster (`Game::with_world`), as the real game does, so a scenario can kill Ringo in its setup. Thirty-two scenarios existed; 29 were added for Pete and Cobb (eight for Pete, 21 for Cobb).
>
> **Left out for now:** the greetings, the barks and combat lines; the "Let me get out of town before we talk" refusal (no condition for position); the closing topic 009 (the generic goodbye covers it); and the Cobb topics that come after the takeover battle (stage 70 and the rewards).
>
> **Trade-offs accepted:** in the game the two "now what" lines (019 and 023) may be wired differently than the transcript suggests; they are read from the text and are not verified in game. The Ringo-death report and the hand-over stay out of reach in play until Ringo can be killed and Doc's trick exists.

---

#### References

- `data/dialogue/slice_goodsprings.json`, `tests/scenarios/goodsprings_dialogue.json`, `crates/engine/tests/dialogue/scenarios.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-28 | Initial entry | deltaog-117 |


---

### Doc Mitchell: Healing as a Vocabulary Addition

**Date:** 2026-10-28
**Status:** Confirmed

---

#### Context / Background

Doc Mitchell's transcript has paid healing at three prices (100, 75 or 50 caps by how hurt the player is), a lie that gets spare supplies for Joe Cobb (Medicine 25), and the doctor's bags for the gunfight. Cobb's hand-over was waiting for the flag Doc's lie sets. The game already tracked wounds between fights and had a `Game::heal` method, but a conversation could neither ask how hurt the player was nor heal them, because dialogue only reaches the game through the shared `Condition` and `Effect` vocabulary.

---

#### Decision

> **The vocabulary grows by one condition and one effect.** `Condition::Health { percent, op }` compares the share of hit points left (0 to 100, `op` defaults to "at least", as in `Reputation`), and `Effect::Heal {}` restores full hit points. Both are in `common.schema.json`, and the contract tests check them in both directions.
>
> **A bounded type for the percentage.** `HealthPercent` (0 to 100) is built through `HealthPercent::of(current, max)`, which rounds down, so a single lost hit point already reads 99 and "100" means unhurt. A maximum of zero reads 0 and a current above the maximum reads full, so the function cannot fail.
>
> **Where the answer lives.** Wounds belong to `Game`, not to `Character`, so `CharacterView::health` has a default of full health (a state that tracks no wounds has nobody hurt) and `Game` overrides it. `CharacterSink::restore_health` has no default, as every sink method; the character sheet implements it as a documented no-op because it records no wounds, and `Game` clears its wounds. The sink method is not called `heal` so it cannot be confused with `Game::heal(amount)`.
>
> **`Heal` is an empty struct variant, not a unit variant.** The contract test caught that serde lets a unit variant of an internally tagged enum ignore `deny_unknown_fields`, so Rust accepted `{"type": "heal", "amount": 5}` while the schema rejects it. `Heal {}` honors the denial.
>
> **Doc's graph** (`data/dialogue/slice_goodsprings.json`): "I'm hurt" offers the three prices only in the band that matches the player (at most 33 percent, 34 to 66, 67 to 99), each as a payment (`caps` then `heal`, one atomic list, so a player who cannot pay is refused and stays hurt) beside the transcript's "I don't have the caps" line; an unhurt player gets no payment option. The bands treat 33, 66 and 99 as inclusive upper limits; the transcript names the numbers but not the edges, so this is a reading. The lie (the "injured man by the highway") is open while Run Goodsprings Run is at stage 20 to 44 and sets `doc_mitchell_gave_cobb_supplies`, the flag Cobb's hand-over reads; a Medicine 25 character is let through and the others hear the refusal. The gunfight bags topic now gives two doctor's bags with no skill check (the transcript shows no label and no refusal line for it, although the walkthrough says Medicine 30), and a flag (`doc_mitchell_gave_bags`) makes it a one-time gift.
>
> **Scenario runner:** it now attaches the item book, takes an optional starting `wounds`, and can expect items, caps and hit points. `Game::with_wounds` is the public way to start hurt (capped at the maximum, and set after the character).
>
> **Left out for now:** radiation cures (the game has no radiation), addiction cures, the "medical supplies" shop topic (vendors), and the opening tests and farewell (the opening quest covers them). The Medicine 25 lie gives nothing but the flag, because the data has no item for Doc's spare supplies (`medical_supplies` is Contreras's quest item).
>
> **Trade-offs accepted:** a player short of caps sees the payment and is refused on picking it, as with Chet, because the vocabulary has no caps condition; the healing prices and bands are read from the transcript text and not verified in game.

---

#### References

- `crates/engine/src/shared/` (`bounds.rs`, `condition.rs`, `effect.rs`, `eval.rs`, `sink.rs`, `view.rs`), `crates/engine/src/game/state.rs`, `schemas/common.schema.json`
- `data/dialogue/slice_goodsprings.json`, `tests/scenarios/goodsprings_dialogue.json`, `crates/engine/tests/game/health.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-28 | Initial entry | deltaog-117 |


---

### Vendors and Barter: A Shop That Remembers

**Date:** 2026-10-30
**Status:** Confirmed

---

#### Context / Background

The Goodsprings dialogue was finished except for the shop topics, which waited for vendors. The roadmap asked for buying and selling with the barter formula. Three shapes were weighed: a stateless price list (endless stock and caps), a vendor with its own stock and caps that change as you trade, and that plus restocking over time. The third needs a game clock the game does not have.

---

#### Decision

**A vendor owns stock and caps (the second shape).** A new feature module, `trade/`, holds `VendorBook` (the merchants, checked once at load), `Market` (each vendor's stock and caps now) and the price functions. The module never imports the inventory: `Game::buy` and `Game::sell` run both halves of a trade on a copy of the game and keep it only if everything worked, the same pattern quest changes use. The laws are checked by a property test: over random sequences of buys and sells, caps and items only change hands, and a refusal changes nothing.

**Prices come from two curated formulas.** `barter_buy_multiplier` is `1.55 - 0.0045*barter` and `barter_sell_multiplier` is `0.45 + 0.0045*barter` in `data/rules/formulas.json`: 155 percent to buy and 45 percent to sell at Barter 0, 110 and 90 percent at Barter 100. **The wiki page could not be fetched** (HTTP 402), so the numbers come from the Game8 Barter table, a secondary source. Its 20-point rows all lie on the line, except the 80-point sell row (91, where the line gives 81), which I treated as a typo. The formulas, their self-tests in `validate.py` and a Rust test that pins the constants to the file are all in place, so replacing them with the wiki's numbers is a data change plus one test. The price of a whole line is rounded once, so a stack of cheap rounds is not rounded up one by one; buying costs at least one cap; a line worth under half a cap cannot be sold.

**Item values.** `ItemBook` gained base values (`with_values`, `value`), loaded from the item records, so the inventory still owns what an item is worth and `trade/` only owns what a merchant does with it.

**Vendors are data.** `data/vendors/vendors.json` (copied to `data/engine/` by the normalizer, with `vendor.schema.json` and reference checks in `validate.py`) holds Chet, Trudy and Doc Mitchell. The wiki has no stock, so the lists are placeholders written from memory, to be replaced by the xEdit export. A dead or hostile NPC does not trade; neither does a player who is down; an equipped item cannot be sold if the sale would give up that copy.

**The dialogue hook is a topic field, not an effect.** Opening a shop changes no state, so `Effect::OpenTrade` (the first plan) would have put a UI request in the state vocabulary, with a sink method every state would have to answer. Instead a topic can name a `shop`, and `Conversation::choose` returns it in the `Reply`; the front end opens the shop. Doc's "I need medical supplies" is wired. Chet's and Trudy's only shop topics in the transcripts are the positive-reputation variants ("wholesale prices from now on"), so they wait for reputation instead of being wired to text that would lie.

**Left out for now:** the trade screen in the TUI (its own cycle), restocking and per-vendor rules (they wait for the game clock, now on the roadmap), barter perks and the reputation discount, item condition affecting price.

---

#### References

- `crates/engine/src/trade/`, `crates/engine/src/game/state.rs`, `crates/engine/src/inventory/book.rs`, `crates/engine/src/dialogue/conversation.rs`
- `data/vendors/`, `data/rules/formulas.json`, `schemas/vendor.schema.json`, `crates/engine/tests/trade/`
- Game8, Fallout: New Vegas Barter table (the source of the price numbers)

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-30 | Initial entry | deltaog-117 |


---

### Trade Screen: Two Panels, and the Screen Holds Only Cursors

**Date:** 2026-10-30
**Status:** Confirmed

---

#### Context / Background

Vendors and barter worked in the engine, but nothing in the TUI could reach them. Three shapes were weighed: one tabbed list that trades one unit per Enter, two side-by-side panels, and a tabbed list with a quantity prompt. The tabbed list with a prompt was my recommendation; **two panels was chosen** because it is closest to the trade window of the original game.

---

#### Decision

**Two panels, a prompt for stacks.** The vendor's stock is on the left and the player's pack on the right, each with its own cursor (Left/Right switches panel). Enter on a stack of one trades it at once; on a bigger stack it opens a "how many" question (Left/Right change the count by 1, Up/Down by 10, Enter confirms, Esc cancels). Moving a whole stack with no question would have made a 200-round stack a one-key mistake, so the question stays, as in the original game.

**The count is a type.** `Quantity` holds a value and the stack size, both private, and every method keeps the value between 1 and the stack size, so the screen cannot ask the engine to move nothing or more than the stack holds. A property test runs random raise and lower steps against it.

**The screen holds only cursors and the pending question.** Stock, pack, prices, caps and equipped marks are read from the game whenever they are drawn or chosen, the same rule the level-up screen follows. Every trade goes through `Game::buy` and `Game::sell`, so a refusal (too few caps, an equipped last copy, a vendor short on caps) reaches the log in the engine's own words, and the screen carries no price or affordability rule of its own. A property test drives random key sequences against the real Goodsprings data and checks that caps and items are conserved across player and vendor after every key, and that both cursors stay inside their lists.

**Two ways in.** The scene lists "Trade at ..." for each vendor in the place or in a building inside it (after the people, before the fights), and a dialogue topic with a `shop` opens the same screen. `dialogue::Next` gained `Shop(vendor)`, and the conversation that opened the shop travels inside the trade screen's state, so leaving the shop returns to the conversation with its menu rebuilt (some topics ask for caps, which the trade may have changed). If the topic also ended the conversation, leaving goes to the scene instead.

**The app now builds the vendors.** `VendorBook::from_data` and `Game::with_vendors` were never called by the real game; `App` does it now, plus a name table (`trade::Names`) because the engine knows items by id only. An item with no record is shown by its id.

**Left out for now:** the inventory screen (worn items are marked "[worn]" and the last worn copy is refused, but nothing can be equipped from the shop), a per-unit price that accounts for rounding the whole line (rows show the price of one unit; the question shows the exact total), and sorting or filtering the lists.

---

#### References

- `crates/tui/src/trade/`, `crates/tui/src/app.rs`, `crates/tui/src/dialogue/screen.rs`, `crates/tui/src/exploration/screen.rs`
- `crates/engine/src/game/state.rs` (`buy`, `sell`, `shop`, `vendors_at`)

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-30 | Initial entry | deltaog-117 |


---

### Pack Screen and Item Use: Hand-Written Uses, Merged into the Records

**Date:** 2026-10-30
**Status:** Confirmed

---

#### Context / Background

Bought stimpaks could not be used and a bought weapon could not be equipped, because the TUI had no pack screen. Using a consumable needs the game to know what an item does, and the imported data describes effects as loose text (`"stat": "HP and Limb Condition", "amount": 30`). Three ways were weighed: a hand-written table of uses (A), reading the imported effects at load (B), and reading them with hand-written overrides (C). **A was chosen**, as recommended.

---

#### Decision

**What an item does is data, written by hand.** `data/items/uses.json` lists a consumable and what using it does (`heal`: hit points restored at once). `scripts/normalize.py` merges each entry into the consumable's record as `on_use`, the same way perk modifiers are merged: an unknown item or a repeated entry fails the merge, so a typo cannot drop an effect. The Rust side is a typed `ItemUse` (`Heal(NonZeroU32)`) on `Consumable`, carried into `ItemBook`. B and C were rejected because the stat names are free text; a string match that misses an item would fail silently.

**Only fully modeled items are listed.** Four items: stimpak (30), banana yucca fruit (7), bitter drink (36), purified water (10). Healing powder is left out, although the plan named it at 18, because it carries a Perception penalty the game does not model; listing it would quietly drop the drawback. Most food adds radiation and is left out for the same reason. A per-second heal is entered as its total, since there is no clock yet; this is written in `data/items/SOURCE.md` so it is revisited when the clock lands.

**Using an item is a `Game` action, not a pack action.** `Game::use_item` needs the player's wounds, which the pack does not hold. It refuses a player who is down, an item not carried, an item with no modeled effect, and a player at full health (so a stray key press does not waste a stimpak), and it changes nothing on a refusal. Dropping is the pack's own action (`Inventory::discard`), and it refuses the last copy of an equipped item so a slot never points at something no longer held.

**The screen is an action menu per stack.** The existing commands have no letter keys for "use" or "drop", and adding some would touch every screen, so Enter on a stack opens its actions (Use, Equip or Unequip, Drop, Back); only the actions that apply are listed. A stack of more than one asks how many to drop with the same bounded count the trade screen uses. As in the trade screen, the pack holds only cursors and the pending question and reads everything else from the game when it draws.

**Two helpers moved to `shared/`.** `Names` and `Quantity` lived in the trade screen's folder; the pack screen needs both, and one screen importing from another breaks the rule that screens depend on `shared` and never on each other. `Menu::resized` is new in `shared/menu.rs`; the trade screen keeps its own private copy of the same few lines, which I left alone to keep the change small.

**The automatic pistol stays.** `take_weapon_in_hand` still wields the pistol when a fight starts with an empty hand, so unequipping it cannot send the player to the Goodsprings fight unarmed. Its comment no longer says the inventory screen is missing.

**Left out for now:** per-item weight and details beyond value and the heal, comparing armor values when equipping, resting, and every consumable whose effect needs a clock, radiation, addiction or the Medicine skill.

---

#### References

- `crates/tui/src/inventory/`, `crates/tui/src/app.rs`, `crates/tui/src/exploration/screen.rs`
- `crates/engine/src/game/state.rs` (`use_item`, `drop_item`), `crates/engine/src/inventory/`
- `data/items/uses.json`, `data/items/SOURCE.md`, `scripts/normalize.py` (`apply_uses`)

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-30 | Initial entry | deltaog-117 |


---

### Game Clock: Time Lives in `Game`, Not Yet in the Vocabulary

**Date:** 2026-10-31
**Status:** Confirmed

---

#### Context / Background

Nothing in the game knew what time it was, and several roadmap items wait on that: resting, vendor restocking, per-second item effects and timed bonuses. The open question was how far the clock reaches. Two ways were weighed: keep it inside `Game` (A), or also add a `time` condition and a `wait` effect to the shared condition/effect language (B). **A was chosen**, as recommended.

---

#### Decision

**A new `clock/` feature owns the time.** `GameClock` counts whole minutes since the start of day 1 at 00:00; day, hour and minute are derived from that one number, so they cannot disagree. A game opens at day 1, 08:00. `Span` is a length of time built only from a non-zero number of minutes or hours, so waiting for nothing cannot be constructed. `GameClock::advance` uses checked addition: an overflow is refused with `ClockError::Overflow` and leaves the clock as it was.

**`Game` holds the clock and decides when it moves.** `Game::wait` is the only way time passes, and it refuses a player who is down. `Game::clock` reads it. The clock is a plain `Copy` value, so cloning a game (which the batch effects do to try changes) copies the time with it.

**Why not the vocabulary yet.** B would have added a `Condition::Time` and an `Effect::Wait`, with schema, contract-test and view/sink-trait changes, before any rule needs them. The first real consumers (resting, restocking, per-second effects) are Rust code in `Game`. When the first data-driven timed rule appears, it will fix the shape of the condition, instead of this cycle guessing it. Changing the schema later costs one cycle; guessing wrong costs a rewrite.

**The scene gets two waits.** "Wait an hour" and "Wait 8 hours" sit between "Pack" and "Quit game". The status line shows the day and time after the place, and a wait writes "Wait 8 hours. It is now Day 1, 16:00." to the log. Existing tests that counted rows from the end were moved to the new positions.

**`from_minutes` and `as_minutes` exist for saves.** A saved game has to write the clock and read it back; both are plain accessors with no rule beyond the type.

**Left out for now:** resting that heals (it needs beds and a healing rule), travel that costs time, vendor restocking, per-second item effects, hardcore needs, and any `Condition`/`Effect` for time. The per-second heals entered as totals in `data/items/uses.json` stay as they are until a rule needs the difference.

---

#### References

- `crates/engine/src/clock/`, `crates/engine/src/game/state.rs` (`clock`, `wait`)
- `crates/engine/tests/clock/`, `crates/engine/tests/game/clock.rs`, `crates/engine/benches/clock.rs`
- `crates/tui/src/app.rs`, `crates/tui/src/exploration/screen.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-31 | Initial entry | deltaog-117 |


---

### Sleep: Beds Are Data, and Only Sleeping Heals

**Date:** 2026-10-31
**Status:** Confirmed

---

#### Context / Background

The clock landed with nothing to heal on it. The question was how the player heals by letting time pass. Three ways were weighed: every wait heals with a constant in code (A), a separate Sleep action that heals while waiting does not (B), or every wait heals with the rate read from a data file (C). **B was chosen**, as recommended. The data has no rest rule, but the perk texts say a sleep "regains all of your Health" and speak of "the bed", so sleeping is tied to a bed and waiting is not.

---

#### Decision

**A new `rest/` feature owns beds and the recovery rule.** `BedBook` lists the beds, each at a place; `restored(max_hp, span)` turns a sleep into hit points: linear in the length, rounded up, never above the maximum, with eight hours (`FULL_REST_HOURS`) restoring everything. Eight hours is a stand-in: the wiki gives no length, only that a sleep restores all health.

**`Game::sleep` runs a sleep as one unit.** It refuses an unknown bed, works out the recovery, lets the time pass through `Game::wait` (so a player who is down is refused and an overflow changes nothing), then closes the wounds and reports what was actually healed. `Game::wait` is unchanged and heals nothing, so stimpaks and Doc Mitchell keep their value. `Game::beds_at` offers the beds at a place, none while the player is down, like `vendors_at`.

**Beds are hand-authored data, as vendors are.** `data/beds/beds.json` is copied to `data/engine/beds.json` by `normalize.py`, checked by `schemas/bed.schema.json`, and `validate.py` checks that each bed stands in a known place. The one record, Doc Mitchell's spare bed, is a placeholder. The real list, with owned and rented beds, is to be fetched together with the other data the perks need (Well Rested, campfire sleeping).

**The scene scrolls.** A "Sleep in ..." row follows the fights. The new row pushed "Quit game" off the 30-line test frame, and the scene was already cut off at the smallest supported terminal, so the rows now scroll with the cursor through the existing `menu::window`. The first-frame test was given a taller terminal (34 lines, was 30) because the town now lists one more row than a 30-line scene can show at once; it still checks every text it checked before. A new test shows the cursor row stays visible at the smallest size.

**Left out for now:** campfire sleeping, owned and rented beds, Well Rested, hardcore mode (where sleep does not heal), a choice of how long to sleep (the scene offers the full rest only), travel that costs time, and timed item effects, which are the next step of the clock work.

---

#### References

- `crates/engine/src/rest/`, `crates/engine/src/game/state.rs` (`with_beds`, `beds_at`, `sleep`)
- `crates/engine/tests/rest/`, `crates/engine/tests/game/rest.rs`, `crates/engine/benches/rest.rs`
- `data/beds/`, `schemas/bed.schema.json`, `scripts/normalize.py`, `scripts/validate.py`
- `crates/tui/src/app.rs`, `crates/tui/src/exploration/screen.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-10-31 | Initial entry | deltaog-117 |


---

### Reputation: Points Underneath, Seven Standings and Exact Tiers on Top

**Date:** 2026-11-01
**Status:** Confirmed

---

#### Context / Background

`Game` still refused reputation and karma effects. The wiki models reputation as two scores per faction, fame and infamy, each falling into one of four tiers, and the pair of tiers picks a title from a 4x4 grid. The engine's rule language only had seven standings (Vilified to Idolized). Two ways to bridge that were weighed: keep the seven standings and show the 16 titles to the player only (A), or add exact tier and score conditions beside the seven (C). A third, replacing the seven with tiers, was dropped: it breaks the one existing rule and the schema for no gain over C. **C was chosen**, as recommended. The deciding fact was that the one reputation rule already shipped, Ringo not offering the gunfight at "Merciful Thug or worse", is not expressible in seven standings: Merciful Thug and Wild Child have high infamy and some fame, and the seven standings read both as Mixed.

---

#### Decision

**Points are the state; standings are derived.** `Standing` keeps fame and infamy points per faction and one karma number. The tier is how many later thresholds a score has reached (the wiki's thresholds, per faction). The seven standings are a pure function of the two tiers: fame alone reads Accepted, Liked or Idolized by tier; infamy alone reads Shunned (tiers one and two, which covers "Hated") or Vilified (tier three); both together read Mixed. The wiki's title grid is kept per faction and read for display only.

**Two exact conditions beside the seven standings.** `reputation_tier` compares the fame or infamy tier with 0 to 3, and `reputation_points` asks for a score of at least some points (the wiki's "a fame score of at least 40"). Both are in the shared vocabulary, the schema and the contract tests, and the existing `reputation` condition is unchanged. The Ghost Town Gunfight gate now reads `reputation_tier` on Goodsprings infamy, tier 3. "Or worse" is read as the whole infamy-3 row, Wild Child included; the wiki does not say.

**Laws.** A gain only adds, and each score stops at 10,000 so repeated gains cannot reach infinity (the highest tier threshold is 100). Forcing a standing (`SetReputation`) replaces both scores with the fewest points that reach it, and is the only way to lower one. Karma is bounded to -1000..=1000. An effect aimed at a faction without a model, with a negative gain or with a non-finite number is refused and changes nothing; a gain that is half valid is refused whole.

**Karma thresholds are a stand-in.** Good from 250, evil from -250. The wiki's karma page is not imported and the stats data has no karma entry, so these come from the game's commonly quoted values and are marked as such in the code, the roadmap and the changelog. Karma-per-action values are still in the low-priority list.

**Goodsprings amounts are one placeholder.** The wiki lists no fame or infamy gains or losses for Goodsprings, so the only amount authored is 3 fame on the gunfight's last stage (one step into the first tier). Nothing in the quest data ever produced a deferred reputation effect, so there was nothing to replay from `Game::deferred()`; the roadmap item is closed on that ground.

**A game with no book refuses reputation.** `Game::new` holds an empty book, as it holds an empty roster, so a reputation effect on it is refused as an unknown faction, while karma works without a book. Tests that run the gunfight to its end now build the book from the real data. Two tests that used karma as their example of an unbuilt feature now use companions.

**Left out for now:** a reputation screen, the per-faction consequences the wiki lists (Chet's and Trudy's 25% discounts, the Followers, NCR and Legion effects), gain and loss amounts for the other 12 modeled factions, faction disguises, and the real karma thresholds.

---

#### References

- `crates/engine/src/reputation/`, `crates/engine/src/shared/` (`condition.rs`, `eval.rs`, `view.rs`, `vocabulary.rs`, `bounds.rs`), `crates/engine/src/game/state.rs` (`with_reputation`, `standing`)
- `crates/engine/tests/reputation/`, `crates/engine/tests/game/reputation.rs`, `crates/engine/tests/shared/`, `crates/engine/benches/reputation.rs`, `tests/scenarios/goodsprings.json`
- `schemas/common.schema.json`, `data/quests/slice_goodsprings.json`, `data/engine/factions.json`
- `crates/tui/src/app.rs`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-11-01 | Initial entry | deltaog-117 |


---

### Four Features in One Cycle: Standing Prices, a Place on the Map, Timed Boosts and Foes Who Can Be Absent

**Date:** 2026-11-02
**Status:** Confirmed

---

#### Context / Background

The roadmap's next four items were built together because they meet at the same places: the scene, `Game`, the trade and combat data. Three of them had a real fork, and the choices were made from COAs (the fourth, reputation consumers, had one credible approach). **Travel: B**, the game owns the player's place (rather than the screen owning it, or adding random encounters, which needs data that does not exist). **Timed effects: A**, boosts that expire with the clock and heals over time entered as their total. **Story-dependent fights: A**, enemies who can be absent (rather than real allies, which is the whole "Finish combat" item, or leaving the helpers unused).

---

#### Decision

**A price cut is data, earned by the conditions rules already use.** A vendor record may carry `discount`: a percent from 1 to 90 (`DiscountPercent`, bounded by construction) and `when` conditions, any one of which earns it. `Game::buy_price_from` prices a purchase at a given vendor and `Game::buy` charges exactly that; `buy_price` stays the list price, and selling is never cut. The cut is applied to the whole line before the single rounding, so a stack of cheap items is not rounded up one by one. Chet and Trudy get 25% for "Accepted or better, or Mixed". The wiki gives no percentage (only "special discount" and "wholesale prices"); 25% is the roadmap's figure and is marked as a stand-in. Their shop dialogue topics use the only transcript lines that exist, the positive-reputation ones, so they are shown only to such a player. The reputation screen reads the standing from the game on every draw and holds only a cursor.

**The player's place lives in `Game`.** `TravelBook` turns the derived neighbor links into a two-way map checked once at load (every link joins two places that exist, over a finite distance above zero); `Game::travel` moves the player and advances the clock by the walk, or does neither. The pace is two minutes a map pixel, rounded up and never zero, so the cave is 72 minutes from Goodsprings. This is a stand-in: the wiki gives no speed. The screen's own "where am I" field is kept for names and lookups only and a test checks it agrees with `Game::at`. The loaded data has people and shops only for Goodsprings, so the places around it are empty; arriving looks up everything again, so the data can grow without touching the screen.

**Boosts end by themselves, and the clock has one door.** `ActiveEffects` keeps each boost with the minute it ends; nothing in force ends at or before the time it was last expired to, a new dose of an item replaces the last (it does not stack), and the tally is exactly the sum of what is in force. All time passes through `Game::wait`, which sleeping and travelling also call, so there is one place where boosts end. `Character` reads a `BoostTally` for attributes, skills, maximum hit points, action points and carry weight, and never changes what the player earned, so ending a boost puts back exactly what was there. A chem's damage resistance adds to the armor's when a fight is prepared. A boost to maximum hit points is a loan: the wounds stay, so the extra hit points go with it, and the repayment cannot kill (the player keeps at least one hit point). A first version kept the boosted hit points instead of the earned ones; a property test of the loan law caught it before it reached the screen.

**Heals over time are entered as their total.** The clock counts minutes and no heal in the data runs longer than one, so spreading a heal across ticks would be machinery for a case the data does not have. Effects the wiki gives in seconds are rounded up to one minute (healing powder's 18 seconds of -1 Perception becomes a minute). Only items whose whole text is modeled are listed: twelve now. Anything that adds radiation, poisons or changes damage dealt stays unlisted, and chems are usable without their addiction chance until addiction exists. The assertion that healing powder is unlisted was a statement of the old placeholder, so it now states the new truth.

**A foe can be absent.** A foe may carry `when` conditions and is in the fight only while they hold; an encounter must keep at least one foe with none (checked when the arsenal is built, so a fight with nobody in it cannot exist). The wiki says six attack the town (five if Cobb is dead), so the Ghost Town Gunfight has Cobb and two gangers always, and three more unless Trudy joined the militia, Easy Pete joined, or Chet supplied it. A town that did all three faces the old three; one that did nothing faces six. Which helper removes which ganger is a stand-in. The two Run Goodsprings Run fights use the same field and the same machinery; defenders' loadouts and hit points are stand-ins, and winning the militia fight kills Sunny, Easy Pete and Trudy (so Trudy's saloon stops trading), while Chet and Doc live, as the wiki says.

**Left out for now:** real allies in combat; random encounters on the road; radiation, poison and addiction; the other per-faction reputation consequences; crafting (so the campfire quest's stage 30 stays); and Victor and the other settlers as defenders.

---

#### References

- `crates/engine/src/trade/`, `crates/engine/src/travel/`, `crates/engine/src/effects/`, `crates/engine/src/shared/boost.rs`, `crates/engine/src/game/` (`state.rs`, `battle.rs`), `crates/engine/src/character/sheet.rs`
- `crates/engine/tests/trade/discount.rs`, `crates/engine/tests/travel/`, `crates/engine/tests/effects/`, `crates/engine/tests/game/boosts.rs`, `crates/engine/tests/game/battle.rs`, `crates/engine/benches/travel.rs`, `crates/engine/benches/effects.rs`
- `crates/tui/src/` (`app.rs`, `reputation/`, `travel/`, `quests/journal.rs`, `exploration/`, `inventory/`, `trade/`)
- `data/vendors/`, `data/combat/`, `data/items/`, `data/dialogue/slice_goodsprings.json`, `data/quests/slice_goodsprings.json`, `schemas/` (`vendor`, `encounter`, `item`)

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-11-02 | Initial entry | deltaog-117 |


### Weapon Realism and Save Games: Checked Loading, and a Wear Rate Nobody Gave Us

**Date:** 2026-11-02
**Status:** Confirmed

---

#### Context / Background

Two roadmap items were built in one cycle: "Save and load" and a slice of "Finish combat". Both had real forks and were chosen from COAs. They belong together because wear on a weapon is state a save must carry: building the combat slice first meant the save format was written once, with the wear already in it, instead of being migrated straight after.

---

#### Decision

**The hit chance reads skill and spread, and says it is a stand-in.** `AimedHitChance` gives `40 + skill / 2 - 3 * spread`, kept between 5 and 95 so no shot is hopeless or certain. The wiki has no formula; the constants were chosen so a mid-skill shooter with a steady weapon lands about two in three, as the old flat 65% did, and the two things the data supports hold as laws (skill never hurts, spread never helps). The starting pistol at Guns 15 now hits 45%. Foes read the same model through the character their record builds. The curve is marked as a stand-in in the code and the roadmap and should be replaced when the xEdit export exists.

**A reload costs its seconds as shots, and never more than a turn.** The data gives reload time in seconds and the shot cost in action points. A first version priced a reload as the shots that fit in the same time (reload time times shots a second times the shot cost) and gave the 9mm pistol 89 action points, more than a whole turn, so a reload could never be paid and a fight could not end. The random-fight property test caught it. The price is now one shot's cost per second of reload (28 for the pistol), and it is capped at the fighter's action points per turn, because a reload that can never be paid would leave a weapon dead once its clip ran out. A test checks every fighter in the real gunfight can pay for a reload.

**Weapons wear one point a shot, per weapon item.** `Condition` is the points lost out of the points a weapon had new, and its multiplier runs from full to half, the range the damage formula allows. The wear rate is a stand-in: the wiki gives none. Wear is kept per item id, so two copies of a weapon share it; instances would need a different pack. Only the player's weapon wears, and the shots are counted by the battle and charged when it is settled, the same way spent rounds are. Nothing repairs a weapon yet, so a long game drifts every weapon toward half damage; repair is on the roadmap and the save already carries the wear. The nine weapons with no action-point cost stay out of fights: the data gives no cost and none was invented.

**A save is a record we control, loaded through the checks.** The save is a plain record with a version number, written as JSON. It holds only what changes in play; what the data files say is rebuilt from the data. Loading starts from a game built from the data and puts each part back with the operations play uses: the quest log refuses a stage that does not exist, the pack an item the game has never heard of, the character a perk it may not have. That is why the record is not simply the live types with a derive: serde would build states the constructors refuse, and every refactor would break old saves. The cost is about fifteen small record types and a conversion each way.

**What loading checks beyond the parts.** A quest status must fit its stage (a stage that finishes a quest cannot belong to one still going). The level is derived from experience, so it cannot disagree with it, and the perks taken plus the perks waiting must equal the perks the levels reached give. A skill must have been under 100 before its last point. A boost that already ended, wounds past the maximum, wear past what a weapon has, and a save with no place for a game that has a map are refused. Not checked: the banked skill points, which depend on what perks added at each level and cannot be worked out afterwards; they are a plain counter that cannot break anything.

**Rewards are not paid again.** Stage rewards and fired rules are already in the pack, the standing and the flags the save holds, so loading sets the quest log directly instead of replaying `Game`'s reactions, and the rules that fired are saved so none fires twice.

**Saves are written whole or not at all.** The text goes to a temporary file and is renamed over the save once it is complete on disk. A save made while a fight is on screen would hold the state from before the fight, because a fight is not part of the game until it is settled; the scene is the only place with Save and Load rows, so in practice saving happens between fights. The version number is checked first: a newer file is refused, an older one is upgraded one version at a time (there is none yet, so the step table is empty and says where to add one), and unknown fields are rejected so a typo cannot silently lose data.

**The player-facing part is deliberately small.** One slot at `$XDG_DATA_HOME/fnv/save.json`, replaced without asking. Several slots, a list to pick from and autosave are on the roadmap. The dice and the log are not saved. Loading puts the scene back where the save was made, because the people, shops and beds shown depend on the place.

**Tests that had to change.** The scene gained two rows before Quit, so the scene tests count from the new end, and three app tests that count rows up from Quit moved by two. The first-frame test uses a taller terminal (40 rows, not 34) because the longer list scrolls and "Quit game" is the last row. Each still asserts what it did before.

**Left out for now:** body-part targeting, initiative, fleeing, allies, repair, saving during a conversation, save slots, and the real hit-chance numbers.

---

#### References

- `crates/engine/src/combat/` (`hit.rs`, `condition.rs`, `stats.rs`, `fight.rs`), `crates/engine/src/game/` (`battle.rs`, `state.rs`, `state/save.rs`), `crates/engine/src/saves/`
- `crates/engine/src/` hooks: `quests/log.rs`, `world/state.rs`, `trade/market.rs`, `effects/active.rs`, `inventory/state.rs`, `character/sheet.rs`, `reputation/state.rs`, `shared/boost.rs`
- `crates/engine/tests/saves/`, `crates/engine/tests/game/battle.rs`, `crates/engine/benches/saves.rs`
- `crates/tui/src/` (`app.rs`, `exploration/screen.rs`, `combat/screen.rs`)

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-11-02 | Initial entry | deltaog-117 |


### Repair, Crafting and Gathering: Stand-In Rates, Stations as Data, and a Stage That May Come Late

**Date:** 2026-11-03
**Status:** Confirmed

---

#### Context / Background

Three roadmap items were built in one cycle: repair, crafting and the rest of the Goodsprings quest slice. They share a thread: By a Campfire on the Trail needs a broc flower and a xander root, a campfire, and healing powder; a weapon wears down and needs a way back. The forks were chosen from COAs: repair by kit with merging and paid vendor repair, stations as data at places, and pickup spots as data.

---

#### Decision

**Repair is arithmetic over condition points, with every rate marked a stand-in.** The wiki gives no repair formula, so the laws follow the shape it describes. A kit lifts a weapon to the Repair skill's share of its condition, rounded up. Merging two copies adds 10% of the new condition on top of what is left of both, held to the same ceiling and never below the better copy. A vendor charges half the weapon's value for a full repair, rounded up to at least a cap, and mends only up to their own Repair skill (Chet, 50). The module imports no other feature: the laws take plain points, and `Game` owns the wear record, the kit and the caps, running each repair on a copy so a refusal changes nothing.

**Two copies of a weapon share one wear record, and the merge says so.** Wear was already kept per item id. Merging with one shared record treats the pair as equally worn, which matches the real rule when they are, and is wrong when a spare is fresh. Per-copy wear would touch the pack, the battle and the save, so it went on the roadmap instead of into this cycle. The merge function itself takes two conditions, so it needs no change when that lands.

**A recipe is a formula or has a reason.** Of 168 imported recipes, 84 can be made. The others are breakdowns (not built), have no output or station, or list an ingredient the wiki's table did not match to an item. Each carries its reason and the property test checks that no recipe is both or neither. One real gap was a data bug: the weapon repair kit's "Wrench (1)" carried the wiki's footnote marker, so it matched nothing; the normalizer now strips the marker. Another consequence: crafting components are now items the player can carry (123 more), since most recipes use them.

**Stations and plants are data, like beds.** The quest says "use the campfire", so crafting checks a station of the recipe's kind at the player's place, and a pickup is a one-time find at a place. Both lists are hand-authored placeholders; the wiki has neither.

**The story moves through a reaction on the recipe.** Making healing powder sets stage 30 of the campfire quest, but only while the quest is between 10 and 30; a reaction has `when` conditions beside its effects, and the whole craft is one unit, so a failing reaction makes nothing. The reaction is merged from `data/crafting/effects.json` into the recipe, the way item uses are merged into consumables.

**A late optional stage is not a step back.** Run Goodsprings Run's stages 30 (Chet) and 40 (Doc) are side tasks in either order, but a stage can only move forward, so a shakedown after the supplies would have been refused and would have failed the whole topic. The stage record gained `optional`: reaching it when the quest is already past it changes nothing. A stage that is not optional is still refused behind one.

**The save grew to version 2.** It gained the list of finds taken; a version 1 save is upgraded on reading. This is the first use of the upgrade table the last cycle left empty.

**Tests that had to change.** The scene gained rows, so the two tests that want the whole list on screen use a terminal 50 rows tall, and the character-row helper counts the new rows. The loader test's scratch directory writes the four new catalogs, and the item-book count includes the miscellany. Each still asserts what it did before.

**Left out for now:** Victor and the other settlers as defenders (they need real allies), the six quests with no stages (the stage engine has nothing to track on them), taking things apart, tools that are not expended (the wrench is used up), and per-copy wear.

---

#### References

- `crates/engine/src/repair/`, `craft/`, `gather/`, `game/state/upkeep.rs`, `quests/log.rs`, `inventory/book.rs`, `saves/record.rs`
- `crates/engine/tests/craft/`, `gather/`, `game/upkeep.rs`, `saves/`, `quests/lifecycle.rs`, `crates/engine/benches/repair.rs`, `craft.rs`
- `crates/tui/src/craft/`, `repair/`, `inventory/screen.rs`, `exploration/screen.rs`, `app.rs`
- `data/stations/`, `data/finds/`, `data/crafting/`, `data/vendors/`, `data/dialogue/slice_goodsprings.json`, `schemas/`, `scripts/normalize.py`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-11-03 | Initial entry | deltaog-117 |


### Weapon Instances, Restocking, Fleeing and Named Saves: Four Features in One Cycle

**Date:** 2026-11-03
**Status:** Confirmed

---

#### Context / Background

The last cycle left four loose ends: two copies of a weapon shared one wear record, shops only ever emptied, a fight could not be left, and the game had one save file. They were built together because three of them touch the save record, so it moved once, to version 3. Each fork was chosen from COAs: weapon objects with ids (B), a folder of named saves (B), a flee roll (A) and a global restock (A). The fourth item's other parts, initiative and body-part targeting, were split: initiative was built and body-part targeting became a spike, because no source gives limb numbers.

---

#### Decision

**Every weapon is an object; everything else stays a count.** The pack keeps ammo, armor and chems as stacks and keeps each weapon as a record with an id and the points it has lost. Two things made this cheaper than it sounds. First, `item_count`, `give_item`, `take_item`, `discard`, `equip` and `weight` keep their meaning, so the quest, craft and dialogue code that speaks in counts did not change. Second, wear never moved into a map keyed by item again: the condition lives on the copy, so a merge, a sale and a drop move it with the copy by construction. The cost is rules for the count-shaped calls: taking a weapon away by kind removes the most worn spare first and the one in hand last, and equipping a kind takes the least worn copy. Ids are never reused within a pack and a save keeps them, so a loaded game saves to the same text.

**A shop does not remember wear.** A sold weapon joins the vendor's stock as an ordinary item and comes back new if bought. That is a hole in a realistic economy but not in conservation of caps or weapons, and a price that follows condition needs a per-item stock the market does not have. It is on the roadmap.

**The save moved once, and the upgrade is data-free.** Version 3 lists weapons with their ids and names the one in hand by id. The version 2 step runs on the JSON alone, so it cannot know which items are weapons; it uses the one thing that says so, the old wear list, and turns each wear entry into one record per carried copy of that kind. A weapon with no wear entry was new and stays a plain count in `items`, which loads as new copies. I said earlier that old saves would be refused; the save module already documents an upgrade step per version, so following it was less code than breaking it.

**Restocking is a function of the clock.** A vendor refills on every multiple of its interval since the game began (3 days unless the record says), checked whenever time moves. Nothing is stored: a save holds the shops as they were, the clock holds the day, and the next move of the clock decides. That makes "many small waits equal one big wait" a property test and not a promise. A refill replaces the shop, so what the player sold is gone and the vendor's caps return to the starting figure; per-vendor rules about what a vendor buys wait for the full economy.

**Initiative is a turn order, not a rewrite.** `Combat` keeps its fighters in the order given, so a `FighterId` still means the position the caller put a fighter in, and adds a separate turn order sorted by Sequence (highest first; ties keep the order given). The player's Sequence is twice Perception, which is the familiar formula but is not in the curated formulas file, so it is marked a stand-in. The cost is that a quicker foe now shoots before the player's first turn: `Battle::open` plays those turns, and the combat screen calls it at the start. Several existing tests assumed the player always opened, so they start fights through a helper that plays the opening with dice that miss.

**Running away is one roll.** `25 + 6 * Agility`, kept between 5 and 95, a stand-in like the hit curve. A failed run costs the rest of the turn and the foes then play; a successful one ends the fight with a new `Fled` status, settled as an outcome that keeps the wounds, the rounds fired and the wear, wins nothing and leaves the encounter on offer. The 5 and 95 mean a fight is never certain to be escapable or inescapable.

**Named saves are a folder, and a name is a checked type.** `SaveName` allows letters, digits, spaces, `-` and `_`, trims the ends, caps the length at 40 and refuses `autosave` in any case, so any name that gets through is safe as a file name and stays inside the folder (a property test over arbitrary text). The automatic save has a slot of its own that a typed name can never reach. The list reads each file's summary (day, time, place, experience) without loading a game, and shows a damaged file with its reason so the player can see it exists.

**Typing needed a second input path.** The key map turns `q`, `j`, `k`, `h`, `l`, space and digits into commands, so a name cannot be typed through it. While a name is open the event loop hands raw keys to `App::type_key`, which keeps `Command` as it was (no exhaustive match in the other screens had to change). Ctrl+C still quits.

**Autosave is off until `main` turns it on.** Tests build `App` many times and walk and sleep in them; an autosave on by default would write into the real data folder. `with_autosave` is called from `main` only. It runs after a walk, a sleep and a settled fight while the player is alive; a failure goes to the log and the game goes on.

**Tests that had to change.** The pack no longer lists weapons as stacks, so the helpers that compared stacks to a model count weapons by kind. The save tests tamper with `pack.weapons` instead of a wear list, and the "gear not carried" refusal gained a sibling for a weapon in hand that is not carried. The scene tests that save and load go through the new list. The trade restock tests, copy tests, initiative tests and slot tests are new.

**Left out for now:** body-part targeting (a spike), a message when shops restock, deleting or renaming a save, and reading the old single `save.json`.

---

#### References

- `crates/engine/src/inventory/copies.rs`, `inventory/state.rs`, `game/state.rs`, `game/state/upkeep.rs`, `game/state/save.rs`, `game/battle.rs`, `combat/fight.rs`, `trade/market.rs`, `trade/book.rs`, `saves/record.rs`, `saves/folder.rs`
- `crates/engine/tests/inventory/copies.rs`, `trade/restock.rs`, `combat/initiative.rs`, `saves/folder.rs`, `saves/files.rs`, `game/battle.rs`, `game/upkeep.rs`
- `crates/tui/src/inventory/screen.rs`, `trade/screen.rs`, `repair/screen.rs`, `combat/screen.rs`, `saves/screen.rs`, `app.rs`, `main.rs`, `shared/input.rs`
- `schemas/vendor.schema.json`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-11-03 | Initial entry | deltaog-117 |


### Conditional Perks, Healing by Medicine, Taking Things Apart and Save Management: Four Features in One Cycle

**Date:** 2026-11-04
**Status:** Confirmed

---

#### Context / Background

Four loose ends were taken together: most perks and traits changed nothing, the only ways to heal were items, Doc and a bed, sixteen recipes were refused as "not built", and saves could be added to but not tidied. Three forks were chosen from COAs: conditional modifiers (C) for perks, healing the way the real game does it, and the defaults for crafting and saves.

---

#### Decision

**A modifier can say when it applies.** The perk vocabulary gained eleven kinds, and six of them take an optional weapon skill, kind of foe or item. The condition is a plain field and not a second condition language, because the three things a perk needs to name (what is in hand, what is being fought, what is being used) are closed sets. Leaving the field out means always. The price was that `Modifier` can no longer be copied (it can hold an item id), which touched only the places that matched on it. A character sums the modifiers for the weapon in hand into one `Edge` when a fight starts, so the fight reads numbers and never walks the perks, and the foe's kind is the only thing the fight resolves at each attack.

**Only perks that can be taken and fully carried were modeled.** Fifteen perks and traits were added. The ones left out fall in three groups: effects the game cannot carry (a player's health below half, the time of day, the armor worn, particular weapons, sneak attacks, limbs), perks with a second effect that is not built (Bloody Mess makes enemies explode, Robotics Expert shuts robots down), and perks that ask for a kill count, a challenge or caps, which the game never offers. The last group is why no perk that hits a kind of foe harder was modeled except Entomologist and Hunter, and why no current encounter names a foe kind other than a person: the machinery is tested and waits for content. Where the wiki gives a number but not where it enters, a choice was made and is marked a stand-in: Finesse is a flat +5 critical chance (the wiki says "equivalent to +5 Luck"), Better Criticals and Hunter scale the critical bonus and not the whole hit, and Fast Shot and Trigger Discipline carry their V.A.T.S. accuracy through spread because the game has no V.A.T.S.

**Healing follows the real game: nothing heals by itself.** I had proposed a capped rest and passive recovery; the choice was "closest to the real game", which has neither. Waiting and walking still heal nothing, so stimpaks, Doc and beds keep their value. What the wiki does say is that Medicine changes how well stimpaks work, so a stimpak restores 30 plus one percent per two points of Medicine (a stand-in; the wiki gives no formula), and Fast Metabolism adds 20 percent. The heal is rounded once, to the nearest point, so a bonus never rounds a heal away. The stimpak's record is `medicine_heal` in the data.

**Taking things apart is a recipe read the other way.** The schema already said it: a breakdown "is taken apart into the listed items". So the item named is spent (as many as `output_qty`, one when absent) and the listed parts come back. The risk was a bench that prints parts, so a test compares every round that can be both made and taken apart: it never gives back more than making it used (a 9mm round costs 13 lead and gives back 11). Twelve of the sixteen work; four name no item to take apart (the wiki's table matched none), and say so.

**A tool that stays is a marked ingredient.** The wiki marks "(2, not expended)" on some ingredients, and the importer now reads it as a carried tool of that count. Only two recipes carry the mark and both are still blocked by something else, so no recipe changed. The weapon repair kit's wrench stays used up: the wiki's table does not mark it, and I did not guess. I had noted the wrench as a case of this earlier; it is not, on the evidence we have.

**Saves: delete, rename, a previous autosave and the old file.** Delete and rename sit on `d` and `r` on the list and each asks first; a rename never replaces another save (checked before the move, since a move over a file would do it silently). Before the autosave is written, the existing one is copied to `autosave_previous` through a staging file, and the new one is written last and whole, so there is always at least one good copy; an autosave that cannot be read is not copied over a good previous one. The old `save.json` sits one level above the folder, so it is listed as "Old save" even before the folder exists, and can be loaded or deleted but never written. `Command` gained two variants, which meant adding them to four matches that list every command.

**Tests that had to change.** The stimpak tests expected a flat 30. The TUI save tests now keep the saves folder inside a folder of their own, so a stray `save.json` in the temp folder cannot show up in a list. The real-data tests that pin which perks and traits carry modifiers were extended, and the wiki text of each new one is pinned.

**Left out for now:** the perks and traits listed above, the four breakdowns with no item, the 51 recipes with a gap in the wiki's table, weapon mods, and stepping back through more than one earlier autosave.

---

#### References

- `crates/engine/src/shared/modifier.rs`, `shared/vocabulary.rs`, `character/edge.rs`, `character/bonus.rs`, `combat/damage.rs`, `combat/fight.rs`, `game/battle.rs`, `game/state.rs`, `game/state/upkeep.rs`, `craft/book.rs`, `saves/folder.rs`
- `crates/engine/tests/character/edge.rs`, `combat/perks.rs`, `game/battle.rs`, `game/upkeep.rs`, `craft/`, `saves/folder.rs`, `shared/contract.rs`
- `crates/tui/src/craft/screen.rs`, `saves/screen.rs`, `app.rs`, `shared/input.rs`
- `schemas/common.schema.json`, `encounter.schema.json`, `item.schema.json`, `rules.schema.json`, `data/perks/modifiers.json`, `data/items/uses.json`, `scripts/normalize.py`

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-11-04 | Initial entry | deltaog-117 |


---

## 🧠 Usage Guidelines

### When to Write an Entry

- **Before committing to a major design choice** – a library, a data model, a dependency.
- **When you revisit a decision** – document the reversal and the reasons.
- **After a significant learning** – so future work avoids the same pitfall.
