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

---

## 🔥 High Priority (Critical)

First playable: **Goodsprings only** (Doc Mitchell, Sunny, the Powder Gangers, Ghost Town Gunfight).

- **Typed data model** – newtype ids and `Condition` / `Effect` enums that mirror `common.schema.json`, so invalid states cannot be represented and the schema and code cannot drift apart.
- **Data loader** – load and validate the engine data for the Goodsprings slice at startup, failing with clear errors.
- **Game state and condition/effect evaluator** – the one place that decides whether a dialogue option, quest rule or encounter applies.
- **Quest engine** – stages, branching, and the lock/fail/unlock rules from the consequence data.
- **Dialogue with skill checks** – a topic runner with conditions and effects; the Goodsprings dialogue graph has to be authored, since the wiki records no links between topics.
- **Turn-based AP combat** – VATS-style action points, DT/DR damage, criticals and weapon condition, using the curated formulas.
- **Reputation and karma** – fame/infamy per faction and town, titles and consequences.
- **TUI v0** – a persistent layout (scene and log, status, active quest panel, action footer) so the player never has to remember commands.
- **Language-neutral golden tests and property tests** – JSON scenarios (state + action = expected state), `proptest` invariants, and a benchmark stub for the combat hot path.

---

## 🟡 Medium Priority (Important)

- **Leveling, perks and traits** – XP table, skill points per level, a perk every two levels, trait choice at character creation.
- **Save and load** – versioned saves of the complete game state, with a migration path.
- **Vendors, barter and crafting** – recipes are imported; vendor stock is not in the wiki and depends on the game-data export below.
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

1. **Define the data models** – id newtypes and `Condition` / `Effect` enums in `crates/engine/src/shared/`, with property tests that round-trip them against `schemas/common.schema.json`.
2. **Write the data loader** – read `data/engine/` for the Goodsprings slice in `crates/engine/src/shared/` and report schema errors with the offending file and id.
3. **Build the evaluator and the view traits** – `shared` evaluates conditions through small read-only view traits with default answers; `game` implements them.
4. **Build the quest engine** – `crates/engine/src/quests/`: state, stages and the consequence rules.
5. **Build the dialogue runner** – `crates/engine/src/dialogue/`, with the Goodsprings graph authored alongside it.
6. **Build turn-based combat** – `crates/engine/src/combat/`, with property tests and a benchmark stub.
7. **TUI v0** – `crates/tui/`: the persistent layout with the quest panel and action footer.
