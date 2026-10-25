#!/usr/bin/env python3
"""Fetch Fallout: New Vegas world data from the Fallout wiki.

Usage:
    fetch_world.py list  <cache_dir>
    fetch_world.py fetch <cache_dir>
    fetch_world.py build <cache_dir> <out_dir>

Covers locations (plus the interactive map markers), perks, traits, factions
(plus the reputation page) and creatures. Reuses the API and wikitext helpers
from fetch_items.py and fetch_quests.py. Writes into <out_dir>:
locations.json, map_markers.json, perks.json, traits.json, factions.json,
reputation.json, creatures.json.
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
fd = load("fetch_dialogue")

COMMON_SKIP = ("cut ", "images", "sounds", "mentioned-only", "test cell", "gallery")
GROUPS = {
    "locations": {"root": "Fallout: New Vegas locations", "skip": COMMON_SKIP},
    "perks": {"root": "Fallout: New Vegas perks", "skip": COMMON_SKIP},
    "traits": {"root": "Fallout: New Vegas traits", "skip": COMMON_SKIP + ("wild wasteland",)},
    "factions": {"root": "Fallout: New Vegas factions", "skip": COMMON_SKIP + ("playing card",),
                 "only": "factions", "main_pages": True},
    "creatures": {"root": "Fallout: New Vegas creatures", "skip": COMMON_SKIP},
}
MAPS = {"base": "Map:FNV World Map", "dead_money": "Map:FNVDM World Map",
        "honest_hearts": "Map:FNVHH World Map", "old_world_blues": "Map:FNVOWB World Map",
        "lonesome_road": "Map:FNVLR World Map"}
EXTRA = {"traits": ["Wild Wasteland (Fallout: New Vegas)", "Wild Wasteland"],
         "factions": ["Brotherhood of Steel (Mojave chapter)"]}
OVERVIEW = "Fallout: New Vegas locations"
REPUTATION = "Fallout: New Vegas reputations"
SKILLS = ["Barter", "Energy Weapons", "Explosives", "Guns", "Lockpick", "Medicine",
          "Melee Weapons", "Repair", "Science", "Sneak", "Speech", "Survival", "Unarmed"]
SPECIALS = ["Strength", "Perception", "Endurance", "Charisma", "Intelligence", "Agility", "Luck"]


def walk(group):
    cfg = GROUPS[group]
    seen, pages, stack = set(), {}, [fi.PREFIX + cfg["root"]]
    while stack:
        cat = stack.pop()
        if cat in seen:
            continue
        seen.add(cat)
        for title in fi.members(cat, "page"):
            if not title.startswith("Category:"):
                pages.setdefault(title, cat)
        for sub in fi.members(cat, "subcat"):
            low = sub.lower()
            if any(s in low for s in cfg["skip"]):
                continue
            if "only" in cfg and cfg["only"] not in low:
                if cfg.get("main_pages"):
                    # other subcategories hold members; keep only their main page
                    pages.setdefault(sub[len(fi.PREFIX):], sub)
                continue
            stack.append(sub)
    return pages, seen


def cmd_list(cache):
    cache.mkdir(parents=True, exist_ok=True)
    listing = {}
    for group in GROUPS:
        pages, cats = walk(group)
        for extra in EXTRA.get(group, []):
            pages.setdefault(extra, "extra")
        listing[group] = pages
        print(f"{group}: {len(pages)} pages, {len(cats)} categories")
    (cache / "world_listing.json").write_text(json.dumps(listing, indent=1))


def cmd_fetch(cache):
    listing = json.loads((cache / "world_listing.json").read_text())
    path = cache / "world_wikitext.json"
    store = json.loads(path.read_text()) if path.exists() else {}
    wanted = {t for g in listing.values() for t in g}
    wanted |= {OVERVIEW, REPUTATION, *MAPS.values()}
    titles = sorted(wanted - set(store))
    print(f"fetching {len(titles)} pages")
    for i in range(0, len(titles), 20):
        d = fi.api(action="query", prop="revisions", rvprop="content",
                   rvslots="main", titles="|".join(titles[i:i + 20]))
        for p in d["query"]["pages"]:
            rev = p.get("revisions")
            store[p["title"]] = rev[0]["slots"]["main"]["content"] if rev else ""
        path.write_text(json.dumps(store))
        print(f"  {min(i + 20, len(titles))}/{len(titles)}")


# ---------------------------------------------------------------- helpers

IMAGE_KEYS = ("image", "icon", "map marker image", "local map", "label", "gallery", "audio")
LOCATION_LINK_KEYS = ("part of", "factions", "leaders", "doctors", "merchants", "sections",
                      "buildings", "quests", "creatures", "robots", "owners", "terminal")


def infobox(wt, prefix):
    for _, body in fq.templates(wt, prefix):
        return fq.raw_params(body)
    return None


def links(raw):
    return [t for t in fq.link_targets(raw or "")
            if not t.startswith(("File:", "Category:", "Timeline"))]


def split_text(raw):
    return [p.strip() for p in fq.text(raw or "").split(";") if p.strip()]


def numbered(raw, game_prefix="FNV"):
    """Pick the column of a numbered infobox (games1, requires1, ...) for FNV."""
    groups = {}
    for k, v in raw.items():
        m = re.fullmatch(r"(.*?)\s?(\d*)", k)
        groups.setdefault(int(m.group(2) or 0), {})[m.group(1)] = v
    base = groups.pop(0, {})
    pick = next((g for _, g in sorted(groups.items())
                 if "FNV" in fq.text(g.get("games", ""))), None)
    if pick is None:
        pick = next((g for _, g in sorted(groups.items())), {})
    return {**base, **pick}


def dlc_of(games):
    for tok in re.split(r"[,\s]+", fq.text(games)):
        if tok in fi.__dict__.get("DLC", {}):
            return tok
    return None


def in_game_description(wt):
    m = re.search(r"\{\{\s*Quotation\s*\|([^|}]*)", wt, re.I)
    return fd.clean(m.group(1)) if m else None


def parse_requirements(raw):
    text = fq.text(raw or "")
    req = {}
    rest = text
    for name in ["Level"] + SPECIALS + SKILLS + ["Karma"]:
        m = re.search(rf"\b{name}\b\s*(?:of\s*)?(\d+)", rest, re.I)
        if not m:
            continue
        n = int(m.group(1))
        key = name.lower().replace(" ", "_")
        if name == "Level":
            req["level"] = n
        elif name in SPECIALS:
            req.setdefault("special", {})[key] = n
        elif name in SKILLS:
            req.setdefault("skills", {})[key] = n
        rest = rest[:m.start()] + rest[m.end():]
    rest = re.sub(r"\b(AND|OR)\b|[;,]", " ", rest).strip()
    rest = re.sub(r"\s+", " ", rest)
    if rest:
        req["other"] = rest
    return req, text


# ------------------------------------------------------------- locations

def parse_overview(wt):
    """Map location page title -> heading path in the locations overview."""
    path, out = [], {}
    for line in wt.splitlines():
        m = re.match(r"^(={2,4})\s*([^=]+?)\s*\1\s*$", line)
        if m:
            level = len(m.group(1)) - 1
            path = path[:level - 1] + [fi.clean(m.group(2))]
            continue
        names = fq.link_targets(line)
        names += [n.strip() for n in re.findall(r"\{\{\s*flink\s*\|(?:[^|}]*\|)?([^|}]+)\}\}", line, re.I)]
        for n in names:
            if path and n not in out:
                out[n] = " > ".join(path)
    return out


def parse_markers(store):
    markers = []
    for dlc, title in MAPS.items():
        try:
            data = json.loads(store.get(title, ""))
        except ValueError:
            continue
        cats = {c["id"]: c["name"] for c in data.get("categories", [])}
        for m in data.get("markers", []):
            pop = m.get("popup", {})
            link = (pop.get("link") or {}).get("url") or pop.get("title")
            x, y = m["position"]
            markers.append({"map": dlc, "id": m.get("id"), "name": pop.get("title"),
                            "page": link, "category": cats.get(m.get("categoryId")),
                            "x": round(x, 1), "y": round(y, 1)})
    return markers


def build_locations(listing, store):
    overview = parse_overview(store.get(OVERVIEW, ""))
    markers = parse_markers(store)
    def key(t):
        return re.sub(r"[^a-z0-9]+", "", re.sub(r"\(.*?\)", "", str(t).lower()))
    by_key = {}
    for m in markers:
        by_key.setdefault(key(m["page"]), m)
        by_key.setdefault(key(m["name"]), m)
    by_page = {t: by_key[key(t)] for t in listing if key(t) in by_key}
    out, skipped = [], []
    for title in sorted(listing):
        raw = infobox(store.get(title, ""), "Infobox location")
        if raw is None:
            skipped.append(title)
            continue
        rec = {"page": title, "category": listing[title], "games": fq.text(raw.get("games", ""))}
        for key, val in raw.items():
            if key.startswith(IMAGE_KEYS) or key == "games":
                continue
            if key in LOCATION_LINK_KEYS:
                got = links(val) or split_text(val)
                if got:
                    rec[key.replace(" ", "_")] = got
            else:
                t = fq.text(val)
                if t:
                    rec[key.replace(" ", "_")] = t
        if title in overview:
            rec["overview_section"] = overview[title]
        if title in by_page:
            mk = by_page[title]
            rec["marker"] = {k: mk[k] for k in ("map", "x", "y", "category")}
        out.append(rec)
    return out, markers, skipped


# ----------------------------------------------------------- perks, traits

def build_perks(listing, store, kind):
    out, skipped = [], []
    for title in sorted(listing):
        wt = store.get(title, "")
        raw = infobox(wt, f"Infobox {kind}")
        if raw is None:
            skipped.append(title)
            continue
        col = numbered(raw)
        rec = {"page": title, "category": listing[title], "games": fq.text(col.get("games", ""))}
        desc = in_game_description(wt)
        if desc:
            rec["description"] = desc
        if kind == "perk":
            req, req_text = parse_requirements(col.get("requires"))
            rec["requirements"] = req
            if req_text:
                rec["requires_text"] = req_text
            for key, dst in (("ranks", "ranks"), ("effects", "effects_text")):
                if col.get(key):
                    rec[dst] = fi.num(fq.text(col[key])) if key == "ranks" else fq.text(col[key])
        else:
            for key in ("requires", "benefits", "penalties"):
                if col.get(key):
                    rec[key] = fq.text(col[key])
        if col.get("formid"):
            rec["formid"] = fq.text(col["formid"])
        out.append(rec)
    return out, skipped


# --------------------------------------------------------------- creatures

def build_creatures(listing, store):
    out, skipped = [], []
    for title in sorted(listing):
        wt = store.get(title, "")
        raw = infobox(wt, "Infobox creature") or {}
        heads = [(m.start(), fi.clean(m.group(2))) for m in
                 re.finditer(r"^(={2,4})\s*([^=\n]+?)\s*\1\s*$", wt, re.M)]
        variants = []
        for pos, body in fq.templates(wt, "Stats creature FNV"):
            row = fq.raw_params(body)
            if "name" not in row:
                continue
            rec = {"group": next((h for p, h in reversed(heads) if p < pos), "")}
            rec["name"] = re.sub(r"\s*;\s*", " ", fq.text(row["name"]))
            for key in ("level", "xp", "perception", "hp", "dt", "dr", "aggression",
                        "confidence", "assistance"):
                v = fi.num(re.sub(r"[^\d.\-]", "", fq.text(row.get(key, "")).split()[0])) \
                    if row.get(key) and fq.text(row.get(key, "")).split() else None
                if isinstance(v, (int, float)):
                    rec[key] = v
            if row.get("formid"):
                rec["formid"] = fq.text(row["formid"])
            attacks = [fq.text(v) for k, v in sorted(row.items()) if k.startswith("attack")]
            if attacks:
                rec["attacks"] = attacks
            if row.get("items"):
                rec["items"] = [re.sub(r"^[*\s]+", "", ln) for ln in
                                fq.text(row["items"]).split(";") if ln.strip()]
            variants.append(rec)
        if not variants:
            skipped.append(title)
            continue
        rec = {"page": title, "category": listing[title],
               "games": fq.text(raw.get("games", "")), "type": fq.text(raw.get("type", "")),
               "location": links(raw.get("location")) or split_text(raw.get("location")),
               "variants": variants}
        out.append(rec)
    return out, skipped


# ---------------------------------------------------------------- factions

def build_factions(listing, store):
    out, skipped = [], []
    for title in sorted(listing):
        wt = store.get(title, "")
        raw = infobox(wt, "Infobox faction") or infobox(wt, "Infobox association")
        if raw is None:
            skipped.append(title)
            continue
        rec = {"page": title, "category": listing[title]}
        for key, val in raw.items():
            if key.startswith(IMAGE_KEYS):
                continue
            key = key.replace(" ", "_")
            lk = links(val)
            t = fq.text(val)
            if key == "games" or key == "mentions":
                rec[key] = t
            elif lk and key in ("leaders", "leader", "capital", "headquarters", "allies",
                                "enemies", "locations", "location", "members", "territories"):
                rec[key] = lk
            elif t:
                rec[key] = t[:400]
        out.append(rec)
    return out, skipped


def table_after(text, heading_re):
    m = re.search(heading_re, text)
    return text[m.end():] if m else ""


def parse_threshold_table(table):
    """Return (ranges, titles matrix) from a fame/infamy table."""
    ranges = []
    for r in re.findall(r"^!\s*(\d+(?:\s*-\s*\d+|\+))\s*$", table, re.M):
        r = re.sub(r"\s+", "", r)
        if r not in ranges:
            ranges.append(r)
    rows = []
    for chunk in re.split(r"^\|-\s*$", table, flags=re.M)[1:]:
        cells = []
        for line in chunk.strip().splitlines():
            if line.startswith("|") and not line.startswith(("|-", "|}")):
                cell = re.sub(r'^\|\s*(?:style="[^"]*"\s*\|)?', "", line)
                cells.append(fd.clean(cell))
        if len(cells) >= 4 and rows is not None:
            rows.append(cells[-4:])
    return ranges[:4], rows[-4:] if len(rows) >= 4 else rows


def parse_bullets(text):
    items, parent = [], ""
    for line in text.splitlines():
        m = re.match(r"^(\*+)\s*(.+)$", line)
        if not m:
            continue
        depth, body = len(m.group(1)), fd.clean(m.group(2))
        if not body:
            continue
        full = f"{parent}: {body}" if depth > 1 and parent else body
        if depth == 1:
            parent = body.rstrip(":")
        rec = {"text": full}
        d = re.search(r"\(\s*([+-]\s?\d+)\s*(?:each|per[^)]*)?\)", full)
        if d:
            rec["delta"] = int(d.group(1).replace(" ", ""))
        items.append(rec)
    return items


def build_reputation(store):
    wt = store.get(REPUTATION, "")
    out = {"bump_breakdown": {}, "benefits": [], "entities": []}
    tabs = re.findall(r"\{\|.*?\n\|\}", wt, re.S)
    if tabs:
        t = tabs[0]
        names = re.findall(r"^!\s*(?:Bump Type)?\s*$|\|\s*(Very Minor|Minor|Average|Major|Very Major)", t, re.M)
        vals = re.findall(r"!Reputation Change\s*\n((?:\|\s*\d+\s*\n?)+)", t)
        nums = [int(n) for n in re.findall(r"\d+", vals[0])] if vals else []
        order = [n for n in names if n]
        out["bump_breakdown"] = dict(zip([o.lower().replace(" ", "_") for o in order], nums))
    benefit_tab = next((x for x in tabs if "Effect" in x and "Faction" in x), None)
    if benefit_tab:
        benefit_tab = re.sub(r'^!\s*width="[^"]*"\s*\|', "!", benefit_tab, flags=re.M)
        for row in fd.parse_table(benefit_tab):
            rec = {k: fd.clean(v) for k, v in row.items() if k in ("faction", "reputation", "effect")}
            if rec.get("effect"):
                out["benefits"].append(rec)
    start = wt.find("==Full reputation change list==")
    body = wt[start:] if start >= 0 else ""
    parts = re.split(r"^===([^=\n]+?)===\s*$", body, flags=re.M)
    for name, section in zip(parts[1::2], parts[2::2]):
        name = fi.clean(name)
        if name.lower() == "foreword":
            continue
        tab = re.search(r"\{\|.*?\n\|\}", section, re.S)
        rec = {"name": name}
        if tab:
            ranges, titles = parse_threshold_table(tab.group(0))
            rec["ranges"], rec["titles"] = ranges, titles
        sub = re.split(r"^(={4,6})\s*([^=\n]+?)\s*\1\s*$", section, flags=re.M)
        current, buckets = "", {}
        for i in range(1, len(sub), 3):
            current = fi.clean(sub[i + 1]).lower()
            text = sub[i + 2] if i + 2 < len(sub) else ""
            buckets[current] = buckets.get(current, "") + "\n" + text
        for key, dst in (("consequences", "consequences"), ("positive", "gains"), ("negative", "losses")):
            items = []
            for k, text in buckets.items():
                if k.startswith(key):
                    items += parse_bullets(text)
            if items:
                rec[dst] = items
        out["entities"].append(rec)
    return out


def dump(path, data):
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(f"{path.name}: {len(data) if isinstance(data, list) else 'ok'}")


def cmd_build(cache, out):
    listing = json.loads((cache / "world_listing.json").read_text())
    store = json.loads((cache / "world_wikitext.json").read_text())
    out.mkdir(parents=True, exist_ok=True)
    skipped = {}
    locs, markers, skipped["locations"] = build_locations(listing["locations"], store)
    dump(out / "locations.json", locs)
    dump(out / "map_markers.json", markers)
    perks, skipped["perks"] = build_perks(listing["perks"], store, "perk")
    dump(out / "perks.json", perks)
    traits, skipped["traits"] = build_perks(listing["traits"], store, "trait")
    dump(out / "traits.json", traits)
    factions, skipped["factions"] = build_factions(listing["factions"], store)
    dump(out / "factions.json", factions)
    creatures, skipped["creatures"] = build_creatures(listing["creatures"], store)
    dump(out / "creatures.json", creatures)
    rep = build_reputation(store)
    dump(out / "reputation.json", rep)
    (cache / "world_skipped.json").write_text(json.dumps(skipped, indent=1))
    print({k: len(v) for k, v in skipped.items()}, "pages skipped (no infobox/stats)")
    print("reputation entities:", len(rep["entities"]), "benefits:", len(rep["benefits"]))


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
