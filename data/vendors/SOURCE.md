# data/vendors

Hand-authored; the wiki has no vendor stock or caps. `vendors.json` is copied
unchanged to `data/engine/vendors.json` by `scripts/normalize.py`.

- Each vendor names the NPC who trades, the place, the caps they hold and the
  items they start with (each item once). Prices are not stored: they come from
  the item's base value and the Barter formulas in `data/rules/formulas.json`.
- The three Goodsprings vendors (Chet, Trudy, Doc Mitchell) are placeholders
  written from memory of what each sells, with round numbers for stock and caps.
  They are to be replaced by the game's own values once the xEdit export exists.

- A vendor may carry a `discount`: a percent off what the player pays, and
  `when` conditions (any one earns it). Chet and Trudy give 25% to a player the
  town finds Accepted or better, or Mixed, per the wiki's reputation notes
  (roadmap wording: "Accepted or better and Mixed or more"). The wiki says only
  "special discount" / "wholesale prices" in the dialogue; 25% is the figure the
  roadmap carries, so treat it as a stand-in until the game-data export.

- A vendor may carry `restock_days`: how many days pass between restocks (3 when absent).
  The wiki gives no interval, so 3 is a stand-in. On each multiple of the interval since
  the game began the vendor goes back to the stock and caps written here.

- A vendor may carry a `repair` service: the vendor's own Repair skill, up to
  which they mend a worn weapon for caps (the price is worked out by the engine
  from the weapon's value; see `repair/`). Chet is the only Goodsprings vendor
  who repairs, with Repair 50: a placeholder, since the wiki lists no repair
  services for Goodsprings. Chet also stocks two weapon repair kits, also a
  placeholder.

- A vendor may carry `flat_prices`: conditions (any one is enough) under which the
  vendor buys and sells at the item's base value, with no Barter markup either way.
  The wiki's Barter page says Chet "will use only the PC's barter skill value to
  determine prices even if they are Idolized with Goodsprings, but with a mixed
  reputation (regardless of how good or bad it is), he will offer 1:1 prices", so
  Chet carries one condition: Goodsprings standing is exactly Mixed. "1:1" read as
  base value both ways is the game's reading, since the page does not spell it out.
  A flat price wins over a `discount`.
