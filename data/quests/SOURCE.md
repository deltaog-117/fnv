# Quest data source

Quest data was imported from the Fallout Wiki (fallout.fandom.com) through its
MediaWiki API, using `scripts/fetch_quests.py` (text is CC BY-SA 3.0; keep this
attribution if redistributed).

One file per quest type (`main`, `side`, `marked`, `unmarked`). Each quest has:

| Key | Contents |
|---|---|
| `infobox` | Giver, location, reward, `previous` / `leads to` / `related` (page titles), `edid`, `formid` |
| `stages` | Stage numbers and descriptions from the quest stage table; `status: finish` ends the quest |
| `chart` | Quick-walkthrough flowchart nodes per section; each inner list is one row, and several nodes in a row are alternative branches |
| `notes`, `outcomes` | Raw bullets from the wiki, to be read for lockouts and consequences |

The wiki describes choices but does not expose engine conditions, so branch
consequences from `notes`/`outcomes` still need a reading pass.

Regenerate with `list`, `fetch` and `build <cache_dir> data/quests` (see the
script docstring).

## consequences.json

Hand-curated from the wiki `notes` of every quest that has them (152 quests).
Each entry is one rule: `quest` (the quest whose page states it), `kind`,
`target`, `target_type`, `when` and a short `note`. When `target` equals
`quest`, the quest itself fails or becomes unobtainable.

| `kind` | Meaning |
|---|---|
| `fails` | The target quest fails |
| `locks` | The target becomes unobtainable, impossible or a route is closed |
| `unlocks` | Opens a quest, option, location or perk |
| `exclusive` | Both cannot be completed in one playthrough |
| `triggers` | Starts the target quest or event |
| `requires` | A prerequisite or branch condition |
| `effect` | Reputation, companion, ending or world-state change |

Only gameplay-relevant rules were kept. Trivia, glitches and PC console
workarounds from the notes were left out. Entries with `target_type: "quest"`
are validated against the quest files.
