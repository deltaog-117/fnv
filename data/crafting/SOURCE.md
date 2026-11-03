# data/crafting

`effects.json` is hand-authored, not imported. Each entry names a recipe and the
story it moves: `scripts/normalize.py` merges the entry into that recipe's record
in `data/engine/recipes.json` as `on_craft`.

- A reaction runs when the recipe is made, in the same step as the crafting. It
  has `when` (every condition must hold; empty means always) and `effects`
  (applied in order). If any effect fails, nothing is crafted.
- An unknown recipe or a repeated entry fails the merge, so a typo cannot
  silently drop a reaction.
- Healing powder moves By a Campfire on the Trail to stage 30 ("use the campfire
  to make healing powder") while the quest is between stage 10 and stage 30.
