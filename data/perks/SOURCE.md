# data/perks

Hand-authored; the wiki records what a perk or trait does only as prose
(`effects_text`, `benefits`, `penalties`). `modifiers.json` gives the few whose
whole effect the game models as typed `modifier` records (see
`schemas/common.schema.json`), and `scripts/normalize.py` merges each entry into
the perk or trait of the same id in `data/engine/perks.json` or `traits.json`.

- An entry names exactly one `perk` or `trait` id; an unknown id or a repeated
  entry fails the merge, so a typo cannot silently drop an effect.
- Amounts are per rank (Swift Learner adds 10% for each rank taken).
- A perk or trait is listed only when every part of its text is modeled. The
  others (Toughness, Small Frame, Kamikaze, ...) change combat numbers the game
  does not carry yet; they stay unlisted, so taking one changes nothing and the
  picker says so.
