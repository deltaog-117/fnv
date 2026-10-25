#!/usr/bin/env python3
"""Fetch Fallout: New Vegas rules data from the Fallout wiki.

Usage:
    fetch_rules.py list  <cache_dir>
    fetch_rules.py fetch <cache_dir>
    fetch_rules.py build <cache_dir> <out_dir>

Covers stats (S.P.E.C.I.A.L., skills, derived statistics), challenges, skill
books and magazines, crafting recipes and the ending slides. Writes into
<out_dir>: stats.json, challenges.json, skill_books.json, recipes.json,
endings.json. Formulas are kept as the wiki's text and math blocks; they are
turned into engine formulas by hand (data/rules/formulas.json).
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

STAT_CATEGORIES = {
    "primary": "Fallout: New Vegas primary statistics",
    "skill": "Fallout: New Vegas skills",
    "derived": "Fallout: New Vegas derived statistics",
}
CHALLENGES = "Fallout: New Vegas challenges"
BOOKS = {"skillbook": "Fallout: New Vegas skill books",
         "skillmagazine": "Fallout: New Vegas skill magazines"}
CRAFTING = "Fallout: New Vegas crafting"
ENDINGS = "Fallout: New Vegas endings"
LEVEL = "Level"
SKIP = ("cut ", "images", "gallery")


def pages_of(category, subcats=True):
    out = {}
    stack, seen = [fi.PREFIX + category], set()
    while stack:
        cat = stack.pop()
        if cat in seen:
            continue
        seen.add(cat)
        for t in fi.members(cat, "page"):
            if not t.startswith("Category:"):
                out.setdefault(t, cat)
        for sub in fi.members(cat, "subcat") if subcats else []:
            if not any(s in sub.lower() for s in SKIP):
                stack.append(sub)
    return out


def cmd_list(cache):
    cache.mkdir(parents=True, exist_ok=True)
    listing = {}
    for kind, cat in STAT_CATEGORIES.items():
        listing[f"stat_{kind}"] = pages_of(cat, subcats=False)
    listing["challenges"] = pages_of(CHALLENGES)
    for kind, cat in BOOKS.items():
        listing[kind] = pages_of(cat)
    for g, p in listing.items():
        print(f"{g}: {len(p)} pages")
    (cache / "rules_listing.json").write_text(json.dumps(listing, indent=1))


def cmd_fetch(cache):
    listing = json.loads((cache / "rules_listing.json").read_text())
    path = cache / "rules_wikitext.json"
    store = json.loads(path.read_text()) if path.exists() else {}
    wanted = {t for g in listing.values() for t in g} | {CRAFTING, ENDINGS, LEVEL}
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


# ---------------------------------------------------------------- parsing

def infobox(wt, prefix):
    for _, body in fq.templates(wt, prefix):
        return fq.raw_params(body)
    return None


def fnv_column(raw):
    """The FNV column of a numbered infobox (games1, modifies1, ...)."""
    groups = {}
    for k, v in raw.items():
        m = re.fullmatch(r"(.*?)\s?(\d*)", k)
        groups.setdefault(int(m.group(2) or 0), {})[m.group(1)] = v
    base = groups.pop(0, {})
    pick = next((g for _, g in sorted(groups.items())
                 if "FNV" in fq.text(g.get("games", ""))), {})
    return {**base, **pick}


def section_of(wt, needle):
    """Text of the deepest heading whose title contains `needle`, else ''."""
    heads = [(m.start(), m.end(), len(m.group(1)), m.group(2).strip())
             for m in re.finditer(r"^(={2,5})\s*([^=\n]+?)\s*\1\s*$", wt, re.M)]
    best = None
    for i, (s, e, lvl, title) in enumerate(heads):
        if needle in title and (best is None or lvl >= best[1]):
            end = next((h[0] for h in heads[i + 1:] if h[2] <= lvl), len(wt))
            best = (wt[e:end], lvl)
    return best[0] if best else ""


def math_blocks(text):
    out = []
    for m in re.finditer(r"<math>(.*?)</math>", text, re.S):
        out.append(re.sub(r"\s+", " ", m.group(1)).strip())
    return out


def build_stats(listing, store):
    out = []
    for group in ("stat_primary", "stat_skill", "stat_derived"):
        for title in sorted(listing[group]):
            wt = store.get(title, "")
            raw = infobox(wt, "Infobox stat")
            if raw is None:
                continue
            col = fnv_column(raw)
            rec = {"page": title, "kind": group.split("_", 1)[1]}
            for key in ("type", "modifies", "governed by", "initial value",
                        "related perks", "related traits", "formid"):
                v = col.get(key, raw.get(key))
                if v:
                    rec[key.replace(" ", "_")] = fq.text(v)
            rec["governed_by_links"] = [t for t in fq.link_targets(col.get("governed by", ""))]
            rec["related_perk_links"] = [t for t in fq.link_targets(col.get("related perks", ""))]
            sect = section_of(wt, "New Vegas")
            scope = sect or wt
            rec["math"] = math_blocks(scope)[:12]
            prose = fd.clean(re.sub(r"<math>.*?</math>", " [math] ", scope, flags=re.S))
            prose = re.sub(r"<ref[^>]*>.*?</ref>|<ref[^>]*/>", "", prose, flags=re.S)
            rec["fnv_text"] = prose[:2500]
            out.append(rec)
    return out


def build_challenges(listing, store):
    out = []
    for title in sorted(listing["challenges"]):
        wt = store.get(title, "")
        raw = infobox(wt, "Infobox challenge")
        if raw is None:
            continue
        rec = {"page": title, "category": listing["challenges"][title],
               "games": fq.text(raw.get("games", ""))}
        for key in ("task", "requires", "repeats", "reward", "formid"):
            if raw.get(key):
                rec[key] = fq.text(raw[key])
        links = fq.link_targets(raw.get("task", ""))
        if links:
            rec["task_links"] = links
        m = re.search(r"is an?\s+(?:\[\[[^\]|]*\|)?([\w\s-]*?challenge)", wt)
        if m:
            rec["challenge_type"] = m.group(1).strip()
        out.append(rec)
    return out


def build_books(listing, store):
    out = []
    for kind in BOOKS:
        for title in sorted(listing[kind]):
            wt = store.get(title, "")
            raw = infobox(wt, "Infobox item") or infobox(wt, "Infobox consumable")
            if raw is None:
                continue
            rec = {"page": title, "kind": kind, "games": fq.text(raw.get("games", ""))}
            for key in ("effects", "weight", "value", "edid", "formid"):
                if raw.get(key):
                    rec[key] = fq.text(raw[key])
            out.append(rec)
    return out


QTY = re.compile(r"\((\d+)\)\s*$")
SKILL_REQ = re.compile(r"\[\[([^\]|]*)(?:\|([^\]]*))?\]\]\s*:\s*(\d+)")


def table_cells(chunk):
    cells = []
    for line in chunk.strip().splitlines():
        if line.startswith("|") and not line.startswith(("|-", "|}", "|+")):
            body = line[1:]
            m = fd.ATTRS.match(body)
            cells.append((m.group(2) if m else body).strip())
        elif cells and line and not line.startswith(("!", "{|")):
            cells[-1] += " " + line.strip()
    return cells


def build_recipes(store):
    wt = store.get(CRAFTING, "")
    start = wt.find("==Craftable items list==")
    body = wt[start:] if start >= 0 else ""
    heads = [(m.start(), len(m.group(1)), fi.clean(m.group(2)))
             for m in re.finditer(r"^(={2,5})\s*([^=\n]+?)\s*\1\s*$", body, re.M)]
    out = []
    for tm in re.finditer(r"\{\|.*?\n\|\}", body, re.S):
        path = []
        for pos, lvl, title in heads:
            if pos > tm.start():
                break
            path = path[:lvl - 2] + [title]
        if any("shopping" in h.lower() for h in path):
            continue  # an aggregate of the other tables, not recipes
        header = " ".join(re.findall(r"^!.*$", tm.group(0), re.M)).lower()
        # breakdown tables list what an item is taken apart into; all others list
        # the ingredients needed to make `name`
        mode = "breakdown" if "items produced" in header else "craft"
        for chunk in re.split(r"^\|-\s*$", tm.group(0), flags=re.M)[1:]:
            cells = table_cells(chunk)
            if len(cells) < 2:
                continue
            name_raw = cells[0]
            name = fd.clean(re.sub(r"\{\{[^{}]*\}\}", "", name_raw))
            rec = {"table": " > ".join(path), "mode": mode,
                   "name": QTY.sub("", name).strip("¹²³ ").strip()}
            q = QTY.search(name)
            if q:
                rec["name_qty"] = int(q.group(1))
            ingredients, station, skill = [], None, None
            for cell in cells[1:]:
                sm = SKILL_REQ.search(cell)
                if sm and not skill:
                    skill = {"skill": fd.clean(sm.group(2) or sm.group(1)), "min": int(sm.group(3))}
                    continue
                text = fd.clean(re.sub(r"\{\{[^{}]*\}\}", "", cell))
                if re.search(r"bench|workbench|campfire|hot plate|coin operator", text, re.I) \
                        and not QTY.search(text):
                    station = station or text
                    continue
                for part in re.split(r"\s*[*;]\s*", text):
                    part = part.strip()
                    if part in ("", "-", "\u2014", "\u2013"):
                        continue
                    qm = QTY.search(part)
                    ingredients.append({"item": QTY.sub("", part).strip(),
                                        "qty": int(qm.group(1)) if qm else 1})
            if skill:
                rec["skill"] = skill
            if station:
                rec["station"] = station
            rec["items"] = ingredients
            out.append(rec)
    return out


def build_endings(store):
    wt = store.get(ENDINGS, "")
    out = []
    parts = re.split(r"^(==)\s*(Slide[^=\n]+?)\s*==\s*$", wt, flags=re.M)
    for i in range(1, len(parts) - 2, 3):
        slide, section = fi.clean(parts[i + 1]), parts[i + 2]
        subs = re.split(r"^===\s*([^=\n]+?)\s*===\s*$", section, flags=re.M)
        blocks = [("", subs[0])] + [(fi.clean(subs[j]), subs[j + 1]) for j in range(1, len(subs) - 1, 2)]
        for variant, text in blocks:
            for tm in re.finditer(r"\{\|.*?\n\|\}", text, re.S):
                tab = re.sub(r'^!\s*style="[^"]*"\s*\|', "!", tm.group(0), flags=re.M)
                tab = tab.replace("<nowiki>#</nowiki>", "#")
                for row in fd.parse_table(tab):
                    narr = fd.clean(row.get("voice-over narration", ""))
                    if not narr:
                        continue
                    cond_raw = row.get("in-game condition", "")
                    rec = {"slide": slide, "variant": variant, "narration": narr,
                           "condition": re.sub(r"\s*\|\}\s*$", "", fd.clean(cond_raw)).strip()}
                    links = [t for t in fq.link_targets(cond_raw)]
                    if links:
                        rec["condition_links"] = links
                    out.append(rec)
    return out


def build_levels(store):
    """XP and karma-title table from the Level page's New Vegas section."""
    sect = section_of(store.get(LEVEL, ""), "New Vegas")
    m = re.search(r"\{\|.*?\n\|\}", sect, re.S)
    if not m:
        return []
    tab = re.sub(r"^!\s*width\s*=\s*\d+\s*\|", "!", m.group(0), flags=re.M)
    tab = re.sub(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]", r"\1", tab)
    out = []
    for row in fd.parse_table(tab):
        level = fi.num(fd.clean(row.get("level", "")))
        if not isinstance(level, int):
            continue
        xp = lambda k: fi.num(re.sub(r"[^\d]", "", row.get(k, "")) or "")
        rec = {"level": level, "xp_total": xp("experience"), "xp_to_next": xp("exp to next level"),
               "dlc_required": 0 if level <= 30 else (level - 26) // 5,
               "karma_titles": {k: fd.clean(row.get(f"karma ({k})", "")) for k in ("good", "neutral", "evil")}}
        out.append(rec)
    return out


def dump(path, data):
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(f"{path.name}: {len(data)}")


def cmd_build(cache, out):
    listing = json.loads((cache / "rules_listing.json").read_text())
    store = json.loads((cache / "rules_wikitext.json").read_text())
    out.mkdir(parents=True, exist_ok=True)
    dump(out / "stats.json", build_stats(listing, store))
    dump(out / "challenges.json", build_challenges(listing, store))
    dump(out / "skill_books.json", build_books(listing, store))
    dump(out / "recipes.json", build_recipes(store))
    dump(out / "endings.json", build_endings(store))
    dump(out / "levels.json", build_levels(store))


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
