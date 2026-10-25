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

## 🧠 Usage Guidelines

### When to Write an Entry

- **Before committing to a major design choice** – a library, a data model, a dependency.
- **When you revisit a decision** – document the reversal and the reasons.
- **After a significant learning** – so future work avoids the same pitfall.
