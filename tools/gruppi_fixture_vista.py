#!/usr/bin/env python3
"""Writes the synthetic `gruppi/1` kinetic fixture the Compare-conditions view is tried on (TP_Mine/tests/gruppi_fixture_kin.json: 3 conditions x 5 times;
TP_Mine/tests/gruppi_fixture.json is the expected output of the Rust module, copied from crates/mzlab-feat/tests/data/gruppi_fixture.json). Test data only: every number is made up here and follows the contract of
docs/studi/TABUI_GRUPPI_ESPERIMENTI.md section 3 (presence, ratio to the reference, sets, shape, prev), so the view has something to draw
before the Rust module is wired in. Run: python3 tools/gruppi_fixture_vista.py"""
import json, itertools
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "TP_Mine" / "tests"
PARENT = "Demo-parent"


def shape_of(areas):
    """shape of a kinetic (>= 3 treated times), as the contract defines it"""
    if len(areas) == 1: return "single"
    if len(areas) < 3: return None
    mx, mn = max(areas), min(x for x in areas if x > 0) if any(x > 0 for x in areas) else 0
    i = areas.index(mx)
    if mn and mx / mn < 1.5: return "flat"
    if i == len(areas) - 1: return "rising"
    if i == 0: return "decay"
    return "bell" if min(areas[i + 1:]) <= mx * (1 - 0.3) else "rising"


def t_half_rise(ts, areas):
    mx = max(areas)
    if mx <= 0: return None
    for k, a in enumerate(areas):
        if a >= 0.5 * mx:
            if k == 0: return ts[0]
            a0, a1 = areas[k - 1], a
            return round(ts[k - 1] + (0.5 * mx - a0) / (a1 - a0) * (ts[k] - ts[k - 1]), 2)
    return None


def prev(present, area, absent_t0, absent_blank, in_std, ms2):
    crit = [absent_t0, absent_blank]
    if "fail" in crit or in_std == "yes": v = "Prev. unlikely"
    elif present and "na" not in crit and ms2 == "linked" and in_std == "no": v = "Prev. likely"
    else: v = "Prev. possible"
    return {"absent_t0": absent_t0, "absent_blank": absent_blank, "in_parent_std": in_std, "above_min": "pass" if area else "na", "ms2": ms2, "verdict": v}


def build(conds, times, rows, ref, replicates, parents):
    """conds: ids in order; times: {cond: [t...]}; rows: dicts with per-condition area lists (None = absent)"""
    out_rows, sets = [], {}
    for rid, r in enumerate(rows):
        cond, pres = [], []
        for c in conds:
            ts = times[c]; ar = r["a"].get(c) or [0.0] * len(ts)
            present = any(a > 0 for a in ar)
            kin = [{"t": t, "area": a if a else None, "sd": (round(a * 0.08, 1) if a and replicates[c] > 1 else None), "n": replicates[c]} for t, a in zip(ts, ar)]
            cond.append({"id": c, "present": present, "n_det": replicates[c] if present else 0, "n_rep": replicates[c], "area": ar[-1] if present else None,
                         "area_sd": round(ar[-1] * 0.08, 1) if present and replicates[c] > 1 else None, "cv": 0.08 if present and replicates[c] > 1 else None,
                         "ratio_ref": None, "shape": shape_of(ar) if present else None, "t_max": ts[ar.index(max(ar))] if present else None,
                         "t_half_rise": t_half_rise(ts, ar) if present and len(ts) > 2 else None, "rate_rel": None, "kin": kin,
                         "prev": prev(present, ar[-1], r["t0"], r["blank"], r["std"], r["ms2"])})
            if present: pres.append(c)
        if ref:
            rc = next(x for x in cond if x["id"] == ref)
            for x in cond:
                x["ratio_ref"] = (round(x["area"] / rc["area"], 3) if x["present"] and rc["present"] else 0 if not x["present"] and rc["present"] else None)
                if len(times[x["id"]]) > 2 and rc["t_half_rise"] and x["t_half_rise"]: x["rate_rel"] = round(rc["t_half_rise"] / x["t_half_rise"], 2)
        out_rows.append({"row": rid, "tp_name": r["name"], "mz": r["mz"], "polarity": r.get("pol", "+"), "rt": r["rt"], "neutral_formula": None, "ppm": None, "other_polarity": None,
                         "present_in": pres, "only_in": pres[0] if len(pres) == 1 else None, "in_all": len(pres) == len(conds), "cond": cond})
        sets.setdefault(tuple(pres), []).append(rid)
    return {"format": "gruppi", "version": 1, "reference": ref, "norm": "raw", "parent_name": PARENT,
            "conditions": [{"id": c, "factors": {}, "files": [], "times": times[c], "replicates": replicates[c],
                            "controls": {"t0": [0], "blank": [1], "std": [2], "dark": []}, "parent": parents[c]} for c in conds],
            "rows": out_rows,
            "sets": [{"conditions": [c for c in conds if c in k], "rows": v} for k, v in sorted(sets.items(), key=lambda kv: (-len(kv[1]), kv[0])) if k],
            "warnings": []}


def kinetic():
    conds = ["Sun", "Lamp", "Dark"]
    ts = [0, 15, 30, 60, 120]
    times = {c: ts for c in conds}
    reps = {c: 2 for c in conds}
    parents = {"Sun": {"k": 0.03, "half_life": 23.1, "remaining": 0.03}, "Lamp": {"k": 0.012, "half_life": 57.8, "remaining": 0.24}, "Dark": {"k": None, "half_life": None, "remaining": 0.97}}
    nz = lambda v: [x if x else None for x in v]
    def A(sun, lamp, dark): return {"Sun": sun, "Lamp": lamp, "Dark": dark}
    rows = [
        dict(name="TP 305", mz=305.3, rt=7.1, t0="pass", blank="pass", std="no", ms2="linked", a=A([1e4, 9e5, 2.4e6, 4.5e6, 6.0e6], [1e4, 3e5, 8e5, 1.6e6, 3.0e6], [0, 0, 0, 0, 0])),
        dict(name="TP 229", mz=229.1, rt=5.2, t0="pass", blank="pass", std="no", ms2="linked", a=A([1e4, 6e5, 1.4e6, 7e5, 1.0e5], [1e4, 2e5, 5e5, 9e5, 6e5], [0, 0, 0, 0, 0])),
        dict(name="TP 224", mz=224.2, rt=4.4, t0="pass", blank="pass", std="no", ms2="none", a=A([2.0e6, 1.0e6, 4e5, 1e5, 2e4], [2.0e6, 1.6e6, 1.2e6, 6e5, 2e5], [2.0e6, 2.0e6, 1.9e6, 2.0e6, 2.0e6])),
        dict(name="TP 391", mz=391.3, rt=9.8, t0="fail", blank="pass", std="yes", ms2="na", a=A([3e5, 3.1e5, 3.0e5, 3.2e5, 3.1e5], [3e5, 3e5, 3e5, 3.1e5, 3e5], [3e5, 3e5, 3.1e5, 3e5, 3e5])),
    ]
    return build(conds, times, rows, "Sun", reps, parents)


if __name__ == "__main__":
    for name, fx in (("gruppi_fixture_kin.json", kinetic()),):
        (OUT / name).write_text(json.dumps(fx, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
        print(name, len(fx["rows"]), "rows", len(fx["conditions"]), "conditions", len(fx["sets"]), "sets")
