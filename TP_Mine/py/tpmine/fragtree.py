# TPMINE-PRIVATE
"""Fragmentation tree of the parent in one format for both worlds (the drawing is js/23_tpmine_albero.js).

HR: the MSn tree (hr.msn) with the strong peaks of each spectrum as leaves; every node has its formula, the neutral loss from its parent and up to three
proposed substructures (SMILES with a weight, from hr.molgraph). LR: a "supposed" two-level tree built from the MS2 of the parent only: each fragment
above 5 % gets every nominal sub-formula of the parent ion within +-0.5 Da and the common neutral losses, all kept as alternatives. Nothing here is a
verdict: the LR tree says "supposed: unit resolution".

Node: {id, parent, level (MS level of the spectrum), kind ('ion' isolated precursor | 'frag' peak), mz, formula, rel (% of the base peak of the parent
spectrum), loss, loss_mass, ppm, n_scans, ce, ghost, smiles [{smiles, score}], alternatives [{formula, loss, err, source}]}."""
from __future__ import annotations

import numpy as np

from mzlab.chem import elements as E

from . import chem
from .hr import formula as F
from .hr import molgraph as MG
from .hr.iimn import LOSSES

MIN_REL = 5.0                   # % of the base peak: weaker peaks are not drawn
MAX_ALT = 6
MAX_SMILES = 3
LR_TOL = 0.5                    # Da, unit resolution
LR_NOTE = "supposed: unit resolution"


# ---------------------------------------------------------------------------------------------------------------------------- SMILES of a fragment
def mask_smiles(mol: MG.Mol, mask: int) -> str:
    """SMILES of the substructure `mask` (atom bitmask of `mol`). Every atom is written in brackets with the hydrogens it carries, so the cut atoms stay open
    valences; aromatic rings are lowercase only when the whole ring is inside the mask, other aromatic bonds are written as alternating single/double."""
    inn = [i for i in range(mol.n) if (int(mask) >> i) & 1]
    ins = set(inn)
    bond = {}
    for k, (a, b, o) in enumerate(mol.bonds):
        if a in ins and b in ins:
            bond[k] = (a, b, o)
    arom: set = set()
    for ring in mol.rings():
        if all(a in ins for a in ring):
            ks = [k for k, (a, b, o) in bond.items() if a in ring and b in ring]
            if sum(1 for k in ks if bond[k][2] == 1.5) >= len(ring):
                arom.update(ring)
    order, matched = {}, set()
    for k, (a, b, o) in bond.items():
        if o == 1.5 and a in arom and b in arom:
            order[k] = 1.5
        elif o == 1.5:
            if a not in matched and b not in matched:
                matched.update((a, b))
                order[k] = 2.0
            else:
                order[k] = 1.0
        else:
            order[k] = o
    adj = {i: [] for i in inn}
    for k, (a, b, _) in bond.items():
        adj[a].append((b, k))
        adj[b].append((a, k))

    def tok(i):
        el = mol.atoms[i]
        h = int(mol.H[i])
        return "[" + (el.lower() if i in arom else el) + ("H" if h == 1 else f"H{h}" if h > 1 else "") + "]"

    def sym(k):
        a, b, _ = bond[k]
        o = order[k]
        if o == 2.0:
            return "="
        if o == 3.0:
            return "#"
        return "-" if (o == 1.0 and a in arom and b in arom) else ""

    seen: set = set()
    kids: dict = {i: [] for i in inn}
    rings: dict = {i: [] for i in inn}
    closed: set = set()
    nring = [0]

    def dfs(i, frm):
        seen.add(i)
        for j, k in adj[i]:
            if k == frm:
                continue
            if j not in seen:
                kids[i].append((j, k))
                dfs(j, k)
            elif k not in closed and not any(k == kk for _, kk in kids[j]):
                closed.add(k)
                nring[0] += 1
                n = nring[0]
                rings[j].append((n, k, True))
                rings[i].append((n, k, False))

    def gen(i):
        s = tok(i)
        for n, k, opening in rings[i]:
            s += (sym(k) if opening else "") + (str(n) if n < 10 else f"%{n}")
        parts = [sym(k) + gen(j) for j, k in kids[i]]
        for p in parts[:-1]:
            s += "(" + p + ")"
        return s + (parts[-1] if parts else "")

    out = []
    for i in inn:
        if i not in seen:
            dfs(i, None)
            out.append(gen(i))
    return ".".join(out)


def _top_smiles(subs, idx, w) -> list:
    """Up to MAX_SMILES different substructures (largest weight first), with their weight."""
    out, got = [], set()
    for j in np.argsort(-np.asarray(w), kind="stable"):
        s = mask_smiles(subs.mol, int(subs.masks[int(idx[j])]))
        if s in got:
            continue
        got.add(s)
        out.append({"smiles": s, "score": round(float(w[j]), 3)})
        if len(out) == MAX_SMILES:
            break
    return out


def _loss_str(par_vec, vec, els):
    d = np.asarray(par_vec) - np.asarray(vec)
    if (d < 0).any() or not d.any():
        return None
    return F.fmt(d, els)


def _loss_mass(s):
    return None if not s else round(float(E.mass(E.parse_formula(s))), 4)


# ---------------------------------------------------------------------------------------------------------------------------------------------- HR
def hr(ex) -> dict:
    """Tree of an `ExperimentHR`: the MSn tree when the experiment has an MSn file, else the DDA MS2 of the parent (exact formulas, no 'supposed')."""
    if ex.tree is not None:
        return _hr_msn(ex)
    p2 = getattr(ex, "parent_ms2", None)
    if p2 is not None and len(p2["mz"]):
        return _two_level(ex.ion_vec, ex.els, ex.parent_mz, p2["mz"], p2["rel"], ex.smiles, ppm=5.0, abs_da=0.002, supposed=False, mode="hr", n_scans=p2.get("n_scans"))
    return {"ok": False, "mode": "hr", "note": "no MSn file and no MS2 of the parent: no fragmentation tree", "nodes": []}


def _hr_msn(ex) -> dict:
    tree, els = ex.tree, ex.tree.els
    by = {n["id"]: n for n in tree.nodes}
    vec = {i: F.vec(n["formula"], els) for i, n in by.items() if n["formula"]}
    kids: dict = {}
    for n in tree.nodes:
        kids.setdefault(n["parent"], []).append(n)
    nodes = []
    root = tree.nodes[0]
    drop = None
    if len(kids.get(root["id"], [])) == 1 and kids[root["id"]][0]["formula"] == root["formula"]:
        drop = root["id"]                   # MS1 and its only MS2 node are the same ion: one node
    smiles_root = [{"smiles": ex.smiles, "score": 1.0}] if ex.smiles else []

    def parent_rel(n):
        par = by.get(n["parent"])
        if n["parent"] in (None, drop):
            return 100.0
        if par is None or n["prec_mz"] is None:
            return None
        near = [p for p in par["peaks"] if abs(p["mz"] - n["prec_mz"]) <= 0.5]
        return round(max(p["rel"] for p in near), 1) if near else None

    for n in tree.nodes:
        if n["id"] == drop:
            continue
        pid = n["parent"] if n["parent"] != drop else None
        sm = []
        if n["id"] in tree.cand:
            idx, w = tree.cand[n["id"]]
            if len(idx):
                sm = _top_smiles(tree.subs, idx, w)
        if n["id"] == "MS1" or (drop and n["parent"] == drop):
            sm = sm or smiles_root
        loss = None
        if n["parent"] in vec and n["id"] in vec:
            loss = _loss_str(vec[n["parent"]], vec[n["id"]], els)
        nodes.append({"id": n["id"], "parent": pid, "level": n["level"], "kind": "ion", "mz": None if n["prec_mz"] is None else round(n["prec_mz"], 4),
                      "formula": n["formula"], "rel": parent_rel(n) if n["id"] != "MS1" else 100.0, "loss": loss, "loss_mass": _loss_mass(loss),
                      "ppm": None if n["ppm"] is None else round(n["ppm"], 1), "n_scans": n["n_scans"], "ce": n["ce"], "ghost": bool(n["ghost"]),
                      "smiles": sm, "alternatives": []})
    # the strong peaks of every spectrum that are not themselves an isolated precursor
    for n in tree.nodes:
        if not n["peaks"]:
            continue
        isolated = kids.get(n["id"], [])
        host = n["id"]
        for j, p in enumerate(n["peaks"]):
            if p["rel"] < MIN_REL or not p["formula"]:
                continue
            if any((c["formula"] == p["formula"]) or (c["prec_mz"] is not None and abs(c["prec_mz"] - p["mz"]) <= 0.5) for c in isolated):
                continue
            if n["formula"] and p["formula"] == n["formula"]:
                continue
            sm = []
            idx, w = tree.peak_cand.get((n["id"], j), (np.zeros(0, int), np.zeros(0)))
            if len(idx):
                sm = _top_smiles(tree.subs, idx, w)
            loss = _loss_str(vec[n["id"]], F.vec(p["formula"], els), els) if n["id"] in vec else None
            nodes.append({"id": f"{n['id']}#{j}", "parent": host, "level": n["level"], "kind": "frag", "mz": round(p["mz"], 4), "formula": p["formula"],
                          "rel": round(p["rel"], 1), "loss": loss, "loss_mass": _loss_mass(loss), "ppm": round(p["ppm"], 1), "n_scans": p["n_scans"], "ce": n["ce"],
                          "ghost": False, "smiles": sm, "alternatives": []})
    return {"ok": True, "mode": "hr", "supposed": False, "note": "", "nodes": nodes}


# ---------------------------------------------------------------------------------------------------------------------------------------------- LR
def lr(ex) -> dict:
    """Supposed tree of a low-resolution experiment from the MS2 of the parent."""
    par = ex.entries[0]
    ms2 = ex.ms2(par, top=12, min_rel=MIN_REL / 100)
    if not ms2["scans"] or not ms2["fragments"]:
        return {"ok": False, "mode": "lr", "note": "no MS2 of the parent in the files", "nodes": []}
    smiles = (ex.parent.get("smiles") or "").strip() or None
    if smiles is None and ex.parent.get("neutral") and not ex.parent["neutral"].strip().replace(" ", "").isalnum():
        smiles = ex.parent["neutral"].strip()
    mz0 = float(ms2["precursor"] or par["mz_x"])
    mzs = np.array([f["mz"] for f in ms2["fragments"]])
    rel = np.array([f["rel"] for f in ms2["fragments"]])
    ion_vec = els = None
    if ex.neutral is not None and ex.adduct == "[M+H]+":
        ion = chem.add(ex.neutral, {"H": 1})
        els = F.element_order(ion)
        ion_vec = F.vec(ion, els)
    return _two_level(ion_vec, els, mz0, mzs, rel, smiles, ppm=0.0, abs_da=LR_TOL, supposed=True, mode="lr", n_scans=ms2["scans"])


def _two_level(ion_vec, els, mz0, mzs, rel, smiles, *, ppm, abs_da, supposed, mode, n_scans=None) -> dict:
    mzs, rel = np.asarray(mzs, float), np.asarray(rel, float)
    top = rel.max() if len(rel) else 1.0
    keep = [j for j in np.argsort(-rel, kind="stable") if rel[j] * 100 / top >= MIN_REL and mzs[j] < mz0 - 1.0][:12]
    space = F.FormulaSpace(ion_vec, els) if ion_vec is not None else None
    subs = None
    if smiles:
        try:
            subs = MG.substructures(MG.parse_smiles(smiles))
        except Exception:      # noqa: BLE001 - too big or not parsable: the tree is drawn without structures
            subs = None
    commons = {k for k in LOSSES}
    root = {"id": "P", "parent": None, "level": 1, "kind": "ion", "mz": round(mz0, 4), "formula": F.fmt(ion_vec, els) if ion_vec is not None else None, "rel": 100.0,
            "loss": None, "loss_mass": None, "ppm": None, "n_scans": n_scans, "ce": None, "ghost": False,
            "smiles": [{"smiles": smiles, "score": 1.0}] if smiles else [], "alternatives": []}
    nodes = [root]
    for j in keep:
        m = float(mzs[j])
        alts = []
        if space is not None:
            for c in space.candidates(m, ppm, abs_da)[:80]:
                v = space.G[int(c)]
                ls = _loss_str(ion_vec, v, els)
                if ls is None:
                    continue
                err = float(space.mass[int(c)] - m)
                alts.append({"formula": F.fmt(v, els), "loss": ls, "err": round(err, 4), "source": "loss" if ls in commons else "subformula"})
            alts.sort(key=lambda a: (a["source"] != "loss", abs(a["err"]), len(a["loss"])))
        else:
            for name, lm in LOSSES.items():
                if abs((mz0 - m) - lm) <= abs_da:
                    alts.append({"formula": None, "loss": name, "err": round(mz0 - lm - m, 4), "source": "loss"})
            alts.sort(key=lambda a: abs(a["err"]))
        alts = alts[:MAX_ALT]
        best = alts[0] if alts else None
        sm = []
        if subs is not None and best and best["formula"]:
            idx, w = subs.candidates(F.vec(best["formula"], els))
            if len(idx):
                sm = _top_smiles(subs, idx, w)
        nodes.append({"id": f"F{len(nodes)}", "parent": "P", "level": 2, "kind": "frag", "mz": round(m, 4), "formula": best["formula"] if best else None,
                      "rel": round(float(rel[j] * 100 / top), 1), "loss": best["loss"] if best else None, "loss_mass": _loss_mass(best["loss"]) if best else round(mz0 - m, 2),
                      "ppm": None, "n_scans": n_scans, "ce": None, "ghost": False, "smiles": sm, "alternatives": alts})
    return {"ok": True, "mode": mode, "supposed": supposed, "note": LR_NOTE if supposed else "", "nodes": nodes}


# ---------------------------------------------------------------------------------------------------------------------------------------- comparison
def compare_hr(tree: dict, localization: dict | None) -> dict:
    """Marks of the parent tree nodes found in a product, from its localisation evidence (hr.localize): 'same' (the fragment of the parent, unshifted) or
    'shifted' (the parent fragment plus the change)."""
    if not localization or not localization.get("evidence"):
        return {"ok": False, "note": "no fragment of the product with a known relation to the parent", "marks": {}}
    same = {e["library"] for e in localization["evidence"] if e["kind"] == "U"}
    shifted = {e["library"] for e in localization["evidence"] if e["kind"] == "S"}
    sign, delta = localization.get("delta_sign", "+"), localization.get("delta", "")
    marks = {}
    for n in tree["nodes"]:
        f = n.get("formula")
        if f in same:
            marks[n["id"]] = {"state": "same"}
        elif f in shifted:
            marks[n["id"]] = {"state": "shifted", "delta": sign + delta}
    return _tally(marks, tree)


def compare_lr(tree: dict, tp_ms2: dict, dm: float | None) -> dict:
    """Same for unit resolution: a fragment of the product within 0.5 Da of the parent node is 'same', within 0.5 Da of the node plus the mass change 'shifted'."""
    fr = tp_ms2.get("fragments") or []
    if not fr:
        return {"ok": False, "note": "no MS2 of the product", "marks": {}}
    marks = {}
    for n in tree["nodes"]:
        if n["kind"] != "frag" or n["mz"] is None:
            continue
        if any(abs(f["mz"] - n["mz"]) <= LR_TOL for f in fr):
            marks[n["id"]] = {"state": "same"}
        elif dm is not None and any(abs(f["mz"] - (n["mz"] + dm)) <= LR_TOL for f in fr):
            marks[n["id"]] = {"state": "shifted", "delta": f"{dm:+.2f}"}
    return _tally(marks, tree)


def _tally(marks, tree):
    n = sum(1 for x in tree["nodes"] if x["kind"] == "frag" or x["id"] != "MS1")
    return {"ok": True, "marks": marks, "n_same": sum(1 for m in marks.values() if m["state"] == "same"),
            "n_shifted": sum(1 for m in marks.values() if m["state"] == "shifted"), "n_nodes": n, "note": ""}
