# World data source

Imported from the Fallout Wiki (fallout.fandom.com) via `scripts/fetch_world.py`
(text is CC BY-SA 3.0; keep this attribution if redistributed).

| File | Contents |
|---|---|
| `locations.json` | Location infoboxes: type, parent, sections/buildings, leaders, merchants, quests, creatures, plus the map marker when one matched |
| `map_markers.json` | Every marker of the wiki's interactive maps (base game and the four DLC maps): name, page, category, x/y on a 1024x1024 map, origin top-left |
| `perks.json`, `traits.json` | Perk and trait infoboxes (the FNV column), with requirements parsed where possible |
| `factions.json` | Faction and association infoboxes (infobox text kept as written) |
| `reputation.json` | The wiki's reputation page: point bumps, benefits table, and per faction/town the fame/infamy thresholds, title matrix, consequences, and point gains/losses |
| `creatures.json` | Creature pages with one row per stat variant (level, XP, HP, DT, DR, attacks, drops) |

The wiki has no road or adjacency data. The travel graph (`neighbors`) is
derived in `scripts/normalize.py` from marker coordinates (each marker's three
nearest plus a spanning tree), so it is an approximation, not the game's routes.

Regenerate: `list`, `fetch`, `build <cache_dir> data/world`.
