# data/beds

Hand-authored; the wiki has no list of usable beds. `beds.json` is copied
unchanged to `data/engine/beds.json` by `scripts/normalize.py`.

- Each bed names the place where the player may sleep. A place with no bed
  record offers no sleep.
- The single Goodsprings bed (Doc Mitchell's house) is a placeholder. The real
  list, with owned and rented beds, is to be fetched together with the perk
  data that reads it (Well Rested, campfire sleeping) once the game-data export
  exists.
