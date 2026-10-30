# data/vendors

Hand-authored; the wiki has no vendor stock or caps. `vendors.json` is copied
unchanged to `data/engine/vendors.json` by `scripts/normalize.py`.

- Each vendor names the NPC who trades, the place, the caps they hold and the
  items they start with (each item once). Prices are not stored: they come from
  the item's base value and the Barter formulas in `data/rules/formulas.json`.
- The three Goodsprings vendors (Chet, Trudy, Doc Mitchell) are placeholders
  written from memory of what each sells, with round numbers for stock and caps.
  They are to be replaced by the game's own values once the xEdit export exists.
