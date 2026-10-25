#!/usr/bin/env python3
"""Fetch Fallout: New Vegas NPC data from the Fallout wiki (MediaWiki API).

Usage:
    fetch_npcs.py list  <cache_dir>
    fetch_npcs.py fetch <cache_dir>
    fetch_npcs.py build <cache_dir> <out_dir>

Reuses the API and wikitext helpers from fetch_items.py / fetch_quests.py.
Writes <out_dir>/npcs.json, one record per character infobox.
"""
import importlib.util
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, HERE / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


fi = load("fetch_items")
fq = load("fetch_quests")

ROOT = "Fallout: New Vegas characters"
NPC_SKIP = (
    "cut ", "images", "mentioned-only", "test cell", "non-gameplay",
    "player characters", "playing card", "sounds", "gallery",
)
SPECIAL_NAMES = ["strength", "perception", "endurance", "charisma",
                 "intelligence", "agility", "luck"]


def cmd_list(cache):
    cache.mkdir(parents=True, exist_ok=True)
    seen, pages, stack = set(), {}, [fi.PREFIX + ROOT]
    while stack:
        cat = stack.pop()
        if cat in seen:
            continue
        seen.add(cat)
        for title in fi.members(cat, "page"):
            pages.setdefault(title, cat)
        for sub in fi.members(cat, "subcat"):
            if not any(s in sub.lower() for s in NPC_SKIP):
                stack.append(sub)
    print(f"{len(pages)} pages, {len(seen)} categories")
    (cache / "npc_listing.json").write_text(json.dumps(pages, indent=1))


def cmd_fetch(cache):
    pages = json.loads((cache / "npc_listing.json").read_text())
    path = cache / "npc_wikitext.json"
    store = json.loads(path.read_text()) if path.exists() else {}
    titles = sorted(set(pages) - set(store))
    print(f"fetching {len(titles)} pages")
    for i in range(0, len(titles), 40):
        d = fi.api(action="query", prop="revisions", rvprop="content",
                   rvslots="main", titles="|".join(titles[i:i + 40]))
        for p in d["query"]["pages"]:
            rev = p.get("revisions")
            store[p["title"]] = rev[0]["slots"]["main"]["content"] if rev else ""
        path.write_text(json.dumps(store))
        print(f"  {min(i + 40, len(titles))}/{len(titles)}")


def special_of(raw):
    m = re.search(r"\{\{\s*SPECIAL\s*\|([^}|]*)", raw or "", re.I)
    if not m:
        return None
    nums = [int(n) for n in re.findall(r"\d+", m.group(1))][:7]
    return dict(zip(SPECIAL_NAMES, nums)) if len(nums) == 7 else None


def tag_skills_of(raw):
    out = {}
    for body in (b for _, b in fq.templates(raw or "", "tag skills")):
        for k, v in fq.raw_params(body).items():
            if k.endswith(" max"):
                continue
            n = fi.num(fq.text(v))
            if isinstance(n, int):
                out[k.replace(" ", "_")] = n
    return out


def derived_of(raw):
    out = {}
    for body in (b for _, b in fq.templates(raw or "", "derived stats")):
        for k, v in fq.raw_params(body).items():
            n = fi.num(fq.text(v))
            if isinstance(n, (int, float)):
                out[k.replace(" ", "_")] = n
    return out


def links_of(raw):
    return [t for t in fq.link_targets(raw or "") if not t.startswith(("File:", "Category:"))]


def cmd_build(cache, out):
    pages = json.loads((cache / "npc_listing.json").read_text())
    store = json.loads((cache / "npc_wikitext.json").read_text())
    npcs, skipped = [], []
    for title in sorted(pages):
        wt = store.get(title, "")
        found = next(((n, b) for n, b in
                      ((n, b) for _, b in fq.templates(wt, "Infobox character")
                       for n in ["Infobox character"])), None)
        if not found:
            skipped.append(title)
            continue
        raw = fq.raw_params(found[1])
        games = fq.text(raw.get("games", ""))
        rec = {"page": title, "category": pages[title], "games": games}
        name = fq.text(raw.get("name", "")) or fq.text(raw.get("displayname", "")) or title
        rec["name"] = name
        for key, dst in (("race", "race"), ("gender", "gender"), ("sex", "gender"),
                         ("karma", "karma"), ("class", "class"), ("edid", "edid"),
                         ("formid", "formid"), ("aggression", "aggression"),
                         ("confidence", "confidence")):
            val = fq.text(raw.get(key, ""))
            if val and dst not in rec:
                rec[dst] = val
        for key, dst in (("affiliation", "affiliation"), ("role", "role"),
                         ("location", "location"), ("quests", "quests")):
            if key in raw:
                targets = links_of(raw[key])
                if key == "role" or not targets:
                    rec[dst] = [r.strip() for r in fq.text(raw[key]).split(";") if r.strip()]
                else:
                    rec[dst] = targets
        if "factions" in raw:
            rec["geck_factions"] = [f.strip() for f in fq.text(raw["factions"]).split(";") if f.strip()]
        m = re.search(r"\d+", fq.text(raw.get("level", "")))
        if m:
            rec["level"] = int(m.group(0))
        special = special_of(raw.get("special"))
        if special:
            rec["special"] = special
        skills = tag_skills_of(raw.get("tag skills"))
        if skills:
            rec["tag_skills"] = skills
        derived = derived_of(raw.get("derived"))
        if derived:
            rec["derived"] = derived
        dl = links_of(raw.get("dialogue"))
        if dl:
            rec["dialogue_pages"] = dl
        npcs.append(rec)
    out.mkdir(parents=True, exist_ok=True)
    (out / "npcs.json").write_text(json.dumps(npcs, indent=1, ensure_ascii=False) + "\n")
    (cache / "npc_skipped.txt").write_text("\n".join(skipped))
    print(f"{len(npcs)} npcs; {len(skipped)} pages without character infobox; "
          f"special {sum('special' in n for n in npcs)}, "
          f"dialogue links {sum('dialogue_pages' in n for n in npcs)}")


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    cmd, cache = argv[1], Path(argv[2])
    if cmd == "list":
        cmd_list(cache)
    elif cmd == "fetch":
        cmd_fetch(cache)
    elif cmd == "build" and len(argv) == 4:
        cmd_build(cache, Path(argv[3]))
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
