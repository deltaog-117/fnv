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

Recipes themselves come from the wiki's tables through `scripts/normalize.py`:

- A `breakdown` recipe takes the item it names apart: that item is spent (as many
  as `output_qty`, one when absent) and the listed ingredients are what comes back.
- An ingredient written "Name (N, not expended)" becomes `kept: true` with `qty` N: a
  tool that must be carried and is not used up.

