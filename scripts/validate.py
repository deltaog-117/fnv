#!/usr/bin/env python3
"""Validate data/engine against schemas/ and check cross-references
(items, quests, npcs, dialogue, locations, perks, traits, factions,
reputation, creatures).

Usage:
    validate.py <repo_root>

Needs the `jsonschema` package (pip install jsonschema). Exits 1 on any error.
"""
import ast
import json
import math
import sys
from collections import Counter
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ITEM_FILES = ("weapons", "armor", "ammo", "consumables", "weapon_mods", "skill_books", "misc")
MAX_SHOWN = 8


SAFE_FUNCS = {"floor": math.floor, "ceil": math.ceil, "min": min, "max": max}
SAFE_NODES = (ast.Expression, ast.BinOp, ast.UnaryOp, ast.Constant, ast.Name, ast.Call,
              ast.Load, ast.Add, ast.Sub, ast.Mult, ast.Div, ast.USub, ast.UAdd)


def check_expression(expr, allowed):
    """Return an error message, or None if expr only uses safe nodes and allowed names."""
    try:
        tree = ast.parse(expr, mode="eval")
    except SyntaxError as exc:
        return f"syntax error: {exc.msg}"
    for node in ast.walk(tree):
        if not isinstance(node, SAFE_NODES):
            return f"disallowed syntax {type(node).__name__}"
        if isinstance(node, ast.Call) and not (
                isinstance(node.func, ast.Name) and node.func.id in SAFE_FUNCS):
            return "only floor, ceil, min and max may be called"
        if isinstance(node, ast.Name) and node.id not in allowed and node.id not in SAFE_FUNCS:
            return f"unknown name {node.id}"
    return None


def walk_vocabulary(nodes):
    """Yield every condition/effect node, descending into all/any/not."""
    for node in nodes:
        yield node
        inner = node.get("of")
        if inner is not None:
            yield from walk_vocabulary(inner if isinstance(inner, list) else [inner])


def refs_in(nodes, field):
    """Every value of `field` in the condition/effect nodes, nested ones included."""
    return [node[field] for node in walk_vocabulary(nodes) if field in node]


def evaluate(expr, env):
    return eval(compile(ast.parse(expr, mode="eval"), "<formula>", "eval"),
                {"__builtins__": {}}, {**SAFE_FUNCS, **env})


def main(argv):
    if len(argv) != 2:
        print(__doc__)
        return 2
    root = Path(argv[1])
    registry = Registry()
    for f in (root / "schemas").glob("*.schema.json"):
        doc = json.load(open(f))
        registry = registry.with_resource(doc["$id"], Resource.from_contents(doc))
    validators = {
        name: Draft202012Validator({"$ref": f"{name}.schema.json"}, registry=registry)
        for name in ("item", "quest", "npc", "dialogue", "location", "perk", "trait",
                     "faction", "reputation", "creature", "encounter", "vendor")
    }

    for name in ("stat", "challenge", "recipe", "ending", "level", "formulas"):
        validators[name] = Draft202012Validator(
            {"$ref": f"rules.schema.json#/$defs/{name}"}, registry=registry)

    errors = Counter()
    shown = Counter()

    def report(group, rec_id, msg):
        errors[group] += 1
        if shown[group] < MAX_SHOWN:
            shown[group] += 1
            print(f"  [{group}] {rec_id}: {msg}")

    engine = root / "data" / "engine"
    items = {}
    ids = Counter()
    for name in ITEM_FILES:
        recs = json.load(open(engine / f"{name}.json"))
        items[name] = recs
        for rec in recs:
            ids[rec["id"]] += 1
            for err in validators["item"].iter_errors(rec):
                report(name, rec.get("id"), err.message[:160])
    quests = json.load(open(engine / "quests.json"))
    quest_ids = Counter(q["id"] for q in quests)
    for q in quests:
        for err in validators["quest"].iter_errors(q):
            report("quests", q["id"], err.message[:160])

    npcs = json.load(open(engine / "npcs.json"))
    npc_ids = Counter(n["id"] for n in npcs)
    for n in npcs:
        for err in validators["npc"].iter_errors(n):
            report("npcs", n["id"], err.message[:160])
        for qid in n.get("quests", []):
            if qid not in quest_ids:
                report("refs", n["id"], f"unknown quest {qid}")
    dialogue = [json.load(open(f)) for f in sorted((engine / "dialogue").glob("*.json"))]
    dialogue_ids = Counter(d["id"] for d in dialogue)
    for d in dialogue:
        for err in validators["dialogue"].iter_errors(d):
            report("dialogue", d["id"], err.message[:160])
        if "speaker" in d and d["speaker"] not in npc_ids:
            report("refs", d["id"], f"unknown speaker {d['speaker']}")
        keys = Counter(t["key"] for t in d["topics"])
        for k, c in keys.items():
            if c > 1:
                report("ids", d["id"], f"duplicate topic key {k}")
        for key in d.get("start", []):
            if key not in keys:
                report("refs", d["id"], f"start lists unknown topic {key}")
        for t in d["topics"]:
            for key in t.get("next", []):
                if key not in keys:
                    report("refs", d["id"], f"{t['key']} lists unknown next {key}")
            if t.get("ends") and t.get("next"):
                report("dialogue", d["id"], f"{t['key']} ends the conversation but has next")
            for ref in refs_in(t.get("conditions", []) + t.get("effects", [])
                               + ([t["check"]] if "check" in t else []), "quest"):
                if ref not in quest_ids:
                    report("refs", d["id"], f"{t['key']} names unknown quest {ref}")
            for ref in refs_in(t.get("conditions", []) + t.get("effects", []), "npc"):
                if ref not in npc_ids:
                    report("refs", d["id"], f"{t['key']} names unknown npc {ref}")
    for n in npcs:
        for did in n.get("dialogue", []):
            if did not in dialogue_ids:
                report("refs", n["id"], f"unknown dialogue {did}")
    for q in quests:
        for gid in q.get("giver_ids", []):
            if gid not in npc_ids:
                report("refs", q["id"], f"unknown giver {gid}")
    def load_group(fname, validator, group):
        recs = json.load(open(engine / fname))
        counts = Counter(r["id"] for r in recs)
        for r in recs:
            for err in validators[validator].iter_errors(r):
                report(group, r["id"], err.message[:160])
        for i, c in counts.items():
            if c > 1:
                report("ids", i, f"duplicate {group} id")
        return recs, counts

    locs, loc_ids = load_group("locations.json", "location", "locations")
    for loc in locs:
        for key in ("children",):
            for ref in loc.get(key, []):
                if ref not in loc_ids:
                    report("refs", loc["id"], f"unknown location {ref}")
        if "parent" in loc and loc["parent"] not in loc_ids:
            report("refs", loc["id"], f"unknown parent {loc['parent']}")
        for nb in loc.get("neighbors", []):
            if nb["id"] not in loc_ids:
                report("refs", loc["id"], f"unknown neighbor {nb['id']}")
        for key in ("leaders", "merchants", "doctors"):
            for ref in loc.get(key, []):
                if ref not in npc_ids:
                    report("refs", loc["id"], f"unknown npc {ref}")
        for ref in loc.get("quests", []):
            if ref not in quest_ids:
                report("refs", loc["id"], f"unknown quest {ref}")
    perks, _ = load_group("perks.json", "perk", "perks")
    traits, _ = load_group("traits.json", "trait", "traits")
    factions, faction_ids = load_group("factions.json", "faction", "factions")
    creatures, _ = load_group("creatures.json", "creature", "creatures")
    for c in creatures:
        vids = Counter(v["id"] for v in c["variants"])
        for i, n in vids.items():
            if n > 1:
                report("ids", c["id"], f"duplicate variant id {i}")
    reputation = json.load(open(engine / "reputation.json"))
    for err in validators["reputation"].iter_errors(reputation):
        report("reputation", "reputation.json", err.message[:160])
    for b in reputation["benefits"]:
        if b["faction"] not in faction_ids:
            report("refs", "reputation.json", f"unknown faction {b['faction']}")
    for n in npcs:
        for ref in n.get("faction_ids", []):
            if ref not in faction_ids:
                report("refs", n["id"], f"unknown faction {ref}")
        for ref in n.get("location_ids", []):
            if ref not in loc_ids:
                report("refs", n["id"], f"unknown location {ref}")
    for q in quests:
        for ref in q.get("location_ids", []):
            if ref not in loc_ids:
                report("refs", q["id"], f"unknown location {ref}")
    for loc in locs:
        for ref in loc.get("faction_ids", []):
            if ref not in faction_ids:
                report("refs", loc["id"], f"unknown faction {ref}")
    # ---- rules data -------------------------------------------------------
    item_ids = set(ids)
    perk_ids = {p["id"] for p in perks}
    stats, _ = load_group("stats.json", "stat", "stats")
    for st in stats:
        for ref in st.get("related_perks", []):
            if ref not in perk_ids:
                report("refs", st["id"], f"unknown perk {ref}")
    load_group("challenges.json", "challenge", "challenges")
    recipes, _ = load_group("recipes.json", "recipe", "recipes")
    for rc in recipes:
        for ref in [rc.get("output_item_id")] + [i.get("item_id") for i in rc["ingredients"]]:
            if ref and ref not in item_ids:
                report("refs", rc["id"], f"unknown item {ref}")
    endings, _ = load_group("endings.json", "ending", "endings")
    for en in endings:
        for ref in en.get("condition_quests", []):
            if ref not in quest_ids:
                report("refs", en["id"], f"unknown quest {ref}")
    levels = json.load(open(engine / "levels.json"))
    for lv in levels:
        for err in validators["level"].iter_errors(lv):
            report("levels", lv.get("level"), err.message[:160])
    encounters, _ = load_group("encounters.json", "encounter", "encounters")
    weapon_ids = {r["id"] for r in items["weapons"]}
    armor_ids = {r["id"] for r in items["armor"]}
    for en in encounters:
        if en["place"] not in loc_ids:
            report("refs", en["id"], f"unknown place {en['place']}")
        for foe in en["foes"]:
            if foe["npc"] not in npc_ids:
                report("refs", en["id"], f"unknown npc {foe['npc']}")
            if foe["weapon"] not in weapon_ids:
                report("refs", en["id"], f"{foe['weapon']} is not a weapon")
            if foe.get("armor") and foe["armor"] not in armor_ids:
                report("refs", en["id"], f"{foe['armor']} is not armor")
        scripted = en.get("requires", []) + en.get("on_win", [])
        for ref in refs_in(scripted, "quest"):
            if ref not in quest_ids:
                report("refs", en["id"], f"names unknown quest {ref}")
        for ref in refs_in(scripted, "npc"):
            if ref not in npc_ids:
                report("refs", en["id"], f"names unknown npc {ref}")
    vendors, _ = load_group("vendors.json", "vendor", "vendors")
    stock_ids = {r["id"] for name in ("weapons", "armor", "ammo", "consumables")
                 for r in items[name]}
    for vendor in vendors:
        if vendor["place"] not in loc_ids:
            report("refs", vendor["id"], f"unknown place {vendor['place']}")
        if vendor["npc"] not in npc_ids:
            report("refs", vendor["id"], f"unknown npc {vendor['npc']}")
        lines = [line["item"] for line in vendor.get("stock", [])]
        for item in lines:
            if item not in stock_ids:
                report("refs", vendor["id"], f"{item} cannot be stocked")
        for item in {i for i in lines if lines.count(i) > 1}:
            report("refs", vendor["id"], f"{item} is listed twice")
    vendor_ids = {v["id"] for v in vendors}
    for d in dialogue:
        for t in d["topics"]:
            if t.get("shop") and t["shop"] not in vendor_ids:
                report("refs", d["id"], f"{t['key']} opens unknown vendor {t['shop']}")
    formulas = json.load(open(engine / "formulas.json"))
    for err in validators["formulas"].iter_errors(formulas):
        report("formulas", "formulas.json", err.message[:160])
    consts = formulas["constants"]
    fids = Counter(f["id"] for f in formulas["formulas"])
    for f in formulas["formulas"]:
        if fids[f["id"]] > 1:
            report("formulas", f["id"], "duplicate formula id")
        bad = check_expression(f["expression"], set(f["vars"]) | set(consts))
        if bad:
            report("formulas", f["id"], bad)
    by_id = {f["id"]: f for f in formulas["formulas"]}

    def calc(fid, **env):
        f = by_id[fid]
        return evaluate(f["expression"], {**consts, **env})
    try:
        tests = [
            ("max_hp", dict(endurance=5, level=1), 200),
            ("max_hp", dict(endurance=10, level=30), 445),
            ("action_points", dict(agility=5), 80),
            ("carry_weight", dict(strength=5), 200),
            ("skill_initial_speech", dict(charisma=5, luck=5), 15),
            ("barter_buy_multiplier", dict(barter=0), 1.55),
            ("barter_buy_multiplier", dict(barter=100), 1.10),
            ("barter_sell_multiplier", dict(barter=0), 0.45),
            ("barter_sell_multiplier", dict(barter=100), 0.90),
            ("damage_after_dr", dict(damage=40, dr=85), 6),
            ("damage_after_dt", dict(damage=40, damage_after_dr=6, dt=10), 8),
        ]
        for fid, env, want in tests:
            got = calc(fid, **env)
            if abs(got - want) > 1e-9:
                report("formulas", fid, f"expected {want} for {env}, got {got}")
        for lv in levels:
            if "xp_to_next" in lv and calc("xp_to_next_level", level=lv["level"]) != lv["xp_to_next"]:
                report("formulas", "xp_to_next_level", f"level {lv['level']} disagrees with levels.json")
    except Exception as exc:  # a broken formula must not crash the whole check
        report("formulas", "selftest", f"{type(exc).__name__}: {exc}")
    for i, n in npc_ids.items():
        if n > 1:
            report("ids", i, "duplicate npc id")
    for i, n in ids.items():
        if n > 1:
            report("ids", i, "duplicate item id")
    for i, n in quest_ids.items():
        if n > 1:
            report("ids", i, "duplicate quest id")

    families = {a["family"] for a in items["ammo"]}
    weapon_ids = {w["id"] for w in items["weapons"]}
    for w in items["weapons"]:
        if "ammo_family" in w and w["ammo_family"] not in families:
            report("refs", w["id"], f"unknown ammo_family {w['ammo_family']}")
    for m in items["weapon_mods"]:
        for wid in m.get("modifies", []):
            if wid not in weapon_ids:
                report("refs", m["id"], f"unknown weapon {wid}")
    stages_of = {x["id"]: {s["stage"] for s in x["stages"]} for x in quests}
    for q in quests:
        for key in ("previous", "leads_to", "related"):
            for t in q.get(key, []):
                if t not in quest_ids:
                    report("refs", q["id"], f"{key}: unknown quest {t}")
        for rule in q.get("rules", []):
            if "target_id" in rule and rule["target_id"] not in quest_ids:
                report("refs", q["id"], f"rule target {rule['target_id']} unknown")
        # Authored conditions and effects name quests, factions, items and npcs by id.
        authored = [rule.get(k, []) for rule in q.get("rules", []) for k in ("conditions", "effects")]
        authored += [stage.get("on_enter", []) for stage in q["stages"]]
        for node in (n for group in authored for n in walk_vocabulary(group)):
            for key, known in (("quest", quest_ids), ("faction", faction_ids), ("item", item_ids),
                               ("npc", npc_ids)):
                if key in node and node[key] not in known:
                    report("refs", q["id"], f"authored {node['type']}: unknown {key} {node[key]}")
            if node.get("type") == "set_stage" and node["stage"] not in stages_of.get(node["quest"], ()):
                report("refs", q["id"], f"authored set_stage: {node['quest']} has no stage {node['stage']}")

    total = sum(len(v) for v in items.values())
    print(f"checked {total} items, {len(quests)} quests, {len(npcs)} npcs, "
          f"{len(dialogue)} dialogue files, {len(locs)} locations, {len(perks)} perks, "
          f"{len(traits)} traits, {len(factions)} factions, {len(creatures)} creatures, "
          f"{len(stats)} stats, {len(recipes)} recipes, {len(endings)} endings, "
          f"{len(formulas['formulas'])} formulas")
    if errors:
        print("errors:", dict(errors))
        return 1
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
