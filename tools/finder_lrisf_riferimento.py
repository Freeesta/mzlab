"""Reference answers of the in-source-fragment classifier of mzFinder (`TP_Mine/py/tpmine/isf.py`) for its Rust port
(`crates/mzlab-feat/src/legacy/lrisf/`, test `crates/mzlab-feat/tests/legacy_lrisf.rs`): the inputs (the reports of `ionfamily.origin_report`
on the synthetic scene of `mzlab.demo.isf_series`, and variations of them) and the outputs of `isf.classify_report`.

    python3 tools/finder_lrisf_riferimento.py [--golden]

Writes tests/golden/finder_lrisf.json with --golden, else prints the number of cases. Synthetic inputs only (fixed seed). A case whose Python
raises is stored with `expect: null` (the Rust returns `None`).
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent))
sys.path.insert(0, str(AQUI.parent / "TP_Mine" / "py"))

from mzlab import demo, ionfamily as F  # noqa: E402
from types import SimpleNamespace  # noqa: E402

import numpy as np  # noqa: E402

from tpmine import chem, fragtree as FT, isf  # noqa: E402

OFF = demo.ISF_OFFSET
PMZ = demo.ISF_PARENT_MZ + OFF
FORMULA = "C14H13F4N3O2S"
IONS = [194.0, 152.0, 346.0, 365.0, 386.0, 380.0, 322.0, 280.0, 400.0, 300.0]


def run(rep, formula_p=None, ramp=None):
    try:
        return F.to_jsonable(isf.classify_report(F.to_jsonable(rep), formula_p, None, ramp))
    except Exception:  # noqa: BLE001 -- the engine catches it too
        return None


def case(name, rep, formula_p=None, ramp=None):
    rep = F.to_jsonable(rep)
    return {"name": name, "rep": rep, "formula_p": formula_p, "ramp": ramp, "expect": run(rep, formula_p, ramp)}


def scene_cases(series):
    out = []
    for k in (len(series), 3, 2, 1):
        sub = series[:k]
        for mz in IONS:
            if k < len(series) and mz not in (194.0, 346.0, 380.0, 365.0):
                continue
            rep = F.origin_report(sub, mz + OFF, PMZ, top=0)
            out.append(case("scene n=%d mz=%g" % (k, mz), rep, FORMULA if mz in (194.0, 152.0, 346.0) else None))
    rep = F.origin_report(series, 194.0 + OFF, PMZ, top=0)
    out.append(case("scene no formula", rep))
    out.append(case("scene incompatible formula", F.origin_report(series, 152.0 + OFF, PMZ, top=0), "CH4"))
    out.append(case("scene reference chosen", F.origin_report(series, 346.0 + OFF, PMZ, top=0, ref=2), FORMULA))
    return out


def mutated(base):
    out = []
    def mut(name, fn, formula_p=FORMULA, ramp=None):
        r = copy.deepcopy(base)
        fn(r)
        out.append(case(name, r, formula_p, ramp))
    mut("no profile fields", lambda r: r.update(profile={}))
    mut("profile null", lambda r: r.update(profile=None))
    mut("no peak of its own", lambda r: r["profile"].update(x_has_peak=False))
    mut("unreliable", lambda r: r["profile"].update(reliable=False))
    mut("fwhm none", lambda r: r["profile"].update(fwhm_ratio=None))
    mut("pearson_w only pearson", lambda r: (r["profile"].pop("pearson_w", None), r["profile"].update(pearson=0.95)))
    mut("sigma none raises", lambda r: r["profile"].update(apex_diff_sigma_s=None))
    mut("p_shift small", lambda r: r["profile"].update(p_shift=0.0001))
    mut("p_shift none", lambda r: r["profile"].update(p_shift=None))
    mut("ratio not ok", lambda r: r.update(ratio={"ok": False}))
    mut("ratio missing", lambda r: r.pop("ratio"))
    mut("no constancy", lambda r: r.pop("ratio_constancy", None))
    mut("constancy low p", lambda r: r.update(ratio_constancy={"p": 1e-6, "k": 5, "cv": 0.8}), None)
    mut("constancy p tiny + profile good", lambda r: (r["profile"].update(pearson_w=0.97), r.update(ratio_constancy={"p": 1e-15, "k": 4, "cv": 5.0})))
    mut("constancy cv only", lambda r: r.update(ratio_constancy={"p": None, "k": 2, "cv": 0.2}))
    mut("no kinetics", lambda r: r.pop("kinetics", None))
    mut("kinetics null rho", lambda r: r.update(kinetics={"spearman_parent": None}))
    mut("warnings flat top", lambda r: r.update(warnings=[{"key": "of.warn.flatTop", "params": {"n": 4}}]))
    mut("warnings coincidence", lambda r: r.update(warnings=[{"key": "of.warn.massCoincidence", "params": {"mz": "345.3, 347.3"}}, {"key": "of.warn.weakIon", "params": {}}]))
    mut("no warnings key", lambda r: r.pop("warnings"))
    mut("ms2 present", lambda r: r.update(ms2={"x_in_p": True, "modified_cosine": 0.8}))
    mut("ms2 absent", lambda r: r.update(ms2={"x_in_p": False, "modified_cosine": None}))
    mut("ms2 membership only", lambda r: r.update(ms2={"x_in_p": True}))
    mut("ms2 empty", lambda r: r.update(ms2={}))
    mut("roles none", lambda r: r.update(roles=[]))
    mut("roles two losses", lambda r: r.update(roles=[{"role": "fragment_candidate", "label": "H2O", "formula_ok": True}, {"role": "fragment_candidate", "label": "CO2", "formula_ok": True},
                                                      {"role": "fragment_candidate", "label": "C2H4", "formula_ok": True}, {"role": "fragment_candidate", "label": "HF", "formula_ok": True}]))
    mut("roles formula not ok", lambda r: r.update(roles=[{"role": "fragment_candidate", "label": "CH4O", "formula_ok": False}]))
    mut("roles formula not ok no parent formula", lambda r: r.update(roles=[{"role": "fragment_candidate", "label": "CH4O", "formula_ok": False}]), None)
    mut("roles formula_ok null", lambda r: r.update(roles=[{"role": "fragment_candidate", "label": "H2O", "formula_ok": None}]))
    mut("neighbors isotope", lambda r: r.update(neighbors=[{"delta": 1, "mz": r["mz"] - 1.0, "area_ratio": 0.4, "pearson": 0.97}]))
    mut("neighbors weak", lambda r: r.update(neighbors=[{"delta": 2, "mz": r["mz"] - 2.0, "area_ratio": 3.0, "pearson": 0.99}, {"delta": 1, "mz": r["mz"] - 1.0, "area_ratio": 0.2, "pearson": 0.95}]))
    mut("neighbors nan ratio", lambda r: r.update(neighbors=[{"delta": 1, "mz": r["mz"] - 1.0, "area_ratio": None, "pearson": 0.97}]))
    mut("heavier isotope role", lambda r: r.update(mz=r["parent_mz"] + 1.0, roles=[{"role": "isotope", "label": "M+1"}]), None)
    mut("heavier adduct+isotope roles", lambda r: r.update(mz=r["parent_mz"] + 22.0, roles=[{"role": "adduct", "label": "Na"}, {"role": "isotope", "label": "M+1"}]), None)
    mut("heavier dimer", lambda r: r.update(mz=r["parent_mz"] + 360.0, roles=[{"role": "dimer", "label": "2M"}]), None)
    mut("heavier unexplained", lambda r: r.update(mz=r["parent_mz"] + 5.0, roles=[]), None)
    mut("equal mass", lambda r: r.update(mz=r["parent_mz"]), None)
    mut("exactly -0.5", lambda r: r.update(mz=r["parent_mz"] - 0.5), FORMULA)
    ramp_up = {"dp": [20, 40, 60, 80, 100], "f": [1, 5, 20, 60, 90], "p": [99, 95, 80, 40, 10]}
    mut("ramp rising", lambda r: None, FORMULA, ramp_up)
    mut("ramp flat falling", lambda r: None, FORMULA, {"dp": [100, 80, 60, 40], "f": [10, 10, 11, 9], "p": [90, 90, 89, 91]})
    mut("ramp two points", lambda r: None, FORMULA, {"dp": [20, 40], "f": [1, 5], "p": [99, 95]})
    mut("ramp zeros", lambda r: None, FORMULA, {"dp": [20, 40, 60, 80, 100], "f": [0, 5, 0, 60, 90], "p": [0, 95, 0, 40, 10]})
    mut("ramp with ms2", lambda r: r.update(ms2={"x_in_p": True, "modified_cosine": 0.3}), FORMULA, ramp_up)
    return out


# ---------------------------------------------------------------------------------------------------------------- supposed tree and comparison
SMILES = ["CC(C)NC(=O)COc1ccccc1", "CCN(CC)C(=O)c1ccccc1", "OC(=O)c1ccccc1O", "CC(=O)Nc1ccc(O)cc1", "NC(=O)c1ccccc1"]


def _ex(mz_x, ms2, neutral, adduct, smiles, neutral_text=None):
    par = {"smiles": smiles, "neutral": neutral_text if neutral_text is not None else (chem.fmt(neutral) if neutral else "")}
    return SimpleNamespace(entries=[{"mz_x": mz_x}], ms2=lambda e, top=12, min_rel=0.05: ms2, parent=par, neutral=neutral, adduct=adduct)


def tree_case(name, mz_x, ms2, neutral, smiles=None, adduct="[M+H]+", neutral_text=None):
    ex = _ex(mz_x, ms2, neutral, adduct, smiles, neutral_text)
    ion = chem.fmt(chem.add(neutral, {"H": 1})) if neutral is not None and adduct == "[M+H]+" else None
    smi = (smiles or "").strip() or None
    if smi is None and neutral_text and not neutral_text.strip().replace(" ", "").isalnum():
        smi = neutral_text.strip()
    inp = {"mz_x": mz_x, "scans": ms2["scans"], "precursor": ms2["precursor"], "mz": [f["mz"] for f in ms2["fragments"]], "rel": [f["rel"] for f in ms2["fragments"]],
           "ion": ion, "smiles": smi, "smiles_in": smiles, "neutral_text": neutral_text}
    return {"name": name, "in": F.to_jsonable(inp), "out": F.to_jsonable(FT.lr(ex))}


def tree_cases(rng):
    out = []
    forms = [{"C": 11, "H": 15, "N": 1, "O": 2}, {"C": 11, "H": 15, "N": 1, "O": 1}, {"C": 7, "H": 6, "O": 3}, {"C": 8, "H": 9, "N": 1, "O": 2}, {"C": 7, "H": 7, "N": 1, "O": 1}]
    for i, (f, sm) in enumerate(zip(forms, SMILES)):
        m = float(sum({"C": 12.0, "H": 1.007825, "N": 14.003074, "O": 15.994915}[e] * n for e, n in f.items()) + 1.007276)
        for variant in range(4):
            mzs = sorted({round(m - d + 0.3 + float(rng.normal(0, 0.04)), 2) for d in rng.uniform(2, m - 30, 8 + variant)}, reverse=True)
            rel = [float(round(x, 1)) for x in rng.uniform(1, 100, len(mzs))]
            rel[int(rng.integers(len(rel)))] = 100.0
            frags = [{"mz": a, "rel": b} for a, b in zip(mzs, rel)]
            ms2 = {"scans": 5 + variant, "precursor": round(m + 0.3, 2) if variant != 3 else None, "fragments": frags}
            kw = [dict(smiles=sm), dict(smiles=None), dict(smiles="  "), dict(smiles=None, neutral_text=sm)][variant]
            out.append(tree_case("tree %d.%d" % (i, variant), round(m + 0.3, 2), ms2, f if variant != 1 else None, **kw))
    # losses of the table at known masses (H2O, NH3, CO2) around the first parent
    m = 194.1176 + 0.3
    fr = [{"mz": round(m - 18.0106, 2), "rel": 100.0}, {"mz": round(m - 17.0265, 2), "rel": 40.0}, {"mz": round(m - 43.9898, 2), "rel": 12.0}, {"mz": round(m - 0.5, 2), "rel": 80.0}]
    out.append(tree_case("losses of the table, no formula", m, {"scans": 3, "precursor": m, "fragments": fr}, None))
    out.append(tree_case("other adduct", m, {"scans": 3, "precursor": m, "fragments": fr}, forms[0], adduct="[M+Na]+"))
    out.append(tree_case("no MS2", m, {"scans": 0, "precursor": None, "fragments": []}, forms[0], smiles=SMILES[0]))
    out.append(tree_case("scans but no fragments", m, {"scans": 4, "precursor": m, "fragments": []}, forms[0]))
    out.append(tree_case("precursor zero uses mz_x", m, {"scans": 2, "precursor": 0.0, "fragments": fr}, forms[0], smiles=SMILES[0]))
    return out


def compare_cases(trees, rng):
    out = []
    for t in trees:
        tree = t["out"]
        if not tree.get("ok"):
            continue
        nodes = [n for n in tree["nodes"] if n["kind"] == "frag"]
        for k, dm in enumerate((None, 15.9949, -14.0157)):
            pick = [n["mz"] + float(rng.normal(0, 0.15)) for n in nodes if rng.random() < 0.5]
            pick += [n["mz"] + (dm or 0.0) + float(rng.normal(0, 0.15)) for n in nodes if rng.random() < 0.4]
            if k == 2:
                pick = []
            frs = [{"mz": round(float(x), 2), "rel": 50.0} for x in pick]
            res = FT.compare_lr(tree, {"fragments": frs}, dm)
            out.append({"name": "%s dm=%s" % (t["name"], dm), "tree": tree, "tp_mz": [f["mz"] for f in frs], "dm": dm, "out": F.to_jsonable(res)})
    return out


def main():
    ser = demo.isf_series(times=(5, 10, 15, 30, 45, 60), seed=777)
    series = [{"label": "t%g" % t, "time": float(t), "table": tb, "key": "tisf_%g" % t} for t, tb, _ in ser]
    base = F.origin_report(series, 346.0 + OFF, PMZ, top=0)
    cases = scene_cases(series) + mutated(base)
    rng = np.random.default_rng(2026)
    trees = tree_cases(rng)
    compares = compare_cases(trees, rng)
    if "--golden" in sys.argv:
        p = AQUI.parent / "tests" / "golden" / "finder_lrisf.json"
        p.write_text(json.dumps({"cases": cases, "tree": trees, "compare": compares}, allow_nan=False, separators=(",", ":")), encoding="utf-8")
        print(p, p.stat().st_size, "bytes")
    print(len(trees), "trees,", len(compares), "comparisons;", len(cases), "cases,", sum(c["expect"] is None for c in cases), "raising")


if __name__ == "__main__":
    main()
