#!/usr/bin/env python3
"""Fetch Fallout: New Vegas item stats from the Fallout wiki (MediaWiki API).

Usage:
    fetch_items.py list   <cache_dir>   # enumerate pages per item group
    fetch_items.py fetch  <cache_dir>   # download wikitext for listed pages
    fetch_items.py build  <cache_dir> <out_dir>   # parse infoboxes -> JSON

The wiki text is cached in <cache_dir> so `build` can be re-run offline.
"""
import json
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

API = "https://fallout.fandom.com/api.php"
UA = "Mozilla/5.0 (fnv-terminal-ai-game data import; personal project)"
PREFIX = "Category:"

# group -> root categories (subcategories are walked, minus SKIP)
GROUPS = {
    "weapons": [
        "Fallout: New Vegas weapons",
    ],
    "armor": [
        "Fallout: New Vegas armor and clothing",
    ],
    "ammo": [
        "Fallout: New Vegas ammunition",
    ],
    "consumables": [
        "Fallout: New Vegas consumables",
    ],
    "weapon_mods": [
        "Fallout: New Vegas weapon mods",
    ],
    "components": [
        "Fallout: New Vegas crafting components",
    ],
}

# Subcategory name fragments that are not playable items.
SKIP = (
    "cut ", "images", "sounds", "test cell", "non-player", "mentioned-only",
    "gallery", "icons", "models", "unused",
)

REQUEST_DELAY = 0.25


def api(**params):
    params["format"] = "json"
    params["formatversion"] = "2"
    url = API + "?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=40) as r:
                data = json.load(r)
            time.sleep(REQUEST_DELAY)
            return data
        except Exception as exc:  # network hiccup: back off and retry
            if attempt == 3:
                raise
            print(f"  retry ({exc})", file=sys.stderr)
            time.sleep(2 * (attempt + 1))


def members(category, cmtype):
    out, cont = [], {}
    while True:
        d = api(action="query", list="categorymembers", cmtitle=category,
                cmlimit="500", cmtype=cmtype, **cont)
        out += [m["title"] for m in d["query"]["categorymembers"]]
        if "continue" not in d:
            return out
        cont = {"cmcontinue": d["continue"]["cmcontinue"]}


def walk(root):
    """Return (pages, categories_visited) under root, honouring SKIP."""
    seen, pages, stack = set(), {}, [PREFIX + root]
    while stack:
        cat = stack.pop()
        if cat in seen:
            continue
        seen.add(cat)
        for title in members(cat, "page"):
            pages.setdefault(title, cat)
        for sub in members(cat, "subcat"):
            low = sub.lower()
            if any(s in low for s in SKIP):
                continue
            stack.append(sub)
    return pages, seen


def cmd_list(cache):
    cache.mkdir(parents=True, exist_ok=True)
    listing = {}
    for group, roots in GROUPS.items():
        pages = {}
        for root in roots:
            got, cats = walk(root)
            pages.update(got)
            print(f"{group}: {root}: {len(got)} pages, {len(cats)} categories")
        listing[group] = pages
    (cache / "listing.json").write_text(json.dumps(listing, indent=1))


def cmd_fetch(cache):
    listing = json.loads((cache / "listing.json").read_text())
    wt_path = cache / "wikitext.json"
    store = json.loads(wt_path.read_text()) if wt_path.exists() else {}
    titles = sorted({t for g in listing.values() for t in g} - set(store))
    print(f"fetching {len(titles)} pages")
    for i in range(0, len(titles), 40):
        batch = titles[i:i + 40]
        d = api(action="query", prop="revisions", rvprop="content",
                rvslots="main", titles="|".join(batch))
        for p in d["query"]["pages"]:
            rev = p.get("revisions")
            store[p["title"]] = rev[0]["slots"]["main"]["content"] if rev else ""
        wt_path.write_text(json.dumps(store))
        print(f"  {min(i + 40, len(titles))}/{len(titles)}")


# ---------------------------------------------------------------- parsing

def find_infobox(text):
    m = re.search(r"\{\{\s*Infobox[^|{}\n]*", text, re.I)
    if not m:
        return None, None
    name = re.sub(r"\s+", " ", m.group(0)[2:].strip())
    depth, i, start = 0, m.start(), m.start()
    while i < len(text):
        if text.startswith("{{", i):
            depth += 1
            i += 2
        elif text.startswith("}}", i):
            depth -= 1
            i += 2
            if depth == 0:
                return name, text[start + 2:i - 2]
        else:
            i += 1
    return name, text[start + 2:]


def split_params(body):
    parts, buf, depth = [], [], 0
    i = 0
    while i < len(body):
        two = body[i:i + 2]
        if two in ("{{", "[["):
            depth += 1
            buf.append(two)
            i += 2
        elif two in ("}}", "]]"):
            depth -= 1
            buf.append(two)
            i += 2
        elif body[i] == "|" and depth == 0:
            parts.append("".join(buf))
            buf = []
            i += 1
        else:
            buf.append(body[i])
            i += 1
    parts.append("".join(buf))
    return parts[1:]  # drop the template name


def clean(v):
    v = re.sub(r"<!--.*?-->", "", v, flags=re.S)
    v = re.sub(r"<ref.*?(</ref>|/>)", "", v, flags=re.S)
    v = re.sub(r"\{\{\s*ID\s*\|\s*([0-9a-fA-F]+)\s*\}\}", r"\1", v)
    v = re.sub(r"\{\{\s*Icon[^{}]*\}\}", "", v, flags=re.I)
    v = re.sub(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]", r"\1", v)
    v = re.sub(r"<br\s*/?>", "; ", v, flags=re.I)
    v = re.sub(r"<[^>]+>", "", v)
    v = re.sub(r"'{2,}", "", v)
    return re.sub(r"\s+", " ", v).strip(" ;")


def parse_infobox(body):
    params = {}
    for part in split_params(body):
        if "=" not in part:
            continue
        k, _, v = part.partition("=")
        k = re.sub(r"\s+", " ", k.strip().lower())
        v = clean(v)
        if k and v:
            params[k] = v
    return params


def num(v):
    try:
        f = float(v.replace(",", ""))
        return int(f) if f.is_integer() else f
    except (ValueError, AttributeError):
        return v


NUMERIC_KEYS = {
    "damage", "strength req", "skill req", "hp", "weight", "value", "dt", "dr",
    "ap", "clip rounds", "reload time", "crit dmg", "crit % mult", "projectiles",
    "attack shots/sec", "min spread", "spread", "ammo use", "range",
    "weight hardcore", "dps", "min range", "max range", "min damage",
}
# apparel / ammo infoboxes repeat these keys with a numeric suffix per item
VARIANT_BASES = {
    "item name", "dt", "dr", "hp", "weight", "value", "class", "repair",
    "variants", "edid", "formid", "weight hardcore", "slot", "effects",
}
SKIP_PREFIXES = ("image", "icon", "perk", "gallery", "quests", "dialogue")
NOT_ITEM_INFOBOXES = ("Infobox gameplay", "Infobox world object")


def normalise(params):
    out = {}
    for k, v in params.items():
        if k.startswith(SKIP_PREFIXES) or re.fullmatch(r"games?\d+", k):
            continue
        out[k] = num(v) if k in NUMERIC_KEYS else v
    return out


def split_variants(params):
    """Group suffix-numbered keys (dt2, value3...) into per-item dicts."""
    items = {}
    for k, v in params.items():
        m = re.fullmatch(r"(.*?)\s?(\d*)", k)
        base, idx = m.group(1), int(m.group(2) or 1)
        if base in VARIANT_BASES:
            items.setdefault(idx, {})[base] = num(v) if base in NUMERIC_KEYS else v
        else:
            items.setdefault(0, {})[k] = v
    shared = items.pop(0, {})
    return shared, [items[i] for i in sorted(items)]


def cmd_build(cache, out):
    listing = json.loads((cache / "listing.json").read_text())
    store = json.loads((cache / "wikitext.json").read_text())
    out.mkdir(parents=True, exist_ok=True)
    report = []
    # weapon mods live under the weapons category tree; keep them separate
    listing["weapons"] = {t: c for t, c in listing["weapons"].items()
                          if t not in listing["weapon_mods"]}
    for group, pages in listing.items():
        records, missing = [], []
        for title in sorted(pages):
            name, body = find_infobox(store.get(title, ""))
            if not body or name in NOT_ITEM_INFOBOXES:
                missing.append(title)
                continue
            params = normalise(parse_infobox(body))
            rec = {"page": title, "infobox": name, "category": pages[title]}
            if group in ("armor", "ammo"):
                shared, variants = split_variants(params)
                rec.update(shared)
                rec["items"] = variants
            else:
                rec.update(params)
            records.append(rec)
        (out / f"{group}.json").write_text(
            json.dumps(records, indent=1, ensure_ascii=False) + "\n")
        n_items = sum(len(r.get("items", [1])) for r in records)
        report.append(f"{group}: {len(records)} pages, {n_items} items, "
                      f"{len(missing)} without infobox")
        if missing:
            (cache / f"missing_{group}.txt").write_text("\n".join(missing))
    print("\n".join(report))


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
