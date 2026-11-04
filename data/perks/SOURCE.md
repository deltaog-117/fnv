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

Modifiers can be conditional (see `schemas/common.schema.json`): `skill` limits one
to a weapon governed by that skill, `foe` to a kind of foe and `item` to one item.
Fifteen perks and seven traits are modeled; the reading of each is pinned to the
wiki's own text by `crates/engine/tests/character/real_data.rs`. Stand-ins:

- Finesse is a flat +5% critical chance; the wiki says "equivalent to +5 Luck".
- Better Criticals and Hunter scale the damage a critical hit adds, since the wiki
  does not say where the critical bonus enters the formula.
- Fast Shot and Trigger Discipline carry their V.A.T.S. accuracy through spread (the
  game has no V.A.T.S.), and their fire rate as action point cost.
- Demolition Expert, Toughness and Hunter's two foe kinds are per rank or per kind
  as the wiki writes them; a perk whose ranks differ in size (such as the 3/6/10%
  foe perks) is not modeled, and those ask for kill counts, so they are not offered.

