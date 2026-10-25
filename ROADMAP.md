# Roadmap

This document outlines the future direction of fnv-terminal-ai-game, a
turn-based terminal game inspired by Fallout: New Vegas, written in Rust.
Items are organized by priority, not by timeline.

---

## ✅ Completed (Milestones Achieved)

- ✅ Game data imported from the Fallout Wiki into raw JSON: 1,182 items (weapons, armor, ammo, consumables, weapon mods, skill books, crafting components), 256 quests, 880 NPCs, 517 dialogue transcripts, 684 locations with map positions, 201 perks, 16 traits, 42 factions and towns, 31 creatures, stats, levels, challenges, recipes and ending slides
- ✅ 116 hand-curated quest consequence rules (fails, locks, unlocks, exclusive paths)
- ✅ 12 JSON Schemas, including the shared `condition` / `effect` vocabulary
- ✅ Normalizer and validator (Python tooling): stable ids, cross-references between files, and formula self-tests
- ✅ Hand-curated engine formulas (skills, HP, AP, carry weight, resistances, XP, DR/DT damage) cross-checked against the wiki's own examples
- ✅ Derived travel graph over the 1024x1024 map markers
- ✅ Stack decision: Rust with `ratatui` for the game, Python for data tooling only
- ✅ Cargo workspace scaffold: a headless `engine` library and a thin `tui` binary, with one module per game capability, shared contracts, and a composition root
- ✅ `scripts/check`: regenerates and validates the data, then runs rustfmt, clippy and the tests, printing output only on failure
- ✅ Decision log (`DIARY.md`) covering the language, the contracts-based architecture, the data sources and the quality gate
- ✅ Typed data model in `crates/engine/src/shared/`: distinct id types, bounded numbers, the `Condition` and `Effect` vocabulary, and contract tests (property tests plus two-way schema checks) that fail when the Rust types and `common.schema.json` drift apart
- ✅ Data loader (`shared/load.rs`): reads `data/engine/` into typed catalogs, scoped to everywhere or to a location such as Goodsprings, with errors that name the file and record id; a load benchmark (about 25 ms for Goodsprings, 49 ms for everything)

- ✅ Condition/effect evaluator (`shared/eval.rs`): `evaluate` over six small read-only view traits with default answers, `apply` over six write-side sink traits (exhaustive, so a new `Effect` variant does not compile until handled), and `apply_all`, which checks a whole list against the state before applying any of it; law, round-trip and batch property tests, and an evaluation benchmark stub
- ✅ Quest engine (`quests/`): `QuestBook` validates quest stages and rules once; `QuestLog` implements `QuestView` and `QuestSink` with terminal completed/failed, forward-only stages and finish-stage completion; lifecycle, property and real-data tests, and a playthrough benchmark stub
- ✅ Game state (`game/`): `Game` holds the quest log and implements every view and sink trait; unbuilt features answer reads with defaults and refuse writes, so `evaluate` and `apply_all` already run against it; batch benchmark stub
- ✅ World state (`world/`): `WorldBook` (NPC roster) and `WorldState` (story flags, NPC fates, ending locks) implementing `WorldView` and `WorldSink`; `Game` now delegates world reads and writes to it; lifecycle, property and through-`Game` tests, and a benchmark stub
- ✅ Goodsprings quest slice: hand-authored stage rewards and rule gates (`data/quests/slice_goodsprings.json`) merged by `normalize.py`; `Game` applies `on_enter` effects and fires consequence rules, undoes a failed change, and postpones rewards for unbuilt features into `Game::deferred()`; ten golden scenarios in `tests/scenarios/`

---

## 🔥 High Priority (Critical)

First playable: **Goodsprings only** (Doc Mitchell, Sunny, the Powder Gangers, Ghost Town Gunfight).

- **Grow the game state** – `Game` gains a real field each time a feature lands (inventory, reputation, character, companions), replacing its placeholder refusals.
- **Finish the Goodsprings slice** – Run Goodsprings Run, By a Campfire on the Trail, and the quests with no stages; fame and infamy amounts; replay `Game::deferred()` as inventory, character and reputation land.
- **Dialogue with skill checks** – a topic runner with conditions and effects; the Goodsprings dialogue graph has to be authored, since the wiki records no links between topics.
- **Turn-based AP combat** – VATS-style action points, DT/DR damage, criticals and weapon condition, using the curated formulas.
- **Reputation and karma** – fame/infamy per faction and town, titles and consequences.
- **Leveling, perks and traits** – XP table, skill points per level, a perk every two levels, trait choice at character creation.
- **Vendors and barter** – buying and selling with the barter formula. Vendor stock is not in the wiki and depends on the game-data export below, so the Goodsprings vendors (Chet, Trudy's stock, etc.) have to be authored by hand until then.
- **TUI v0** – a persistent layout (scene and log, status, active quest panel, action footer) so the player never has to remember commands.
- **Language-neutral golden tests and property tests** – JSON scenarios (state + action = expected state), `proptest` invariants, and a benchmark stub for the combat hot path.

---

## 🟡 Medium Priority (Important)

- **Save and load** – versioned saves of the complete game state, with a migration path.
- **Crafting** – recipes are imported.
- **Companions** – recruitment, loyalty rules, companion perks and their quests.
- **Mod support for the community** – content packs as data (items, quests, dialogue) validated against the same schemas, so players can create and share mods.
- **Import from the game's own data (xEdit export)** – blocked until a copy of the game is available. It would supply real quest conditions, dialogue links, loot and vendor lists, and exact stats, and replace most hand-authoring.
- **Expand beyond Goodsprings** – Primm, Novac and the main quest through Ring-a-Ding-Ding.

---

## 🟢 Low Priority (Nice‑to‑Have)

- **Faction disguises and karma-per-action tables** – small wiki imports that complete the reputation rules.
- **Hardcore mode** – hunger, thirst and sleep, with addiction mechanics.
- **Challenges and ending slides** – the data is imported; this is the player-facing part.
- **Radio stations, terminal and holotape texts** – flavor content for a text interface.
- **Mouse support and themes** – including a Pip-Boy green-on-black theme.
- **DLC areas** – Dead Money, Honest Hearts, Old World Blues, Lonesome Road.

---

## 🌌 Long‑Term Vision

- **LLM mode** – generated NPC text or world events constrained by the game state, so a model cannot contradict quest flags. Built after the deterministic game works.
- **The full Mojave** – New Vegas, the faction arcs and all endings.
- **A community mod ecosystem** – shared packs built on the stable data schemas.

---

## 🎯 Next Actions (Immediate)

1. **Build the dialogue runner** – `crates/engine/src/dialogue/`, with the Goodsprings graph authored alongside it.
2. **Build turn-based combat** – `crates/engine/src/combat/`, with property tests and a benchmark stub.
3. **TUI v0** – `crates/tui/`: the persistent layout with the quest panel and action footer.
