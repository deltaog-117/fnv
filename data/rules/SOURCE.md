# Rules data source

Imported from the Fallout Wiki (fallout.fandom.com) via `scripts/fetch_rules.py`
(text is CC BY-SA 3.0; keep this attribution if redistributed).

| File | Contents |
|---|---|
| `stats.json` | S.P.E.C.I.A.L., the 13 skills and the derived statistics: infobox fields, the wiki's formula text and math blocks, prose from the New Vegas section |
| `levels.json` | Level 1-50: XP totals, XP to next level, add-ons needed, karma titles |
| `challenges.json` | Challenge infoboxes (task, requirement, repeatability, reward) |
| `skill_books.json` | Skill books and skill magazines |
| `recipes.json` | Rows of the crafting page's tables (workbench, reloading bench, campfire, hot plate) |
| `endings.json` | Ending slides: narration plus the in-game condition |
| `formulas.json` | **Hand-curated** engine formulas and constants, written from the wiki's stat pages and combat page. Not generated; edit it directly. |

Notes:
- `formulas.json` is checked by `scripts/validate.py` (safe expressions only,
  plus self-tests such as max HP 200 at Endurance 5, and XP to next level
  matching every row of `levels.json`).
- Recipe tables vary in layout. Ammunition conversion tables parse poorly and a
  few stray cells remain among the ingredients; breakdown tables are marked
  `mode: breakdown`.
- Only the formulas I could read and cross-check are encoded. The combat page
  calls its damage pipeline a general guide, not exact.

Regenerate the raw files: `list`, `fetch`, `build <cache_dir> data/rules`.
