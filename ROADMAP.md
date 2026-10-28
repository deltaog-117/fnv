# Roadmap

This document outlines the future direction of fnv-terminal-ai-game, a
turn-based terminal game inspired by Fallout: New Vegas, written in Rust.
Items are organized by priority, not by timeline.

---

## ✅ Completed (Milestones Achieved)

- ✅ Game data imported from the Fallout Wiki into raw JSON: 1,182 items (weapons, armor, ammo, consumables, weapon mods, skill books, crafting components), 256 quests, 880 NPCs, 517 dialogue transcripts, 684 locations with map positions, 201 perks, 16 traits, 42 factions and towns, 31 creatures, stats, levels, challenges, recipes and ending slides
- ✅ 116 hand-curated quest consequence rules (fails, locks, unlocks, exclusive paths)
- ✅ 13 JSON Schemas, including the shared `condition` / `effect` vocabulary
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
- ✅ Dialogue runner (`dialogue/`): `DialogueBook` (validated topic graph) and `Conversation` (start topics, follow-ups, conditions, locked checks, atomic effects, endings) over any `State`; `start` and `ends` added to the dialogue schema; hand-authored Doc Mitchell and Sunny Smiles graphs in `data/dialogue/slice_goodsprings.json`; 24 tests including a property test over random graphs, seven golden scenarios in `tests/scenarios/goodsprings_dialogue.json` and a menu benchmark stub (about 300 ns per menu)
- ✅ Goodsprings quest slice: hand-authored stage rewards and rule gates (`data/quests/slice_goodsprings.json`) merged by `normalize.py`; `Game` applies `on_enter` effects and fires consequence rules, undoes a failed change, and postpones rewards for unbuilt features into `Game::deferred()`; ten golden scenarios in `tests/scenarios/`
- ✅ Combat core (`combat/`): `Combat` runs a turn-based fight (action points, rounds, a win check) over a small `Combatant` trait, with the curated damage pipeline (weapon damage, resistance capped at 85, threshold with a 20% floor), bounded combat numbers, a swappable `HitModel` with a flat stand-in, and deterministic `Dice`; 29 tests including property tests and twelve golden scenarios in `tests/scenarios/combat.json`, and a fight benchmark stub (about 490 ns)
- ✅ TUI v0 (`crates/tui/`): an Elm-style `App` with a persistent layout (status line, scene, log, active-quest panel, key footer), an exploration screen and a dialogue screen over the engine's `Conversation`; property and headless render tests over the real Goodsprings data
- ✅ Character and inventory (`character/`, `inventory/`): `Character` (S.P.E.C.I.A.L. and XP, with level, hit points, action points, carry weight and starting skills derived from the curated formulas) and `Inventory` (item stacks, whole-number caps, weapon/outfit/head slots) over a 922-item `ItemBook`; `Game` owns both, so XP, caps and item rewards now apply instead of being deferred; the status line shows level and caps; the formulas, constants and XP table are pinned to `formulas.json` and `levels.json` by tests
- ✅ Combat wired into the game (`game/battle.rs`, `combat/ai.rs`, `crates/tui/src/combat/`): `Arsenal` checks every weapon, armor piece and encounter once at load; `Fighter` builds a `Combatant` from the character, the equipped weapon and armor, or from an NPC record; `Game` keeps the player's wounds between fights, offers encounters by quest state, starts a `Battle` and settles it (a win applies the encounter's effects once; a loss leaves the player down); the enemy side plays a plain tactic; the TUI has a combat screen and a game-over screen. The Ghost Town Gunfight is hand-authored in `data/combat/encounters.json`; 20 new tests including property tests over random fights
- ✅ Goodsprings gunfight path (dialogue): Ringo offers the quest (stage 5) and starts the countdown ("Let's do this", stage 60), Sunny joins the line (stage 65), and the fight is offered only from stage 65; the By a Campfire on the Trail quest starts from Sunny (stage 10) and ends with her errand to Trudy (stage 40); nine golden scenarios. The optional stages 20 to 50 are skipped for now
- ✅ Trudy's dialogue (`data/dialogue/slice_goodsprings.json`): the Cobb argument and the Ringo follow-ups, the news chains, the robot talk and the Speech 25 / Sneak 25 enlistment (flag `trudy_joined_militia`, open from stage 10 to 60); her radio and shop topics wait for the radio quest and vendors; eight golden scenarios, which can now set attributes
- ✅ Ammunition and reload: weapons hold a clip and spend rounds with every shot; a Reload action refills it from the rounds in the pack, foes reload when empty, the rounds fired leave the pack when the fight is settled, and Doc's starting kit includes 30 rounds of 9mm

- ✅ Leveling, perks and traits (engine): skill points at each level-up (`10 + 0.5 * intelligence`, half points carried over) and spending them, three tagged skills (+15), a perk pick every second level, up to two traits, and `Character::create`; `PerkBook` checks who may take which perk; a typed `Modifier` vocabulary (`shared/modifier.rs`) lets `data/perks/modifiers.json` make a perk or trait change a number, with five perks and two traits modeled so far; `Game::spend_skill_point` and `Game::choose_perk`; progression, property, real-data and contract tests
- ✅ Chet's dialogue (`data/dialogue/slice_goodsprings.json`): the Khans, New Vegas and ammo chatter, the gunfight supply talk (1000 caps or Barter 25; flag `chet_supplied_militia`) and the Powder Ganger shakedown (Barter 25 or Speech 25; flag `chet_shaken_down`, waits for Joe Cobb's quest start); his shop topics wait for vendors; eight golden scenarios
- ✅ Level-up screen (`crates/tui/src/character/`): a "Character" row in the scene (it says when skill points or perk picks are waiting) opens a screen to spend skill points on any of the 13 skills and to take an open perk, with the engine's own reason shown on a refusal and a note when the game does not model a perk yet; the real game now builds its `PerkBook`; long lists scroll with the cursor
- ✅ Character creation screen (`crates/tui/src/character/creation.rs`): before the first scene the player spends the 5 extra S.P.E.C.I.A.L. points, tags three skills and picks up to two traits, each in its own panel (Left/Right or Tab to move, Enter to open, Esc to close), with a Derived box and a Done button that stays locked until the choices are valid; a property test shows no key sequence breaks a limit or builds a character the engine would refuse

---

## 🔥 High Priority (Critical)

First playable: **Goodsprings only** (Doc Mitchell, Sunny, the Powder Gangers, Ghost Town Gunfight).

- **Grow the game state** – `Game` gains a real field each time a feature lands (reputation and companions are still placeholder refusals; character and inventory are done), replacing its placeholder refusals.
- **Finish the Goodsprings slice** – Run Goodsprings Run, By a Campfire on the Trail, and the quests with no stages; fame and infamy amounts; replay `Game::deferred()` once reputation lands.
- **Goodsprings dialogue content** – author the graphs for Easy Pete and Joe Cobb (Chet, Trudy, Ringo, Sunny and Doc's talk topics are done), then the Doc Mitchell intro and the paid topics (healing, supplies) once inventory exists. The wiki records no links or skill-check thresholds, so each one is read from the transcript.
- **Finish combat** – the real hit-chance and spread model (needs the wiki or the xEdit export; every attack hits 65% for now), body-part targeting, weapon condition that wears down, initiative, allies (Sunny and the townsfolk fight beside the player in the game), fleeing, healing between fights, a reload cost per weapon instead of the flat 2 action points, and replacing the placeholder foe hit points and loadouts with the game's own values. The nine weapons with no action-point cost cannot be wielded in a fight yet.
- **Reputation and karma** – fame/infamy per faction and town, titles and consequences.
- **Perks and traits not yet modeled** – the engine, the level-up screen and the character creation screen are done; still to do is modeling the perks and traits whose effects the game does not carry yet (combat numbers, for example). The creation screen shows the Derived numbers without trait effects until the character is made.
- **Vendors and barter** – buying and selling with the barter formula. Vendor stock is not in the wiki and depends on the game-data export below, so the Goodsprings vendors (Chet, Trudy's stock, etc.) have to be authored by hand until then.
- **TUI screens beyond v0** – travel between places, the quest journal, character sheet and inventory (the player wields the pistol and wears leather armor automatically until the inventory screen exists), each added with the engine feature it shows.
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

1. **Finish the Goodsprings quest slice** – the gunfight path is playable (stages 5, 10, 60, 65, 70); what remains is the campfire's stage 30 (needs crafting: gather a broc flower and a xander root, make healing powder), the optional stages 20 to 50 of the gunfight (Trudy, Chet, Doc, Easy Pete), Run Goodsprings Run (Joe Cobb's side), and the quests with no stages.
2. **Author the rest of the Goodsprings dialogue** – Easy Pete next (his stage 50 and the dynamite), then Joe Cobb (Chet's shakedown topics wait for his Run Goodsprings Run start); see "Goodsprings dialogue content" above; the TUI lists each NPC as soon as their graph is authored. Doc Mitchell's paid topics can now charge caps and give items.
