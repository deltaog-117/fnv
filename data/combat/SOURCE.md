# data/combat

Hand-authored; the wiki has no encounter or loadout data. `encounters.json` is
copied unchanged to `data/engine/encounters.json` by `scripts/normalize.py`.

- Each encounter names a place, the conditions that offer it, the foes (an NPC
  record plus the weapon and armor ids they carry) and the effects of winning.
- Foe hit points are placeholders chosen so the Goodsprings fight can be won with
  a starting character. They are to be replaced by the game's own values once the
  xEdit export exists (see the project notes).
- The requirement is `ghost_town_gunfight` at stage 65 ("Join up with Sunny") or
  later. The optional stages 20 to 50 (Trudy, Chet, Doc, Easy Pete) are not
  authored, so they are skipped: Ringo sets stage 60 and Sunny sets stage 65.
