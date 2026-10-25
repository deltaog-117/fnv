#!/usr/bin/env python3
"""Fetch Fallout: New Vegas quest data from the Fallout wiki (MediaWiki API).

Usage:
    fetch_quests.py list  <cache_dir>
    fetch_quests.py fetch <cache_dir>
    fetch_quests.py build <cache_dir> <out_dir>

Reuses the API and wikitext helpers from fetch_items.py. Per quest it keeps the
infobox (with link targets for the quest graph), the stage table, the flowchart
nodes grouped by row, and the Notes/Outcomes bullets for later reading.
"""
import importlib.util
import json
import re
import sys
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "fetch_items", Path(__file__).with_name("fetch_items.py"))
fi = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(fi)

ROOT = "Fallout: New Vegas quests"
QUEST_SKIP = fi.SKIP + ("cut ", "unused", "quest items")
LINK_KEYS = ("previous", "leads to", "related")
CHART_SKIP = {"class", "colspan", "rowspan", "style", "align", "width"}
SECTIONS = ("Notes", "Outcomes", "Objectives")


def walk_quests():
    seen, pages, stack = set(), {}, [fi.PREFIX + ROOT]
    while stack:
        cat = stack.pop()
        if cat in seen:
            continue
        seen.add(cat)
        for title in fi.members(cat, "page"):
            pages.setdefault(title, cat)
        for sub in fi.members(cat, "subcat"):
            if any(s in sub.lower() for s in QUEST_SKIP):
                continue
            stack.append(sub)
    return pages, seen


def cmd_list(cache):
    cache.mkdir(parents=True, exist_ok=True)
    pages, cats = walk_quests()
    print(f"{len(pages)} pages, {len(cats)} categories")
    (cache / "quest_listing.json").write_text(json.dumps(pages, indent=1))


def cmd_fetch(cache):
    pages = json.loads((cache / "quest_listing.json").read_text())
    path = cache / "quest_wikitext.json"
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


# ---------------------------------------------------------------- parsing

def text(v):
    """Clean wikitext to plain text, keeping the first arg of {{tooltip}}."""
    v = re.sub(r"\{\{\s*ID\s*\|\s*([0-9a-fA-F]+)\s*\}\}", r"\1", v)
    v = re.sub(r"\{\{\s*tooltip\s*\|([^|{}]*)[^{}]*\}\}", r"\1", v, flags=re.I)
    v = re.sub(r"\{\{[^{}]*\}\}", "", v)
    return fi.clean(v)


def link_targets(raw):
    return [t.strip() for t in re.findall(r"\[\[([^\]|#]+)", raw)]


def templates(wt, name):
    """Yield the body of every {{name ...}} template in wt."""
    for m in re.finditer(r"\{\{\s*" + re.escape(name) + r"\b", wt, re.I):
        depth, i = 0, m.start()
        while i < len(wt):
            if wt.startswith("{{", i):
                depth += 1
                i += 2
            elif wt.startswith("}}", i):
                depth -= 1
                i += 2
                if depth == 0:
                    yield m.start(), wt[m.start() + 2:i - 2]
                    break
            else:
                i += 1


def raw_params(body):
    out = {}
    for part in fi.split_params(body):
        if "=" in part:
            k, _, v = part.partition("=")
            out[re.sub(r"\s+", " ", k.strip().lower())] = v
    return out


def parse_infobox(wt):
    for _, body in templates(wt, "Infobox quest"):
        raw = raw_params(body)
        info = {}
        for k, v in raw.items():
            if k.startswith(("image", "icon")):
                continue
            if k in LINK_KEYS:
                info[k] = link_targets(v)
            elif k in ("given by", "location"):
                info[k] = link_targets(v) or [text(v)]
            else:
                val = text(v)
                if val:
                    info[k] = val
        return info
    return None


def parse_stages(wt):
    for _, body in templates(wt, "Quest stage table"):
        raw = raw_params(body)
        stages = []
        for k, v in raw.items():
            m = re.fullmatch(r"stage(\d+)", k)
            if not m:
                continue
            n = m.group(1)
            st = {"stage": fi.num(text(v)), "desc": text(raw.get(f"desc{n}", ""))}
            status = text(raw.get(f"status{n}", ""))
            if status:
                st["status"] = status
            stages.append(st)
        return sorted(stages, key=lambda s: str(s["stage"]).zfill(6))
    return []


def headings(wt):
    return [(m.start(), m.group(2).strip())
            for m in re.finditer(r"^(={2,3})\s*([^=\n]+?)\s*\1\s*$", wt, re.M)]


def parse_charts(wt):
    """Flowchart nodes per section, one list per chart row."""
    heads = headings(wt)
    charts = {}
    for pos, body in templates(wt, "chart"):
        raw = raw_params(body)
        nodes = []
        for k, v in raw.items():
            if k in CHART_SKIP:
                continue
            val = text(v)
            if val and not re.fullmatch(r"\d+|(width|height)\s*:.*", val):
                nodes.append({"kind": re.sub(r"\d+$", "", k), "text": val})
        if not nodes:
            continue
        section = next((h for p, h in reversed(heads) if p < pos), "")
        charts.setdefault(section, []).append(nodes)
    return charts


def parse_sections(wt):
    heads = headings(wt) + [(len(wt), "")]
    out = {}
    for (start, name), (end, _) in zip(heads, heads[1:]):
        if name not in SECTIONS:
            continue
        lines = [text(ln.lstrip("*# ").strip())
                 for ln in wt[start:end].splitlines()[1:]
                 if ln.lstrip().startswith(("*", "#")) or name == "Outcomes"]
        lines = [ln for ln in lines if ln]
        if lines:
            out[name.lower()] = lines
    return out


def cmd_build(cache, out):
    pages = json.loads((cache / "quest_listing.json").read_text())
    store = json.loads((cache / "quest_wikitext.json").read_text())
    quests, skipped = {}, []
    for title in sorted(pages):
        wt = store.get(title, "")
        info = parse_infobox(wt)
        if not info:
            skipped.append(title)
            continue
        rec = {"page": title, "category": pages[title], "infobox": info,
               "stages": parse_stages(wt), "chart": parse_charts(wt)}
        rec.update(parse_sections(wt))
        quests[title] = rec

    # graph check: every previous / leads to / related link should be a quest
    known = set(quests) | {t.replace(" (quest)", "") for t in quests}
    dangling = sorted({(q, k, t) for q, r in quests.items()
                       for k in LINK_KEYS for t in r["infobox"].get(k, [])
                       if t not in known})
    out.mkdir(parents=True, exist_ok=True)
    by_type = {}
    for q in quests.values():
        by_type.setdefault(q["infobox"].get("type", "other").lower(), []).append(q)
    for kind, recs in sorted(by_type.items()):
        name = re.sub(r"[^a-z0-9]+", "_", kind.lower()).strip("_") or "other"
        (out / f"{name}.json").write_text(
            json.dumps(recs, indent=1, ensure_ascii=False) + "\n")
        print(f"{name}: {len(recs)} quests")
    (cache / "quest_skipped.txt").write_text("\n".join(skipped))
    (cache / "quest_dangling.txt").write_text(
        "\n".join(" | ".join(d) for d in dangling))
    with_stages = sum(1 for q in quests.values() if q["stages"])
    with_chart = sum(1 for q in quests.values() if q["chart"])
    print(f"total {len(quests)} quests; stages {with_stages}, chart {with_chart}; "
          f"{len(skipped)} pages without infobox; {len(dangling)} dangling links")


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
