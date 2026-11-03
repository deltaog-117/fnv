# data/finds

Hand-authored; the wiki has no list of where plants and scrap lie. `finds.json`
is copied unchanged to `data/engine/finds.json` by `scripts/normalize.py`.

- A find is something the player can pick up at a place, once: it lists the
  items (each a weapon, armor, ammo, consumable or crafting item, with a count)
  and the place. Taking it is a game action; nothing respawns.
- The two Goodsprings finds (a broc flower and a xander root, the ingredients
  of healing powder for By a Campfire on the Trail) are placeholders: the wiki
  says the plants grow around Goodsprings but not where. The real positions are
  to be fetched once the game-data export exists.
