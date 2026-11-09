# data/companions

Hand-authored; the wiki has no loadouts for companions. `companions.json` is copied
unchanged to `data/engine/companions.json` by `scripts/normalize.py`.

- Each record names the companion, the NPC they are and what they fight with (a
  weapon and optional armor id, like a foe in `data/combat/encounters.json`; a
  `when` is not allowed). Every loadout is a placeholder until the xEdit export
  exists, and a companion's hit points come from the NPC record when it has some.
- `perk` names a perk whose modifiers (in `data/perks/modifiers.json`) the player
  has while the companion is in the party. Only Arcade Gannon's Better Healing is
  modeled; the other companion perks are about vision, crafting or highlighting,
  which the game does not have.
- `refuses_when` lists conditions under which the companion will not join and
  leaves. They come from the wiki's "Fallout: New Vegas reputations" benefits:
  Arcade refuses a negative Followers standing and a positive Legion one, Boone a
  negative NCR standing and a positive Legion one, Cass a negative NCR standing,
  Veronica a negative Brotherhood standing. "Negative" is Shunned or worse and
  "positive" is Accepted or better.
- The party holds two (a stand-in: the real game allows one person and one robot or
  creature). The eight companions are the base-game ones; Marcus and the add-on
  companions are left out.
