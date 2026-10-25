# data/combat

Hand-authored; the wiki has no encounter or loadout data. `encounters.json` is
copied unchanged to `data/engine/encounters.json` by `scripts/normalize.py`.

- Each encounter names a place, the conditions that offer it, the foes (an NPC
  record plus the weapon and armor ids they carry) and the effects of winning.
- Foe hit points are placeholders chosen so the Goodsprings fight can be won with
  a starting character. They are to be replaced by the game's own values once the
  xEdit export exists (see the project notes).
- The requirement (`ghost_town_gunfight` at stage 10 or later) is a stand-in: the
  stages 20 to 65 are not authored yet, so the fight opens as soon as Sunny has
  agreed to help.
