# Item data source

Item stats were imported from the Fallout Wiki (fallout.fandom.com) infoboxes
through its MediaWiki API, using `scripts/fetch_items.py`. Wiki text is
available under CC BY-SA 3.0, so keep this attribution if the data is
redistributed.

| File | Contents |
|---|---|
| `weapons.json` | One record per weapon page (vanilla, DLC, unique) |
| `armor.json` | One record per page; `items` lists each armor/clothing piece |
| `ammo.json` | One record per page; `items` lists each ammo variant |
| `consumables.json` | Chems, food, drink, medicine |
| `weapon_mods.json` | Weapon modification items |

`uses.json` is the exception: it is hand-authored, not imported, and the
regeneration commands below leave it alone. Each entry names a consumable
`item` and what using it does (`heal`: hit points restored at once), and
`scripts/normalize.py` merges it into that item's record in
`data/engine/consumables.json` as `on_use`.

- An unknown item or a repeated entry fails the merge, so a typo cannot
  silently drop an effect.
- An item is listed only when every part of its text is modeled. Anything that
  adds radiation, poisons the player or changes damage dealt is not (those wait
  for radiation, poison and the real combat numbers).
- A bonus or penalty that lasts is a `boost`: a stat, an amount and a length in
  whole minutes. The clock counts minutes, so the wiki's seconds are rounded up
  to one (healing powder's 18 seconds of -1 Perception is one minute). A new
  dose of the same item replaces the last; it does not stack.
- An effect that the wiki gives per second (+2 HP/s for 5s) is entered as its
  total (10) and restored at once, since no heal runs longer than the clock's
  one-minute step.
- Chems are listed without their addiction chance: addiction is not built yet,
  so using a chem has no downside beyond its own penalties.

Values are the wiki's own cleaned infobox fields (`edid` and `formid` are the
in-game identifiers). Fields vary by item type, so not every record has every
key.

Regenerate with:

    python3 -I scripts/fetch_items.py list  <cache_dir>
    python3 -I scripts/fetch_items.py fetch <cache_dir>
    python3 -I scripts/fetch_items.py build <cache_dir> data/items
