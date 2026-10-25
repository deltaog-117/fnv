# Engine data schemas

JSON Schema (draft 2020-12), language-neutral. Raw wiki imports live in
`data/items`, `data/quests`, `data/npcs`, `data/dialogue`, `data/world` and `data/rules` and are never edited (except `data/rules/formulas.json`, `data/quests/consequences.json` and `data/quests/slice_*.json`, which are hand-curated); `scripts/normalize.py`
turns them into the engine-ready files in `data/engine/`, and
`scripts/validate.py` checks those files against these schemas.

| Schema | Describes | Status |
|---|---|---|
| `common.schema.json` | Shared ids, enums, and the `condition` / `effect` vocabulary | Done |
| `item.schema.json` | Weapons, armor, ammo, consumables, weapon mods (`kind` selects the shape) | Done, populated |
| `quest.schema.json` | Quests with stages, links, walkthrough rows and consequence rules | Done, populated |
| `npc.schema.json` | NPCs (stats, roles, quests, dialogue links) | Done, populated |
| `location.schema.json` | Locations with map position and a derived travel graph | Done, populated |
| `perk.schema.json`, `trait.schema.json` | Perks (with parsed requirements) and traits | Done, populated |
| `faction.schema.json`, `reputation.schema.json` | Factions and towns with fame/infamy models; global reputation rules | Done, populated |
| `creature.schema.json` | Creature species with stat variants | Done, populated |
| `rules.schema.json` | Stats, levels, challenges, recipes, endings and the formula file (definitions selected by `$ref`) | Done, populated |
| `dialogue.schema.json` | One file per speaker: flat topics, with slots for conditions, effects and `next` | Done, populated (graph not authored) |

## Conventions

- Ids are lowercase snake_case slugs of the display name, made unique with the
  form id or a counter. Cross-references use ids, never page titles.
- `*_text` fields keep the wiki's original wording next to any parsed field.
- `condition` and `effect` are the machine-readable vocabulary for quest
  rules, dialogue and NPC availability. Quest rules carry the wiki's
  `when` / `note`; the Goodsprings slice (`data/quests/slice_goodsprings.json`)
  authors `conditions` / `effects` for its rules and `on_enter` effects for
  stages, and `normalize.py` merges them. Other quests are still unauthored.

## Regenerate and check

    python3 -I scripts/normalize.py data
    python3 -I scripts/validate.py .      # needs: pip install jsonschema
