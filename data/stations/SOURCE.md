# data/stations

Hand-authored; the wiki has no list of where the crafting stations stand.
`stations.json` is copied unchanged to `data/engine/stations.json` by
`scripts/normalize.py`.

- Each station names its kind (`Campfire`, `Workbench` or `Reloading bench`, the
  words `recipes.json` uses) and the place where it stands. A recipe can be made
  only at a station of its kind. A place with no station record offers no crafting.
- All three Goodsprings stations are placeholders. The campfire is the one the
  quest By a Campfire on the Trail asks for; the real positions are to be fetched
  once the game-data export exists.
