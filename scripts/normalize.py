#!/usr/bin/env python3
"""Turn the raw wiki imports into engine-ready data.

Usage:
    normalize.py <data_dir>   # reads data/{items,quests,npcs,dialogue,world,rules}, writes data/engine

Raw files are never modified. Output follows schemas/*.schema.json; run
scripts/validate.py afterwards.
"""
import json
import math
import re
import sys
from pathlib import Path

DLC = {
    "FNV": "base", "FNVDM": "dead_money", "FNVHH": "honest_hearts",
    "FNVOWB": "old_world_blues", "FNVLR": "lonesome_road",
    "FNVGRA": "gun_runners_arsenal",
}
SKILL = {
    "guns": "guns", "big guns": "guns", "energy weapons": "energy_weapons",
    "explosives": "explosives", "melee weapons": "melee_weapons",
    "unarmed": "unarmed",
}
WEAPON_CLASS = {
    "gun": "gun", "gunautomatic": "automatic", "gunhandload": "handload",
    "melee": "melee", "unarmed": "unarmed", "grenade": "grenade",
    "mine": "mine", "npc gun": "gun", "misc": "misc", "placed": "mine",
}
HEAD_WORDS = re.compile(
    r"helmet|\bhat\b|\bcap\b|\bhood\b|mask|goggles|beret|visor|headgear|bandana|"
    r"headdress|headband|crown|glasses|\bhead\b", re.I)
TITLE_SUFFIX = re.compile(r"\s*\((?:Fallout: New Vegas|quest|Honest Hearts)\)$")


def strip_title(title):
    title = re.sub(r"\s*\{\{.*?\}\}", "", title)
    return TITLE_SUFFIX.sub("", title).strip()


def slug(name):
    return re.sub(r"[^a-z0-9]+", "_", strip_title(name).lower()).strip("_") or "unnamed"


class Ids:
    """Hands out unique ids; a repeated slug gets the form id or a counter."""

    def __init__(self):
        self.used = set()

    def make(self, name, form_id=None):
        base = slug(name)
        cand = base
        if cand in self.used and form_id:
            cand = f"{base}_{form_id[-4:]}"
        n = 2
        while cand in self.used:
            cand = f"{base}_{n}"
            n += 1
        self.used.add(cand)
        return cand


def num(v):
    if isinstance(v, (int, float)):
        return v
    try:
        f = float(str(v).replace(",", ""))
    except ValueError:
        return None
    return int(f) if f.is_integer() else f


def first_hex(v):
    m = re.search(r"[0-9a-fA-F]{1,8}", str(v or ""))
    return m.group(0).lower().zfill(8) if m else None


def first_edid(v):
    if not v:
        return None
    return re.sub(r"\s*\(.*?\)", "", str(v).split(";")[0]).strip() or None


def dlc_of(games):
    return DLC.get(str(games).split(",")[0].strip(), "base")


def put(rec, key, value):
    if value is not None and value != "" and value != [] and value != {}:
        rec[key] = value


def common(rec, raw, name, ids, kind, item=None):
    src = item if item is not None else raw
    form_id = first_hex(src.get("formid"))
    out = {
        "id": ids.make(name, form_id), "name": name, "kind": kind,
        "dlc": dlc_of(raw.get("games", "FNV")),
        "value": num(src.get("value")) or 0,
        "weight": num(src.get("weight")) or 0,
        "source": raw["page"],
    }
    put(out, "edid", first_edid(src.get("edid")))
    put(out, "form_id", form_id)
    return out


# ------------------------------------------------------------------ items

def ammo_items(raw_ammo, ids):
    out, families = [], {}
    for page in raw_ammo:
        family = slug(page["page"])
        families[strip_title(page["page"]).lower()] = family
        base = None
        for item in page["items"]:
            name = item.get("item name") or strip_title(page["page"])
            base = base or name
            rec = common(page, page, name, ids, "ammo", item)
            rec["family"] = family
            rec["variant"] = ("standard" if name == base
                              else name.split(",", 1)[-1].strip())
            put(rec, "weight_hardcore", num(item.get("weight hardcore")))
            out.append(rec)
    return out, families


def resolve_ammo(text, families):
    key = strip_title(text.split(";")[0]).lower()
    for cand in (key, key + " round", key.replace(" round", "")):
        if cand in families:
            return families[cand]
    return next((f for k, f in families.items() if k.startswith(key)), None)


def weapons(raw, ids, families):
    out = []
    for r in raw:
        name = strip_title(r["page"])
        rec = common(r, r, name, ids, "weapon")
        wtype = str(r.get("type", "")).lower()
        skill = SKILL.get(str(r.get("skill", "")).lower())
        cls = WEAPON_CLASS.get(wtype.replace("unique", "").strip())
        if not cls:
            cls = {"melee_weapons": "melee", "unarmed": "unarmed"}.get(skill, "misc")
        rec["weapon_class"] = cls
        if "unique" in wtype or "unique" in r["category"].lower():
            rec["unique"] = True
        put(rec, "skill", skill)
        rec["damage"] = num(r.get("damage")) or 0
        for src, dst in (("crit dmg", "crit_damage"), ("crit % mult", "crit_chance_mult"),
                         ("attack shots/sec", "shots_per_sec"), ("ap", "ap_cost"),
                         ("strength req", "strength_req"), ("skill req", "skill_req"),
                         ("hp", "condition_hp"), ("min spread", "spread"),
                         ("projectiles", "projectiles"), ("ammo use", "ammo_per_shot"),
                         ("clip rounds", "clip_size"), ("reload time", "reload_time"),
                         ("explosion damage", "explosion_damage")):
            v = num(r.get(src))
            if v is None:
                continue
            if dst in ("strength_req", "skill_req", "projectiles", "clip_size"):
                v = int(v)
            put(rec, dst, v)
        if r.get("ammo"):
            fam = resolve_ammo(r["ammo"], families)
            put(rec, "ammo_family" if fam else "ammo_text", fam or r["ammo"])
        put(rec, "effect_text", r.get("other effect"))
        put(rec, "repair_text", r.get("repair"))
        out.append(rec)
    return out


def armor(raw, ids):
    out = []
    for r in raw:
        atype = str(r.get("type", "")).lower()
        power = "power armor" in r["category"].lower()
        multi = len(r["items"]) > 1
        for item in r["items"]:
            name = strip_title(item.get("item name") or r["page"])
            rec = common(r, r, name, ids, "armor", item)
            if "headwear" in atype or "helmet" in atype or (HEAD_WORDS.search(name) and (multi or "armor" in atype or not atype)):
                slot = "head"
            elif "clothing" in atype:
                slot = "clothing"
            else:
                slot = "body"
            rec["slot"] = slot
            cls = str(item.get("class", "")).lower()
            if power:
                rec["armor_class"] = "power"
            elif cls in ("light", "medium", "heavy"):
                rec["armor_class"] = cls
            rec["dt"] = num(item.get("dt")) or 0
            put(rec, "dr", num(item.get("dr")))
            put(rec, "condition_hp", num(item.get("hp")))
            put(rec, "effects_text", item.get("effects"))
            put(rec, "repair_text", item.get("repair"))
            if "unique" in atype or "unique" in r["category"].lower():
                rec["unique"] = True
            out.append(rec)
    return out


STAT_EFFECT = re.compile(
    r"^([+-]\s?\d+(?:\.\d+)?)\s+([A-Za-z][A-Za-z ]*?)(/s| per second)?"
    r"(?:\s+for\s+(\d+(?:\.\d+)?)\s*(seconds?|secs?|s|minutes?|mins?|min|m|hours?|h)\b\.?)?\s*$")
UNIT_S = {"s": 1, "m": 60, "h": 3600}


def parse_effects(text):
    out = []
    for part in text.split(";"):
        m = STAT_EFFECT.match(part.strip())
        if not m:
            continue
        eff = {"stat": m.group(2).strip(), "amount": float(m.group(1).replace(" ", ""))}
        if eff["amount"].is_integer():
            eff["amount"] = int(eff["amount"])
        if m.group(3):
            eff["per_second"] = True
        if m.group(4):
            eff["duration_s"] = float(m.group(4)) * UNIT_S[m.group(5)[0].lower()]
        out.append(eff)
    return out


def consumables(raw, ids):
    out = []
    for r in raw:
        rec = common(r, r, strip_title(r["page"]), ids, "consumable")
        text = str(r.get("effects", ""))
        rec["effects_text"] = text
        put(rec, "effects_hardcore_text", r.get("effects hc"))
        effects = parse_effects(text)
        if effects:
            rec["effects"] = effects
        add = str(r.get("addiction", ""))
        if add and add.lower() != "none":
            m = re.match(r"([A-Za-z][\w -]*?)\s+addiction[;:]?\s*(\d+(?:\.\d+)?)?", add)
            rec["addiction"] = {"type": (m.group(1) if m else add).strip().lower()}
            if m and m.group(2):
                rec["addiction"]["chance_percent"] = float(m.group(2))
        put(rec, "skill", r.get("skill"))
        out.append(rec)
    return out


def weapon_mods(raw, ids, weapon_recs):
    by_name = {}
    for w in weapon_recs:
        by_name.setdefault(w["name"].lower(), []).append(w["id"])
    out = []
    for r in raw:
        rec = common(r, r, strip_title(r["page"]), ids, "weapon_mod")
        rec["effects_text"] = str(r.get("effects", ""))
        mod = str(r.get("modifies", ""))
        put(rec, "modifies_text", mod)
        found = [i for part in mod.split(";")
                 for i in by_name.get(strip_title(part).lower(), [])]
        put(rec, "modifies", found)
        out.append(rec)
    return out


# ----------------------------------------------------------------- quests

def quests(files, consequences, ids):
    raw = [q for f in files for q in json.load(open(f))]
    title_id = {}
    recs = {}
    for q in sorted(raw, key=lambda q: q["page"]):
        info = q["infobox"]
        rec = {"id": ids.make(q["page"], first_hex(info.get("formid"))),
               "name": strip_title(q["page"])}
        title_id[q["page"]] = rec["id"]
        title_id.setdefault(strip_title(q["page"]), rec["id"])
        recs[q["page"]] = (q, rec)

    for page, (q, rec) in recs.items():
        info = q["infobox"]
        qtype = str(info.get("type", "")).lower()
        rec["kind"] = "main" if qtype == "main" else "side"
        rec["tracked"] = qtype != "unmarked"
        rec["dlc"] = dlc_of(info.get("games", "FNV"))
        put(rec, "edid", first_edid(info.get("edid")))
        put(rec, "form_id", first_hex(info.get("formid")))
        rec["source"] = page
        put(rec, "giver", info.get("given by") or None)
        put(rec, "location", info.get("location") or None)
        put(rec, "reward_text", info.get("reward"))
        m = re.search(r"(\d[\d,]*)\s*(?:\[\[)?\s*XP", str(info.get("reward", "")))
        if m:
            rec["xp"] = int(m.group(1).replace(",", ""))
        unresolved = []
        for src, dst in (("previous", "previous"), ("leads to", "leads_to"), ("related", "related")):
            found = []
            for t in info.get(src, []):
                if t in title_id:
                    found.append(title_id[t])
                else:
                    unresolved.append(t)
            put(rec, dst, sorted(set(found)))
        put(rec, "unresolved_links", sorted(set(unresolved)))
        rec["stages"] = [
            {k: v for k, v in (("stage", s["stage"]), ("description", s["desc"]),
                               ("finish", True if s.get("status") == "finish" else None))
             if v is not None}
            for s in q["stages"] if isinstance(s["stage"], int)]
        chart = q["chart"]
        rows = chart.get("Quick walkthrough") or next(iter(chart.values()), [])
        put(rec, "walkthrough", rows)
        put(rec, "notes", q.get("notes", []) + q.get("outcomes", []))
        rules = []
        for c in consequences:
            if c["quest"] != page:
                continue
            rule = {k: c[k] for k in ("kind", "target", "target_type", "when", "note") if k in c}
            if c["target_type"] == "quest":
                rule["target_id"] = title_id[c["target"]]
            rules.append(rule)
        put(rec, "rules", rules)
    return sorted((rec for _, rec in recs.values()), key=lambda r: r["id"]), title_id


# ------------------------------------------------------------ npcs, dialogue

SKILLS = {"barter", "energy_weapons", "explosives", "guns", "lockpick", "medicine",
          "melee_weapons", "repair", "science", "sneak", "speech", "survival", "unarmed"}
SPECIALS = {"strength", "perception", "endurance", "charisma", "intelligence", "agility", "luck"}
SECTIONS = {"topics", "conversation", "combat", "detection"}


def npc_dlc(games):
    tokens = [t.strip() for t in str(games).split(",") if t.strip()]
    fnv = next((t for t in tokens if t.startswith("FNV")), None)
    return DLC.get(fnv, "base"), (fnv is not None or not tokens or tokens == ["none"])


def npcs(raw, title_id, dialogue_by_page):
    ids = Ids()
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        dlc, keep = npc_dlc(r.get("games", ""))
        if not keep:
            continue
        form_id = first_hex(r.get("formid"))
        rec = {"id": ids.make(r["name"], form_id), "name": strip_title(r["name"]),
               "dlc": dlc, "source": r["page"]}
        put(rec, "edid", first_edid(r.get("edid")))
        put(rec, "form_id", form_id)
        for key in ("race", "gender", "karma", "aggression", "confidence"):
            put(rec, key, r.get(key))
        put(rec, "level", r.get("level"))
        put(rec, "special", r.get("special"))
        put(rec, "skills", {k: v for k, v in r.get("tag_skills", {}).items() if k in SKILLS})
        d = r.get("derived", {})
        for key in ("hp", "hp_max", "dt"):
            put(rec, key, d.get(key))
        roles = r.get("role", [])
        if any("companion" in x.lower() for x in roles):
            rec["companion"] = True
        put(rec, "affiliations", r.get("affiliation"))
        put(rec, "roles", roles)
        put(rec, "locations", r.get("location"))
        put(rec, "geck_factions", r.get("geck_factions"))
        quest_ids = sorted({title_id[q] for q in r.get("quests", []) if q in title_id})
        put(rec, "quests", quest_ids)
        put(rec, "unresolved_quests", sorted({q for q in r.get("quests", []) if q not in title_id}))
        dlg = sorted({dialogue_by_page[p] for p in r.get("dialogue_pages", []) if p in dialogue_by_page})
        put(rec, "dialogue", dlg)
        out.append(rec)
    return out


def dialogue_check(check):
    name = str(check.get("name", "")).replace(" ", "_")
    if name in SKILLS:
        return {"type": "skill", "skill": name, "min": min(check["min"], 100)}
    if name in SPECIALS and 1 <= check["min"] <= 10:
        return {"type": "special", "stat": name, "min": check["min"]}
    return None


def dialogue(files):
    """Return (docs, page_to_id); `speaker` is filled in by link_speakers."""
    ids = Ids()
    docs, by_page = [], {}
    for f in files:
        raw = json.load(open(f))
        did = ids.make(Path(f).stem)
        by_page[raw["page"]] = did
        topics, keys = [], Ids()
        for t in raw["topics"]:
            sec = t["section"] if t["section"] in SECTIONS else "other"
            rec = {"key": keys.make(t.get("id") or f"{sec}_topic"), "section": sec,
                   "responses": t["responses"]}
            put(rec, "topic", t.get("id"))
            put(rec, "prompt", t.get("prompt"))
            put(rec, "label", t.get("label"))
            if "check" in t:
                put(rec, "check", dialogue_check(t["check"]))
            topics.append(rec)
        docs.append({"id": did, "speaker_name": raw["speaker"], "source": raw["page"],
                     "topics": topics})
    return docs, by_page


def link_speakers(docs, npc_recs):
    by_name = {}
    for n in npc_recs:
        by_name.setdefault(n["name"].lower(), []).append(n["id"])
    by_dialogue = {d: n["id"] for n in npc_recs for d in n.get("dialogue", [])}
    for doc in docs:
        speaker = by_dialogue.get(doc["id"])
        if not speaker:
            hits = by_name.get(doc["speaker_name"].lower(), [])
            speaker = hits[0] if len(hits) == 1 else None
            if speaker:
                npc = next(n for n in npc_recs if n["id"] == speaker)
                npc.setdefault("dialogue", []).append(doc["id"])
        put(doc, "speaker", speaker)


def link_givers(quest_recs, npc_recs):
    """Match quest givers by display name or by wiki page title."""
    by_name = {}
    for n in npc_recs:
        for key in {n["name"].lower(), n["source"].lower()}:
            by_name.setdefault(key, []).append(n["id"])
    for q in quest_recs:
        found = [by_name[g.lower()][0] for g in q.get("giver", [])
                 if len(set(by_name.get(g.lower(), []))) == 1]
        put(q, "giver_ids", found)


# ------------------------------------------------------------------- world

def display(page):
    """Page title without a trailing disambiguation like ' (Fallout: New Vegas)'."""
    return re.sub(r"\s*\([^)]*\)$", "", strip_title(page)).strip()


def lookup(recs, *keys):
    """Map lowercase names/titles to the ids of records that own them uniquely."""
    seen = {}
    for r in recs:
        for k in keys:
            if r.get(k):
                seen.setdefault(str(r[k]).lower(), set()).add(r["id"])
    return {k: next(iter(v)) for k, v in seen.items() if len(v) == 1}


def travel_graph(nodes, k=3):
    """Edges between map nodes: each node's k nearest plus a minimum spanning tree."""
    n = len(nodes)
    dist = lambda a, b: math.hypot(a["x"] - b["x"], a["y"] - b["y"])
    edges = set()
    for i, a in enumerate(nodes):
        near = sorted((dist(a, b), j) for j, b in enumerate(nodes) if j != i)[:k]
        edges |= {(min(i, j), max(i, j)) for _, j in near}
    parent = list(range(n))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    for d, i, j in sorted((dist(nodes[i], nodes[j]), i, j)
                          for i in range(n) for j in range(i + 1, n)):
        if find(i) != find(j):
            parent[find(i)] = find(j)
            edges.add((i, j))
    adj = {i: [] for i in range(n)}
    for i, j in edges:
        d = round(dist(nodes[i], nodes[j]), 1)
        adj[i].append((nodes[j]["id"], d))
        adj[j].append((nodes[i]["id"], d))
    return {nodes[i]["id"]: [{"id": t, "distance": d} for t, d in sorted(v, key=lambda e: e[1])]
            for i, v in adj.items()}


def locations(raw, markers, title_id, npc_recs):
    ids = Ids()
    recs = []
    id_by_title = {}
    for r in sorted(raw, key=lambda r: r["page"]):
        id_by_title[r["page"]] = ids.make(display(r["page"]), None)
    npc_ids = lookup(npc_recs, "source", "name")
    matched = set()
    for r in sorted(raw, key=lambda r: r["page"]):
        kind, *rest = str(r.get("type", "unknown")).lower().split() or ["unknown"]
        rec = {"id": id_by_title[r["page"]], "name": display(r["page"]), "kind": kind,
               "dlc": npc_dlc(r.get("games", ""))[0], "source": r["page"]}
        if rest and rest[0] in ("small", "medium", "large"):
            rec["size"] = rest[0]
        put(rec, "cell", r.get("cell_name"))
        put(rec, "region", r.get("overview_section"))
        mk = r.get("marker")
        if mk:
            matched.add((mk["map"], mk["x"], mk["y"]))
            rec["marked"] = True
            rec["map"] = mk["map"]
            rec["position"] = {"x": mk["x"], "y": mk["y"]}
            put(rec, "map_category", mk.get("category"))
        part = r.get("part_of", [])
        if part:
            if part[0] in id_by_title:
                rec["parent"] = id_by_title[part[0]]
            else:
                rec["parent_text"] = display(part[0])
        put(rec, "children", sorted({id_by_title[c] for key in ("sections", "buildings")
                                     for c in r.get(key, []) if c in id_by_title}))
        put(rec, "factions", [display(f) for f in r.get("factions", [])])
        unresolved = []
        for key in ("leaders", "merchants", "doctors"):
            found = []
            for name in r.get(key, []):
                hit = npc_ids.get(name.lower())
                if hit:
                    found.append(hit)
                else:
                    unresolved.append(display(name))
            put(rec, key, sorted(set(found)))
        put(rec, "people_unresolved", sorted(set(unresolved)))
        put(rec, "quests", sorted({title_id[q] for q in r.get("quests", []) if q in title_id}))
        put(rec, "creatures", [display(c) for c in r.get("creatures", [])])
        recs.append(rec)
    for m in markers:
        if (m["map"], m["x"], m["y"]) in matched:
            continue
        rec = {"id": ids.make(m["name"] or m["page"]), "name": m["name"] or m["page"],
               "kind": str(m.get("category") or "marker").lower().split()[0],
               "dlc": m["map"], "marked": True, "map": m["map"],
               "position": {"x": m["x"], "y": m["y"]}, "source": m["page"] or m["name"]}
        put(rec, "map_category", m.get("category"))
        recs.append(rec)
    for dlc in {r["map"] for r in recs if "map" in r}:
        nodes = [{"id": r["id"], **r["position"]} for r in recs if r.get("map") == dlc]
        if len(nodes) < 2:
            continue
        graph = travel_graph(nodes)
        for r in recs:
            if r.get("map") == dlc:
                r["neighbors"] = graph[r["id"]]
    return sorted(recs, key=lambda r: r["id"])


def perks_and_traits(raw, kind):
    ids = Ids()
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        form_id = first_hex(r.get("formid"))
        rec = {"id": ids.make(display(r["page"]), form_id), "name": display(r["page"]),
               "dlc": npc_dlc(r.get("games", ""))[0], "source": r["page"]}
        put(rec, "form_id", form_id)
        put(rec, "description", r.get("description"))
        if kind == "perk":
            if isinstance(r.get("ranks"), int) and r["ranks"] >= 1:
                rec["ranks"] = r["ranks"]
            put(rec, "requirements", r.get("requirements"))
            put(rec, "requires_text", r.get("requires_text"))
            put(rec, "effects_text", r.get("effects_text"))
        else:
            for key in ("requires", "benefits", "penalties"):
                put(rec, key, r.get(key))
        out.append(rec)
    return out


def factions(raw, rep):
    ids = Ids()
    skip = {"page", "category", "games", "mentions", "formid", "leader", "leaders",
            "locations", "location"}
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        form_id = first_hex(r.get("formid"))
        rec = {"id": ids.make(display(r["page"]), form_id), "name": display(r["page"]),
               "kind": "faction", "dlc": npc_dlc(r.get("games", ""))[0], "source": r["page"]}
        put(rec, "form_id", form_id)
        put(rec, "leaders", [display(x) for key in ("leader", "leaders") for x in r.get(key, [])
                             if isinstance(r.get(key), list)])
        put(rec, "locations", [display(x) for key in ("locations", "location")
                               for x in r.get(key, []) if isinstance(r.get(key), list)])
        put(rec, "info", {k: v for k, v in r.items() if k not in skip})
        out.append(rec)
    by_name = {f["name"].lower(): f for f in out}
    for ent in rep["entities"]:
        f = by_name.get(ent["name"].lower())
        if f is None:
            f = {"id": ids.make(ent["name"]), "name": ent["name"], "kind": "town", "dlc": "base"}
            out.append(f)
            by_name[ent["name"].lower()] = f
        block = {}
        if len(ent.get("ranges", [])) == 4 and len(ent.get("titles", [])) == 4 \
                and all(len(t) == 4 for t in ent["titles"]):
            block["thresholds"] = [int(re.match(r"\d+", x).group(0)) for x in ent["ranges"]]
            block["titles"] = ent["titles"]
        for key in ("consequences", "gains", "losses"):
            put(block, key, ent.get(key))
        put(f, "reputation", block)
    return sorted(out, key=lambda r: r["id"])


def reputation_rules(rep, faction_recs):
    by_name = {f["name"].lower(): f["id"] for f in faction_recs}
    benefits = [{"faction": by_name[b["faction"].lower()], "reputation": b["reputation"],
                 "effect": b["effect"]}
                for b in rep["benefits"] if b.get("faction", "").lower() in by_name
                and b.get("reputation")]
    return {"bump_points": rep["bump_breakdown"], "benefits": benefits}


def creatures(raw):
    ids = Ids()
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        rec = {"id": ids.make(display(r["page"])), "name": display(r["page"]),
               "dlc": npc_dlc(r.get("games", ""))[0], "source": r["page"]}
        put(rec, "type", r.get("type"))
        put(rec, "locations", [display(x) for x in r.get("location", [])])
        vids = Ids()
        variants = []
        for v in r["variants"]:
            form_id = first_hex(v.get("formid"))
            var = {"id": vids.make(v["name"], form_id), "name": strip_title(v["name"])}
            put(var, "group", v.get("group"))
            put(var, "form_id", form_id)
            for key in ("level", "xp", "perception", "hp", "dt", "dr", "aggression",
                        "confidence", "assistance"):
                if isinstance(v.get(key), (int, float)):
                    var[key] = v[key]
            put(var, "attacks", v.get("attacks"))
            put(var, "items", v.get("items"))
            variants.append(var)
        rec["variants"] = variants
        out.append(rec)
    return out


def link_names(npc_recs, quest_recs, loc_recs, faction_recs):
    """Add *_ids fields by matching wiki names to factions and locations."""
    fac = lookup(faction_recs, "name", "source")
    loc = lookup(loc_recs, "name", "source")

    def match(names, table):
        return sorted({table[display(n).lower()] for n in names if display(n).lower() in table}
                      | {table[n.lower()] for n in names if n.lower() in table})
    for n in npc_recs:
        put(n, "faction_ids", match(n.get("affiliations", []), fac))
        put(n, "location_ids", match(n.get("locations", []), loc))
    for q in quest_recs:
        put(q, "location_ids", match(q.get("location", []), loc))
    for l in loc_recs:
        put(l, "faction_ids", match(l.get("factions", []), fac))


# ------------------------------------------------------------------- rules

SKILL_NAMES = ["barter", "energy_weapons", "explosives", "guns", "lockpick", "medicine",
               "melee_weapons", "repair", "science", "sneak", "speech", "survival", "unarmed"]
SPECIAL_IDS = ["strength", "perception", "endurance", "charisma", "intelligence", "agility", "luck"]


def skill_books(raw, ids):
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        form_id = first_hex(r.get("formid"))
        text = str(r.get("effects", ""))
        rec = {"id": ids.make(display(r["page"]), form_id), "name": display(r["page"]),
               "kind": "skill_book", "dlc": npc_dlc(r.get("games", ""))[0],
               "value": num(r.get("value")) or 0, "weight": num(r.get("weight")) or 0,
               "source": r["page"], "effects_text": text}
        put(rec, "edid", first_edid(r.get("edid")))
        put(rec, "form_id", form_id)
        skill = next((sk for sk in SKILL_NAMES if sk.replace("_", " ") in text.lower()), None)
        if skill is None:
            continue  # e.g. the Skill book recipe, which trains no skill
        rec["skill"] = skill
        amount = re.search(r"(\d+)", text)
        if amount:
            rec["amount"] = int(amount.group(1))
        if r["kind"] == "skillmagazine":
            rec["periodical"] = True
        dur = re.search(r"for\s+(\d+)\s*(sec|min|hour|s|m|h)\b", text, re.I)
        if dur:
            rec["duration_s"] = int(dur.group(1)) * {"s": 1, "m": 60, "h": 3600}[dur.group(2)[0].lower()]
        out.append(rec)
    return out


def misc_items(raw, ids, seen_pages):
    """Crafting components not already covered by another item group."""
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        if r["page"] in seen_pages:
            continue
        rec = common(r, r, display(r["page"]), ids, "misc")
        put(rec, "item_type", r.get("type"))
        put(rec, "component_of_text", r.get("component of"))
        put(rec, "effects_text", r.get("effects"))
        out.append(rec)
    return out


def rule_stats(raw, perk_recs):
    perks = lookup(perk_recs, "name", "source")
    ids = Ids()
    out = []
    for r in raw:
        rec = {"id": ids.make(display(r["page"])), "name": display(r["page"]),
               "kind": r["kind"], "source": r["page"]}
        put(rec, "modifies", r.get("modifies"))
        gov = [display(t).lower() for t in r.get("governed_by_links", [])]
        put(rec, "governed_by", [g for g in gov if g in SPECIAL_IDS])
        put(rec, "initial_value", r.get("initial_value"))
        put(rec, "math", r.get("math"))
        put(rec, "related_perks", sorted({perks[t.lower()] for t in r.get("related_perk_links", [])
                                          if t.lower() in perks}))
        put(rec, "form_id", first_hex(r.get("formid")))
        out.append(rec)
    return out


def challenges(raw):
    ids = Ids()
    out = []
    for r in sorted(raw, key=lambda r: r["page"]):
        form_id = first_hex(r.get("formid"))
        rec = {"id": ids.make(display(r["page"]), form_id), "name": display(r["page"]),
               "dlc": npc_dlc(r.get("games", ""))[0], "source": r["page"],
               "task": r.get("task", "")}
        put(rec, "requires", r.get("requires"))
        if r.get("repeats"):
            rec["repeatable"] = "not" not in r["repeats"].lower()
        put(rec, "reward_text", r.get("reward"))
        xp = re.search(r"(\d[\d,]*)\s*XP", str(r.get("reward", "")))
        if xp:
            rec["xp"] = int(xp.group(1).replace(",", ""))
        ctype = re.sub(r"\s*challenge$", "", r.get("challenge_type") or "").strip()
        put(rec, "challenge_type", ctype)
        put(rec, "form_id", form_id)
        out.append(rec)
    return out


def recipes(raw, item_recs):
    by_name = lookup(item_recs, "name")
    ids = Ids()
    out = []
    for r in raw:
        rec = {"id": ids.make(r["name"] + " " + r["mode"]), "name": r["name"], "mode": r["mode"],
               "table": r["table"]}
        put(rec, "output_qty", r.get("name_qty"))
        item = by_name.get(r["name"].lower())
        put(rec, "output_item_id", item)
        sk = r.get("skill")
        if sk:
            skill = slug(sk["skill"])
            if skill in SKILL_NAMES:
                rec["skill"] = {"skill": skill, "min": min(sk["min"], 100)}
        put(rec, "station", r.get("station"))
        ing = []
        for i in r["items"]:
            name = re.sub(r"\s*\|\}\s*$", "", i["item"]).strip()
            item_id = by_name.get(name.lower())
            noise = (len(name) > 45 or name.endswith(".") or re.fullmatch(r"[\d\s]+", name)
                     or re.search(r"\bseconds?\b|per second|schematic|^\w+: \d+$", name, re.I))
            if noise and not item_id:
                continue
            e = {"item": name, "qty": max(1, i["qty"])}
            put(e, "item_id", item_id)
            ing.append(e)
        rec["ingredients"] = ing
        out.append(rec)
    return out


def endings(raw, title_id):
    ids = Ids()
    out = []
    for r in raw:
        rec = {"id": ids.make(f"{r['slide']} {r['variant']}"), "slide": r["slide"],
               "narration": r["narration"]}
        put(rec, "variant", r.get("variant"))
        put(rec, "condition", r.get("condition"))
        put(rec, "condition_quests", sorted({title_id[t] for t in r.get("condition_links", [])
                                             if t in title_id}))
        out.append(rec)
    return out


def levels(raw):
    out = []
    for r in raw:
        rec = {"level": r["level"], "dlc_required": r["dlc_required"], "karma_titles": r["karma_titles"]}
        for key in ("xp_total", "xp_to_next"):
            if isinstance(r.get(key), int):
                rec[key] = r[key]
        out.append(rec)
    return out


def dump(path, data):
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(f"{path.name}: {len(data)}")


def main(argv):
    if len(argv) != 2:
        print(__doc__)
        return 2
    data = Path(argv[1])
    out = data / "engine"
    out.mkdir(exist_ok=True)
    load = lambda n: json.load(open(data / "items" / f"{n}.json"))
    ids = Ids()
    ammo, families = ammo_items(load("ammo"), ids)
    weps = weapons(load("weapons"), ids, families)
    dump(out / "ammo.json", ammo)
    dump(out / "weapons.json", weps)
    dump(out / "armor.json", armor(load("armor"), ids))
    book_pages = {b["page"] for b in json.load(open(data / "rules" / "skill_books.json"))}
    dump(out / "consumables.json",
         consumables([c for c in load("consumables") if c["page"] not in book_pages], ids))
    mods = weapon_mods(load("weapon_mods"), ids, weps)
    dump(out / "weapon_mods.json", mods)
    rdir = data / "rules"
    rl = lambda n: json.load(open(rdir / f"{n}.json"))
    books = skill_books(rl("skill_books"), ids)
    seen = {x["page"] for g in ("weapons", "armor", "ammo", "consumables", "weapon_mods")
            for x in load(g)} | book_pages
    misc = misc_items(load("components"), ids, seen)
    dump(out / "skill_books.json", books)
    dump(out / "misc.json", misc)
    files = [f for f in sorted((data / "quests").glob("*.json")) if f.name != "consequences.json"]
    cons = json.load(open(data / "quests" / "consequences.json"))
    quest_recs, title_id = quests(files, cons, Ids())
    dfiles = [f for f in sorted((data / "dialogue").glob("*.json")) if f.name != "index.json"]
    docs, dialogue_by_page = dialogue(dfiles)
    npc_recs = npcs(json.load(open(data / "npcs" / "npcs.json")), title_id, dialogue_by_page)
    link_speakers(docs, npc_recs)
    link_givers(quest_recs, npc_recs)
    wdir = data / "world"
    wl = lambda n: json.load(open(wdir / f"{n}.json"))
    rep_raw = wl("reputation")
    faction_recs = factions(wl("factions"), rep_raw)
    loc_recs = locations(wl("locations"), wl("map_markers"), title_id, npc_recs)
    link_names(npc_recs, quest_recs, loc_recs, faction_recs)
    dump(out / "locations.json", loc_recs)
    dump(out / "perks.json", perks_and_traits(wl("perks"), "perk"))
    dump(out / "traits.json", perks_and_traits(wl("traits"), "trait"))
    dump(out / "factions.json", faction_recs)
    (out / "reputation.json").write_text(json.dumps(
        reputation_rules(rep_raw, faction_recs), indent=1, ensure_ascii=False) + "\n")
    dump(out / "creatures.json", creatures(wl("creatures")))
    item_recs = [r for f in ("weapons", "armor", "ammo", "consumables", "weapon_mods",
                             "skill_books", "misc") for r in json.load(open(out / f"{f}.json"))]
    dump(out / "stats.json", rule_stats(rl("stats"), json.load(open(out / "perks.json"))))
    dump(out / "levels.json", levels(rl("levels")))
    dump(out / "challenges.json", challenges(rl("challenges")))
    dump(out / "recipes.json", recipes(rl("recipes"), item_recs))
    dump(out / "endings.json", endings(rl("endings"), title_id))
    (out / "formulas.json").write_text((rdir / "formulas.json").read_text())
    dump(out / "quests.json", quest_recs)
    dump(out / "npcs.json", npc_recs)
    ddir = out / "dialogue"
    ddir.mkdir(exist_ok=True)
    for old in ddir.glob("*.json"):
        old.unlink()
    for doc in docs:
        (ddir / f"{doc['id']}.json").write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print(f"dialogue: {len(docs)} files, "
          f"{sum(1 for d in docs if d.get('speaker'))} linked to an NPC")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
