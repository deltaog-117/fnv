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
