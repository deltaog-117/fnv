#!/usr/bin/env python3
"""Fetch Fallout: New Vegas dialogue transcripts from the Fallout wiki.

Usage:
    fetch_dialogue.py list  <cache_dir>
    fetch_dialogue.py fetch <cache_dir>
    fetch_dialogue.py build <cache_dir> <out_dir>

The wiki keeps one `<Name>.txt` page per speaker, holding tables of
topic / player prompt / emotion / NPC response. It does not record which
topic follows which, nor any conditions, so the import is a flat topic list.
Writes <out_dir>/<page>.json per transcript plus <out_dir>/index.json.
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
CATEGORY = "Category:Fallout: New Vegas dialogue files"
BATCH = 15  # transcripts are large; keep responses small


def cmd_list(cache):
    cache.mkdir(parents=True, exist_ok=True)
    titles = fi.members(CATEGORY, "page")
    print(f"{len(titles)} transcripts")
    (cache / "dialogue_listing.json").write_text(json.dumps(sorted(titles), indent=1))


def cmd_fetch(cache):
    titles = json.loads((cache / "dialogue_listing.json").read_text())
    path = cache / "dialogue_wikitext.json"
    store = json.loads(path.read_text()) if path.exists() else {}
    todo = [t for t in titles if t not in store]
    print(f"fetching {len(todo)} transcripts")
    for i in range(0, len(todo), BATCH):
        d = fi.api(action="query", prop="revisions", rvprop="content",
                   rvslots="main", titles="|".join(todo[i:i + BATCH]))
        for p in d["query"]["pages"]:
            rev = p.get("revisions")
            store[p["title"]] = rev[0]["slots"]["main"]["content"] if rev else ""
        path.write_text(json.dumps(store))
        print(f"  {min(i + BATCH, len(todo))}/{len(todo)}")


# ---------------------------------------------------------------- parsing

ATTRS = re.compile(r'\s*((?:(?:row|col)span="\d+"\s*)+)\|(.*)$', re.S)
ROWSPAN = re.compile(r'rowspan="(\d+)"')
CHECK = re.compile(r"^\[\s*([A-Za-z][A-Za-z ]*?)\s+(\d+)\s*\]\s*")
LINKABLE = re.compile(r"\{\{\s*linkable\s*\|([^}|]*)[^}]*\}\}", re.I)
EMOTION = re.compile(r"^(\w[\w ]*?)\s+(\d+)$")


def clean(v):
    v = re.sub(r"<!--.*?-->", "", v, flags=re.S)
    v = LINKABLE.sub(r"\1", v)
    v = re.sub(r"\{\{[^{}]*\}\}", "", v)
    v = re.sub(r"<br\s*/?>", "\n", v, flags=re.I)
    v = re.sub(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]", r"\1", v)
    v = re.sub(r"'{2,}", "", v)
    return re.sub(r"[ \t]+", " ", v).strip()


def split_cells(row):
    """Cells of one table row as (rowspan, text) pairs."""
    cells = []
    for line in row.split("\n"):
        if not line.startswith("|") or line.startswith(("|-", "|}", "|+")):
            if cells and line and not line.startswith(("!", "{|")):
                cells[-1] = (cells[-1][0], cells[-1][1] + "\n" + line)
            continue
        body = line[1:]
        span = 1
        attr = ATTRS.match(body)
        if attr:
            m = ROWSPAN.search(attr.group(1))
            span = int(m.group(1)) if m else 1
            body = attr.group(2)
        cells.append((span, body.strip()))
    return cells


def parse_table(text):
    """Return rows as dicts keyed by header name, honouring rowspans."""
    header = [h.strip().lower() for h in re.findall(r"^!\s*(.+)$", text, re.M)]
    rows, carry = [], {}
    for chunk in re.split(r"^\|-\s*$", text, flags=re.M)[1:]:
        cells = split_cells(chunk)
        if not cells:
            continue
        row, it = {}, iter(cells)
        for col, name in enumerate(header):
            if col in carry and carry[col][1] > 0:
                row[name] = carry[col][0]
                carry[col][1] -= 1
                continue
            try:
                span, val = next(it)
            except StopIteration:
                break
            row[name] = val
            if span > 1:
                carry[col] = [val, span - 1]
        rows.append(row)
    return rows


def parse_page(title, wt):
    speaker = re.search(r"\{\{\s*DT\s*\|([^}|]*)", wt)
    page = {"page": title, "speaker": clean(speaker.group(1)) if speaker else title[:-4],
            "topics": []}
    heads = [(m.start(), m.group(1).strip()) for m in
             re.finditer(r"^==\s*([^=\n]+?)\s*==\s*$", wt, re.M)]
    by_id = {}
    for m in re.finditer(r"\{\|.*?\n\|\}", wt, re.S):
        section = clean(next((h for p, h in reversed(heads) if p < m.start()), "Topics"))
        for row in parse_table(m.group(0)):
            topic_raw = row.get("topic", "")
            prompt_raw = row.get("topic text / prompt", "")
            response = clean(row.get("response text", ""))
            if not response:
                continue
            tid = clean(topic_raw.split("<br", 1)[0]) or None
            key = (section, tid or "", clean(prompt_raw))
            topic = by_id.get(key)
            if topic is None:
                topic = {"id": tid, "section": section.lower(), "responses": []}
                prompt = clean(prompt_raw)
                if prompt:
                    chk = CHECK.match(prompt)
                    if chk:
                        topic["check"] = {"name": chk.group(1).strip().lower(),
                                          "min": int(chk.group(2))}
                        prompt = prompt[chk.end():]
                    topic["prompt"] = prompt
                label = re.sub(r"^.*?<br\s*/?>", "", topic_raw, flags=re.S) if "<br" in topic_raw else ""
                if clean(label):
                    topic["label"] = clean(label)
                by_id[key] = topic
                page["topics"].append(topic)
            emo = EMOTION.match(clean(row.get("emotion", "")))
            resp = {"text": response}
            if emo:
                resp["emotion"] = emo.group(1).lower()
                resp["intensity"] = int(emo.group(2))
            topic["responses"].append(resp)
    return page


def cmd_build(cache, out):
    store = json.loads((cache / "dialogue_wikitext.json").read_text())
    out.mkdir(parents=True, exist_ok=True)
    index, empty = [], []
    for title in sorted(store):
        page = parse_page(title, store[title])
        if not page["topics"]:
            empty.append(title)
            continue
        fname = re.sub(r"[^A-Za-z0-9]+", "_", title[:-4] if title.endswith(".txt") else title).strip("_")
        (out / f"{fname}.json").write_text(json.dumps(page, indent=1, ensure_ascii=False) + "\n")
        index.append({"file": f"{fname}.json", "page": title, "speaker": page["speaker"],
                      "topics": len(page["topics"]),
                      "responses": sum(len(t["responses"]) for t in page["topics"])})
    (out / "index.json").write_text(json.dumps(index, indent=1, ensure_ascii=False) + "\n")
    (cache / "dialogue_empty.txt").write_text("\n".join(empty))
    print(f"{len(index)} transcripts, {sum(i['topics'] for i in index)} topics, "
          f"{sum(i['responses'] for i in index)} responses; {len(empty)} pages without a table")


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
