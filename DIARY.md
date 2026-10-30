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

## 🧠 Usage Guidelines

### When to Write an Entry

- **Before committing to a major design choice** – a library, a data model, a dependency.
- **When you revisit a decision** – document the reversal and the reasons.
- **After a significant learning** – so future work avoids the same pitfall.
