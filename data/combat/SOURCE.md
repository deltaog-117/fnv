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

## Foes that depend on the story, and the Run Goodsprings Run fights

- A foe may carry `when` conditions; it is in the fight only while they all hold.
  The game has no allies yet, so a helper who joined the town is written as an
  enemy who is absent. Every encounter keeps at least one foe with no `when`.
- Ghost Town Gunfight: Joe Cobb and two Powder Gangers always attack. The wiki
  says six attack (five if Cobb is dead), so three more gangers are present
  unless Trudy joined the militia, Easy Pete joined, or Chet supplied it. A
  player who prepared everything faces the old three; one who prepared nothing
  faces six. Which prep removes which ganger is a stand-in: the wiki does not say.
- Kill Ringo (stage 10 to 15) and Attack the Goodsprings militia (stage 60 to 70)
  are the two fights of Run Goodsprings Run. The wiki gives no loadouts or hit
  points for the defenders, so those are stand-ins. Trudy stays out of the
  militia fight when Chet was shaken down; winning kills Sunny, Easy Pete and
  Trudy, so Trudy's saloon stops trading. Chet and Doc survive, as in the wiki.
- Victor never fights: the transcript has him agree to help and then "doze off"
  after an override command, so he is a dialogue flag (`victor_joined_militia`) and
  not an ally. The two Goodsprings settlers that Trudy's Speech or Sneak 25 check
  enlists fight under her flag; the wiki gives neither a count nor a loadout, so two
  settlers with a 9mm pistol and 25 hit points are stand-ins.
