# Dialogue data source

Imported from the Fallout Wiki "dialogue files" transcripts (`<Name>.txt`
pages) via `scripts/fetch_dialogue.py` (CC BY-SA 3.0; keep this attribution if
redistributed). One JSON file per transcript, plus `index.json`.

Each topic has the wiki's topic id, the player's prompt, a `label` when the
wiki shows one, and one or more NPC `responses` with emotion and intensity.
Sections are the transcript headings (Topics, Conversation, Combat,
Detection, plus a few speaker-specific ones).

What the wiki does NOT provide: which topic follows which, the conditions that
make a line available (skill checks, reputation, quest stage), and the effects
of a choice. Only a handful of prompts carry a bracketed skill check, which is
kept as `check`. The conversation graph is therefore a flat list; linking
topics and attaching conditions/effects is authoring work.

Regenerate: `list`, `fetch`, `build <cache_dir> data/dialogue`.

## Hand-authored slice

`slice_goodsprings.json` is hand-written, not imported. For each listed
speaker it gives the `start` topics (the menu a conversation opens with) and,
per topic key, any of `conditions`, `effects`, `check`, `next` and `ends`.
`normalize.py` merges it into `data/engine/dialogue/` and raises on an unknown
dialogue or topic. The wiki gives no links, so every link and condition was
read from the transcript text and is not verified in game. Do not regenerate
or overwrite this file when re-importing the transcripts.
