# NPC data source

Imported from the Fallout Wiki (fallout.fandom.com) character infoboxes via
`scripts/fetch_npcs.py` (text is CC BY-SA 3.0; keep this attribution if
redistributed). `npcs.json` holds one record per page in the Fallout: New Vegas
character categories (cut, mentioned-only, test-cell and non-gameplay
characters excluded).

Fields kept: display name, race, gender, karma, class, editor id, form id,
affiliation, role, location, linked quests, GECK factions, level, SPECIAL,
tagged skills, HP/DT, and the link to the speaker's dialogue transcript.
Characters that only appear in other games (their `games` field has no FNV
token) are dropped when the data is normalized.

Regenerate: `list`, `fetch`, `build <cache_dir> data/npcs`.
