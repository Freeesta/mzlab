# TPMINE-PRIVATE
"""MSn tree of the parent from a direct-infusion MSn file (WP3): nodes = fragmentation paths, consensus spectra, exact precursor masses taken
from the parent node, formulas under the hierarchical sub-formula rule, atom sets constrained by the tree. numpy only.

Facts the code relies on (measured, see the project notes): the whole path is only in the filter string; precursor masses in the file are
nominal; the nearest generation of a block often has no spectrum of its own; spectra of a node contain peaks heavier than the precursor and
peaks of earlier stages (contamination), and artefacts without any formula; calibration drifts by a few ppm."""
from __future__ import annotations

import time
from collections import defaultdict

import numpy as np

from mzlab.chem import elements as E

from . import formula as F
from . import molgraph as MG

MIN_PEAKS = 3                 # a node with fewer clean peaks is a ghost
HEAVY = 1.5                   # peaks above precursor + HEAVY are not products of it


def node_key(path) -> tuple:
    """(nominal m/z, activation, collision energy) per stage."""
    return tuple((int(round(mz)), act, ce) for mz, act, ce in path)


def node_id(key) -> str:
    return ">".join(f"{mz}@{act.lower()}{ce:g}" for mz, act, ce in key) if key else "MS1"


def consensus(run, idx, ppm: float = 5.0, min_frac: float = 0.5):
    """Consensus centroid spectrum of the scans idx: each scan normalised to its maximum, peaks within `ppm` merged (sort, diff, cumsum of the
    breaks), kept when present in at least `min_frac` of the scans, mean intensity. Returns (mz, intensity, n_scans_with_the_peak)."""
    from .spectra import consensus_arrays
    reads = [run.read(i) for i in idx]
    return consensus_arrays([r[0] for r in reads], [r[1] for r in reads], ppm, min_frac, 0.0, n_total=len(idx))


class MsnTree:
    """The result: `nodes` (list of dicts, JSON-ready except the numpy entries kept in `_arr`), the formula space, calibration and atom sets."""

    def __init__(self):
        self.nodes: list[dict] = []
        self.by_id: dict = {}
        self.cal = F.Calibration()
        self.els: list[str] = []
        self.space = None
        self.subs = None
        self.cand: dict = {}            # node id -> (indices into subs, weights)
        self.peak_cand: dict = {}       # (node id, peak index) -> (indices, weights)
        self.timing: dict = {}
        self.warnings: list[str] = []

    def node(self, nid: str) -> dict:
        return self.by_id[nid]

    def formulas(self) -> dict:
        """node id -> ion formula of its precursor."""
        return {n["id"]: n["formula"] for n in self.nodes if n["formula"]}

    def summary(self) -> list[dict]:
        """JSON-ready nodes (without the numpy arrays)."""
        return [{k: v for k, v in n.items() if not k.startswith("_")} for n in self.nodes]


def build_tree(run, ion, smiles: str | None = None, *, prec_ppm: float = 5.0, frag_ppm: float = 5.0, frag_abs: float = 0.002,
               calibrate: str = "auto", progress=None, on_node=None) -> MsnTree:
    """MSn tree of the ion `ion` (ion formula, dict or string, e.g. the [M+H]+ of the parent) from the levels >= 2 of `run`.

    smiles (optional, the neutral parent): adds the atom sets of every node and peak, constrained by the tree."""
    t0 = time.perf_counter()
    tree = MsnTree()
    ion_f = E.parse_formula(ion) if isinstance(ion, str) else dict(ion)
    els = F.element_order(ion_f)
    top = F.vec(ion_f, els)
    tree.els = els
    tree.space = F.FormulaSpace(top, els)
    sp = tree.space
    ion_mz = float(F.ion_mass(top, els))

    # ---- groups of scans (nodes) by path ------------------------------------------------------------------------------------------
    groups: dict = defaultdict(list)
    skipped = 0
    for s in run.scans:
        if s.level == 1:
            groups[()].append(s.index)
        elif s.path and len(s.path) == s.level - 1:
            groups[node_key(s.path)].append(s.index)
        else:
            skipped += 1
    if skipped:
        tree.warnings.append(f"{skipped} MSn scans without a path in the filter string: ignored")
    keys = sorted(groups, key=lambda k: (len(k), k))
    by_nominal: dict = defaultdict(list)
    for k in keys:
        by_nominal[tuple(m for m, _, _ in k)].append(k)

    # ---- consensus spectra --------------------------------------------------------------------------------------------------------
    t1 = time.perf_counter()
    cons = {}
    for i, k in enumerate(keys):
        cons[k] = consensus(run, groups[k])
        if progress:
            progress(f"MSn tree: node {i + 1}/{len(keys)}", (i + 1) / len(keys))
    tree.timing["consensus"] = round(time.perf_counter() - t1, 3)

    # ---- calibration: unique formula assignments of the strong peaks of all nodes -----------------------------------------------
    allmz = np.concatenate([cons[k][0][cons[k][1] >= 0.05 * (cons[k][1].max() if len(cons[k][1]) else 1)] for k in keys if k != () and len(cons[k][0])] or [np.zeros(0)])
    root_obs = None
    if () in cons and len(cons[()][0]):
        near = np.abs(cons[()][0] - ion_mz) <= 0.5
        if near.any():
            root_obs = float(cons[()][0][near][np.argmax(cons[()][1][near])])
            allmz = np.append(allmz, root_obs)
    if calibrate == "none" or len(allmz) == 0:
        tree.cal = F.Calibration()
    else:
        tree.cal = F.fit_calibration(allmz, sp, within=top, window_ppm=10.0, mode={"auto": "auto", "constant": "constant", "linear": "linear"}[calibrate])

    # ---- nodes, shallowest first --------------------------------------------------------------------------------------------------
    info: dict = {}
    rmz, rrel = (tree.cal.apply(cons[()][0]), cons[()][1]) if () in cons else (np.zeros(0), np.zeros(0))
    root = {"key": (), "formula": top, "prec_mz": ion_mz if root_obs is None else float(tree.cal.apply(root_obs)), "mz": rmz, "rel": rrel}
    info[()] = root
    tree.nodes.append({"id": "MS1", "path": [], "level": 1, "ce": None, "n_scans": len(groups.get((), [])), "prec_mz": root["prec_mz"],
                       "formula": F.fmt(top, els), "ppm": (root["prec_mz"] - ion_mz) / ion_mz * 1e6, "ghost": False, "parent": None,
                       "peaks": [], "via": 0, "atom_candidates": None})
    tree.by_id["MS1"] = tree.nodes[0]
    shown = 0                                  # nodes already reported to on_node (a node is final once its iteration is over)

    def report():
        nonlocal shown
        while on_node and shown < len(tree.nodes):
            on_node(tree.nodes[shown])
            shown += 1

    def find_parent(k):
        """Node of path[:-1]: the same CE if it exists, else the one with the most scans; if the generation is not in the file, the nearest ancestor."""
        pk, depth = k[:-1], 1
        while True:
            if pk == ():
                return (), depth
            cand = by_nominal.get(tuple(m for m, _, _ in pk), [])
            if cand:
                same = [c for c in cand if c == pk]
                return (same[0] if same else max(cand, key=lambda c: len(groups[c]))), depth
            pk, depth = pk[:-1], depth + 1

    for k in keys:
        if k == ():
            continue
        report()
        pk, depth = find_parent(k)
        par = info.get(pk)
        nominal = k[-1][0]
        mz_raw, rel, nsc = cons[k]
        mz = tree.cal.apply(mz_raw) if len(mz_raw) else mz_raw
        node = {"id": node_id(k), "path": [list(x) for x in k], "level": len(k) + 1, "ce": k[-1][2], "n_scans": len(groups[k]), "prec_mz": None,
                "formula": None, "ppm": None, "ghost": True, "parent": node_id(pk), "via": depth - 1, "peaks": [], "atom_candidates": None,
                "removed": {"heavy": 0, "not_subformula": 0, "no_formula": 0}}
        tree.nodes.append(node)
        tree.by_id[node["id"]] = node
        # exact precursor: the strongest peak of the parent within +-0.5 of the nominal value
        pm, pr = (par["mz"], par["rel"]) if par is not None and len(par["mz"]) else (np.zeros(0), np.zeros(0))
        sel = np.flatnonzero(np.abs(pm - nominal) <= 0.5) if len(pm) else np.zeros(0, int)
        if par is None or len(sel) == 0:
            node["note"] = "precursor not found in the parent spectrum"
            info[k] = {"key": k, "formula": None, "prec_mz": None, "mz": mz, "rel": rel}
            continue
        prec = float(pm[sel[np.argmax(pr[sel])]])
        cands = sp.candidates(prec, prec_ppm, 0.0, within=par["formula"])
        if len(cands) == 0:
            node["prec_mz"] = prec
            node["note"] = "no formula for the precursor (subformula of the parent)"
            info[k] = {"key": k, "formula": None, "prec_mz": prec, "mz": mz, "rel": rel}
            continue
        c0 = int(cands[0])
        fvec = sp.G[c0]
        node["prec_mz"], node["formula"], node["ppm"] = prec, F.fmt(fvec, els), float((prec - sp.mass[c0]) / sp.mass[c0] * 1e6)
        node["n_formulas"] = int(len(cands))
        # clean the spectrum: no peak heavier than the precursor, only sub-formulas of the node's formula, none without any formula
        n_any = sp.count(mz, frag_ppm, frag_abs)
        n_sub = sp.count(mz, frag_ppm, frag_abs, within=fvec)
        heavy = mz > prec + HEAVY
        not_sub = ~heavy & (n_sub == 0) & (n_any > 0)
        nofm = ~heavy & (n_any == 0)
        keep = ~heavy & (n_sub > 0)
        node["removed"] = {"heavy": int(heavy.sum()), "not_subformula": int(not_sub.sum()), "no_formula": int(nofm.sum())}
        base_removed = bool(len(rel)) and not keep[int(np.argmax(rel))]
        kidx = np.flatnonzero(keep)
        top_rel = rel[kidx].max() if len(kidx) else 1.0
        peaks = []
        for j in kidx:
            cc = sp.candidates(float(mz[j]), frag_ppm, frag_abs, within=fvec)
            c = int(cc[0])
            peaks.append({"mz": float(mz[j]), "mz_raw": float(mz_raw[j]), "rel": float(100 * rel[j] / top_rel), "formula": F.fmt(sp.G[c], els),
                          "ppm": float((mz[j] - sp.mass[c]) / sp.mass[c] * 1e6), "n_formulas": int(len(cc)), "n_scans": int(nsc[j])})
        node["peaks"] = peaks
        node["ghost"] = bool(len(peaks) < MIN_PEAKS or base_removed)
        node["base_peak_removed"] = base_removed
        info[k] = {"key": k, "formula": fvec, "prec_mz": prec, "mz": mz, "rel": rel,
                   "peak_vecs": np.array([sp.G[int(sp.candidates(float(mz[j]), frag_ppm, frag_abs, within=fvec)[0])] for j in kidx]) if len(kidx) else np.zeros((0, len(els)), np.int64)}
    report()
    tree.timing["tree"] = round(time.perf_counter() - t0, 3)

    # ---- atom sets ----------------------------------------------------------------------------------------------------------------
    if smiles:
        mol = MG.parse_smiles(smiles)
        tree.subs = MG.substructures(mol)
        _atom_sets(tree, info, keys)
    tree.timing["total"] = round(time.perf_counter() - t0, 3)
    return tree


def _atom_sets(tree: MsnTree, info: dict, keys: list):
    """Candidate substructures of every node (from its formula), then the hierarchical constraint to a fixed point: a child candidate must be a
    subset of at least one candidate of its parent, and a parent candidate must contain at least one candidate of each child that has any."""
    subs = tree.subs
    cand = {}
    for n in tree.nodes:
        if n["formula"] is None or n["id"] == "MS1":
            continue
        cand[n["id"]] = subs.candidates(F.vec(n["formula"], tree.els))
    kids = defaultdict(list)
    for n in tree.nodes:
        if n["id"] in cand and n["parent"] in cand:
            kids[n["parent"]].append(n["id"])
    for _ in range(10):
        changed = False
        for p, cs in kids.items():
            for c in cs:
                pidx, pw = cand[p]
                cidx, cw = cand[c]
                if len(cidx) == 0 or len(pidx) == 0:
                    continue
                pm, cm = subs.masks[pidx], subs.masks[cidx]
                sub = ((cm[:, None] & ~pm[None, :]) == 0)
                okc = sub.any(1)
                okp = sub.any(0)
                if not okc.all():
                    cand[c] = (cidx[okc], cw[okc] / max(cw[okc].sum(), 1e-300))
                    changed = True
                if okp.any() and not okp.all():
                    cand[p] = (pidx[okp], pw[okp] / max(pw[okp].sum(), 1e-300))
                    changed = True
        if not changed:
            break
    tree.cand = cand
    for n in tree.nodes:
        if n["id"] in cand:
            idx = cand[n["id"]][0]
            bits = subs.bits[idx]
            n["atom_candidates"] = {"n": int(len(idx)), "certain": np.flatnonzero(bits.all(0)).tolist() if len(idx) else [],
                                    "possible": np.flatnonzero(bits.any(0)).tolist() if len(idx) else []}
    # candidates of every clean peak: substructures of the node's own candidates
    for n in tree.nodes:
        if n["id"] not in cand or not n["peaks"]:
            continue
        pm = subs.masks[cand[n["id"]][0]]
        for j, pk in enumerate(n["peaks"]):
            idx, w = subs.candidates(F.vec(pk["formula"], tree.els))
            if len(idx) and len(pm):
                ok = ((subs.masks[idx][:, None] & ~pm[None, :]) == 0).any(1)
                idx, w = idx[ok], w[ok]
                w = w / w.sum() if len(w) else w
            tree.peak_cand[(n["id"], j)] = (idx, w)
            pk["n_atom_candidates"] = int(len(idx))
