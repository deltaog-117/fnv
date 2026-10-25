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

Values are the wiki's own cleaned infobox fields (`edid` and `formid` are the
in-game identifiers). Fields vary by item type, so not every record has every
key.

Regenerate with:

    python3 -I scripts/fetch_items.py list  <cache_dir>
    python3 -I scripts/fetch_items.py fetch <cache_dir>
    python3 -I scripts/fetch_items.py build <cache_dir> data/items
