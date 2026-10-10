# TPMINE-PRIVATE
"""High-resolution experiment (WP12): from the LC-HRMS files of a treatment series (and, if there is one, a direct-infusion MSn file of the parent) to
a ranked list of possible transformation products, each with its formula, kinetics, modified region, level of confidence and the evidence for it.

One file at a time is in memory: peak table -> features -> the trace of the parent and of its fragments -> table dropped. Only the small tables of
features and the index of the MS2 scans of every file are kept; peaks are read again for the few things that need them (MS2 of a candidate,
families of the best candidates). numpy only; runs in Pyodide."""
from __future__ import annotations

import copy
import gc
import time
from pathlib import Path

import numpy as np

from mzlab.chem import elements as E
from mzlab.reader.mzml import Run

from .. import chem
from . import features as FT
from . import filters as FL
from . import formula as F
from . import iimn
from . import kinetics as KN
from . import localize as LZ
from . import msn
from . import network as NW
from . import similarity as SM
from . import spectra as SP

DEFAULT_SETTINGS = {"floor": FT.FLOOR, "min_height": FT.MIN_HEIGHT, "fold": 5.0, "min_treated": 2, "rt_min": 0.7, "max_rows": 500, "iimn_top": 400,
                    "ppm_prec": 5.0, "weights": dict(NW.WEIGHTS), "polarity": 1, "inclusion": 50, "min_ms2_scans": 1, "flat_min_present": 2, "fill_gaps": True}
NO_PROGRESS = lambda text, frac=None, phase=None, payload=None: None      # noqa: E731


def classify(run: Run) -> str:
    """'msn' (levels 3 or above: the infusion of a standard), 'hr' (high-resolution MS1: the series), or the kind of the unit-resolution path."""
    levels = {s.level for s in run.scans}
    if max(levels, default=1) >= 3:
        return "msn"
    if getattr(run, "hr1", False) and 1 in levels:
        return "hr"
    return "other"


def ion_formula(neutral: dict, adduct: str = "[M+H]+") -> dict:
    """Ion formula of the neutral parent for the adducts the path knows ([M+H]+: one hydrogen more)."""
    if adduct != "[M+H]+":
        raise ValueError("high resolution uses the [M+H]+ ion")
    return chem.add(neutral, {"H": 1})


class ExperimentHR:
    def __init__(self, files: list[dict], parent: dict, settings: dict | None = None, progress=None):
        self.progress = progress or NO_PROGRESS
        self.s = {**copy.deepcopy(DEFAULT_SETTINGS), **(settings or {})}
        self.timing: dict = {}
        self.warnings: list[str] = []
        # --- parent
        text = parent.get("smiles") or parent.get("neutral")
        if not text:
            raise ValueError("the neutral molecular formula or the SMILES of the parent is required")
        self.smiles = parent.get("smiles") or (parent["neutral"] if not _is_formula(parent["neutral"]) else None)
        self.neutral = chem.neutral_formula(text)
        self.ion = ion_formula(self.neutral, parent.get("adduct") or "[M+H]+")
        self.els = F.element_order(self.ion)
        self.ion_vec = F.vec(self.ion, self.els)
        self.parent_mz = float(F.ion_mass(self.ion_vec, self.els))
        self.parent_name = parent.get("name") or ""
        # --- files
        self.files = []
        for d in files:
            x = {"name": d["name"], "path": d["path"], "label": d.get("label") or Path(d["name"]).stem, "time": d.get("time"), "type": d.get("type") or "sample"}
            self.files.append(x)
        self.lc = [x for x in self.files if x.get("kind", "hr") == "hr" and x["type"] in ("sample", "blank", "control")]
        self.msn_file = next((x for x in self.files if x.get("kind") == "msn"), None)
        self.tree = None
        self.library = None
        self.result: dict | None = None
        self._cons: dict = {}
        self._cand: dict = {}

    # ------------------------------------------------------------------ pipeline
    def run(self) -> dict:
        t0 = time.perf_counter()
        self._classify_files()
        self._tree()
        self._features()
        self._filters()
        self._fill_gaps()
        self._assemble()
        self._network()
        self.timing["total"] = round(time.perf_counter() - t0, 2)
        self.result = self._summary()
        return self.result

    def _classify_files(self):
        for x in self.files:
            if "kind" not in x:
                r = Run(x["path"])
                x["kind"] = classify(r)
                r.close()
        self.lc = [x for x in self.files if x["kind"] == "hr" and x["type"] in ("sample", "blank", "control")]
        self.msn_file = next((x for x in self.files if x["kind"] == "msn"), None)
        if not self.lc:
            raise ValueError("LC-HRMS (high-resolution) files are required: none found")
        self.lc.sort(key=lambda x: (x["time"] is None, x["time"] if x["time"] is not None else 0.0))
        ref = [x for x in self.lc if (x["time"] is not None and x["time"] <= 0) or x["type"] in ("blank", "control")]
        if not ref:
            self.warnings.append("no reference file (dark, t0 or blank): cannot tell what appears with the treatment")
        if not [x for x in self.lc if x["time"] is not None and x["time"] > 0]:
            raise ValueError("treated samples are required (time > 0)")

    def _tree(self):
        if self.msn_file is None:
            self.warnings.append("no MSn file of the standard: no fragment tree, no localisation")
            return
        self.progress("MSn tree of the parent...", 0.03, "tree")
        r = Run(self.msn_file["path"])
        t = time.perf_counter()
        self.tree = msn.build_tree(r, self.ion, self.smiles, on_node=lambda n: self.progress(
            "MSn tree of the parent...", 0.03, "tree", {"kind": "node", "id": n["id"], "parent": n["parent"], "level": n["level"],
                                                         "mz": None if n["prec_mz"] is None else round(n["prec_mz"], 4), "formula": n["formula"], "ghost": bool(n["ghost"])}))
        r.close()
        self.timing["albero"] = round(time.perf_counter() - t, 2)
        self.library = iimn.build_library(self.tree) if self.tree.subs is not None else None

    def _features(self):
        """File by file: reference files first, to know where the parent elutes; features, MS2 index, session, trace of the parent and of the fragments."""
        n = len(self.lc)
        order = sorted(range(n), key=lambda k: (0 if self._is_ref(self.lc[k]) else 1, k))
        self.feats: list = [None] * n
        self.ms2idx: list = [None] * n
        self.stamps: list = [None] * n
        self.parent_rt_hint = None
        masses = sorted(set(self.library.masses().tolist())) if self.library is not None else []
        self.isf_masses = masses
        self.isf_measures: list = [None] * n
        t0 = time.perf_counter()
        for step, k in enumerate(order):
            x = self.lc[k]
            self.progress(f"Looking for features: {x['label']}", 0.05 + 0.45 * step / n, "features")
            r = Run(x["path"])
            tab = r.table(1, self.s["polarity"])
            self.feats[k] = FT.detect(tab, floor=self.s["floor"], min_height=self.s["min_height"])
            self.progress(f"Looking for features: {x['label']}", 0.05 + 0.45 * (step + 1) / n, "features", {"kind": "features", "file": x["label"], "n": int(len(self.feats[k]))})
            self.ms2idx[k] = SP.index_ms2(r, x["path"])
            self.stamps[k] = KN.run_start(x["path"])
            if self.parent_rt_hint is None:
                w = FL.parent_window(tab, self.parent_mz)
                if w is not None:
                    self.parent_rt_hint = w["rt"]
            if masses:
                self.isf_measures[k] = FL.isf_measure(tab, masses, self.parent_mz, rt_hint=self.parent_rt_hint)
            r._tables.clear()
            r.close()
            del tab
            gc.collect()
        self.timing["feature"] = round(time.perf_counter() - t0, 2)
        self.times = np.array([x["time"] if x["time"] is not None else (-1.0 if self._is_ref(x) else np.nan) for x in self.lc], float)
        self.sessions = KN.sessions(self.stamps)

    @staticmethod
    def _is_ref(x: dict) -> bool:
        return (x["time"] is not None and x["time"] <= 0) or x["type"] in ("blank", "control")

    def _filters(self):
        self.progress("Aligning and filtering...", 0.52, "align")
        t = time.perf_counter()
        types = [("blank" if x["type"] == "blank" else "control" if x["type"] == "control" else "sample") for x in self.lc]
        self.al = FT.align(self.feats)
        self.sess_factors = KN.session_factors(self.al.area, self.sessions)
        if self.sess_factors["applied"]:
            self.warnings.append("measurement sessions differ: areas were rescaled to the reference session")
        self.funnel = FL.run_filters(self.al, self.feats, self.times, types, self.ion, fold=self.s["fold"], min_treated=self.s["min_treated"], rt_min=self.s["rt_min"],
                                     min_height=self.s["min_height"], flat_min_present=self.s["flat_min_present"],
                                     isf_masses=self.isf_masses, parent_rt=self.parent_rt_hint)
        self.ref_c, self.tr_c = FL.split_columns(self.times, types)
        self.timing["filtri"] = round(time.perf_counter() - t, 2)
        # parent: area per file (saturation), peak limits
        g = self._parent_group()
        self.parent_group = g
        self.saturation = KN.parent_saturation(self.times, self.al.area[g]) if g is not None else {"saturated": False, "note": ""}
        if self.saturation["saturated"]:
            self.warnings.append(self.saturation["note"])
        self.isf_hidden = []
        if any(m is not None for m in self.isf_measures):
            self.isf_hidden = FL.isf_hidden(self.isf_measures, self.times, types, self.isf_masses, self.sessions)

    def _fill_gaps(self):
        """Zero areas of the candidates in the treated files become the area of the same ion re-extracted at the intensity floor (a weak product at
        the first times falls under `min_height` and would look like a late one). The files are read again, one at a time; `self.filled` marks the cells."""
        self.filled = np.zeros(self.al.area.shape, bool)
        if not self.s["fill_gaps"] or len(self.funnel.idx) == 0:
            return
        t0 = time.perf_counter()
        self.progress("Filling gaps...", 0.55, "align")
        getters = self._runs()
        last = []

        def tables(k):
            for r in last:                                   # one file in memory at a time
                r._tables.clear()
                r.close()
            last.clear()
            r = getters[k]()
            last.append(r)
            return r.table(1, self.s["polarity"])
        self.filled = FT.fill_gaps(self.al, self.funnel.idx, tables, [int(c) for c in self.tr_c], floor=self.s["floor"])
        self._close_runs()
        gc.collect()
        self.timing["riempimento"] = round(time.perf_counter() - t0, 2)

    def _parent_group(self):
        near = np.flatnonzero(np.abs(self.al.mz - self.parent_mz) <= self.parent_mz * 5e-6)
        if len(near) == 0:
            return None
        if self.parent_rt_hint is not None:
            close = near[np.abs(self.al.rt[near] - self.parent_rt_hint) <= 0.3]
            near = close if len(close) else near
        return int(near[np.argmax(self.al.area[near].max(1))])

    # ------------------------------------------------------------------ candidates
    def _runs(self):
        """Open runs, on demand (a callable per file: the file is opened the first time it is needed)."""
        cache = {}

        def getter(k):
            def f():
                if k not in cache:
                    cache[k] = Run(self.lc[k]["path"])
                return cache[k]
            return f
        self._open = cache
        return [getter(k) for k in range(len(self.lc))]

    def _close_runs(self):
        """Close the files opened on demand (their pages count in the resident memory while the mappings are alive)."""
        for r in list(getattr(self, "_open", {}).values()):
            r._tables.clear()
            r.close()
        getattr(self, "_open", {}).clear()

    def _ms2_of(self, mz: float, limits) -> dict | None:
        sel = SP.select_ms2(self.ms2idx, mz, limits)
        if len(sel) < self.s["min_ms2_scans"]:
            return None
        a, b, rts = SP.read_spectra(self.getters, sel)
        for r in getattr(self, "_open", {}).values():                  # in the browser every open file keeps a 16 MB window: give them back
            r.release()
        c = SP.ms2_consensus(a, b)
        c["rts"], c["raw"] = rts, (a, b)
        return c

    def _assemble(self):
        t0 = time.perf_counter()
        self.progress("Assembling the candidates...", 0.58, "candidates")
        al, idx = self.al, self.funnel.idx
        self.getters = self._runs()
        space, _ = FL.tp_space(self.ion)
        table = NW.transformation_table(self.els)
        pv = self.ion_vec
        amax = al.area[idx][:, self.tr_c].max(1) if len(idx) else np.zeros(0)
        lg = np.log10(np.maximum(amax, 1))
        lgn = (lg - lg.min()) / max(lg.max() - lg.min(), 1e-12) if len(lg) else lg
        index = NW.mz_index(al)
        # parent MS2 (HCD) and the library
        pg = self.parent_group
        self.parent_ms2 = None
        if pg is not None:
            self.parent_ms2 = self._ms2_of(self.parent_mz, [(al.left[pg, k], al.right[pg, k]) for k in range(len(self.lc))])
        if self.parent_ms2 is not None and self.tree is not None and self.library is not None:
            self.library = iimn.build_library(self.tree, dda_peaks=(self.parent_ms2["mz"], self.parent_ms2["rel"]))
        pspec = SM.prepare(self.parent_ms2["mz"], self.parent_ms2["rel"], self.parent_mz) if self.parent_ms2 is not None else None
        cands, specs, precs, pos = [], [], [], []
        ref_max_all = al.area[:, self.ref_c].max(1) if len(self.ref_c) else np.zeros(len(al))
        for k, g in enumerate(idx):
            mz = float(al.mz[g])
            c = {"id": int(g), "mz": mz, "rt": float(al.rt[g]), "area_max": float(amax[k]), "areas": al.area[g].tolist(), "time_step": self._step()}
            cc = space.candidates(mz, 3.0)
            c["n_formulas"] = int(len(cc))
            if len(cc):
                v = space.G[int(cc[0])]
                c["formula"], c["ppm"] = F.fmt(v, self.els), float(space.error_ppm(mz, int(cc[0])))
                c["derivation"] = NW.derivation_name(v, pv, self.els, table)
                c["delta"] = ("-" if (v - pv).sum() < 0 else "+") + F.fmt(np.abs(v - pv), self.els)
                exp = NW.expected_isotopes(v, self.els)
                st, tx = NW.isotopes_coherent(NW.observed_isotopes(al, int(g), index), exp)
                c["isotopes"] = {"status": st, "text": tx}
            else:
                c["formula"] = None
            d = KN.descriptors(self.times, al.area[g])
            if d.get("ok"):
                d["absent_in_reference"] = KN.not_in_reference(self.times, al.area[g][self.tr_c], float(ref_max_all[g]))
                c["kinetics"] = d
            ms2 = self._ms2_of(mz, [(al.left[g, f], al.right[g, f]) for f in range(len(self.lc))]) if pspec is not None else None
            if ms2 is not None:
                sp = SM.prepare(ms2["mz"], ms2["rel"], mz)
                c["ms2"] = {"n_scans": ms2["n_scans"], "modcos": None, "n_matched": 0}
                self._cons[int(g)] = ms2
                specs.append(sp)
                precs.append(mz)
                pos.append(k)
            cands.append(c)
        if pspec is not None and specs:
            M, W, P = SM.pad([pspec] + specs, [self.parent_mz] + precs)
            _, sc, nm = SM.one_against_all(M, W, P, 0)
            _, se, ne = SM.entropy_one_against_all(M, W, P, 0)
            for j, kk in enumerate(pos):
                cands[kk]["ms2"].update(modcos=float(sc[j]), n_matched=int(nm[j]), entropy=float(se[j]), n_matched_entropy=int(ne[j]))
            self._pad = (M, W, P, pos)
        # localisation of the candidates that have MS2 and a formula
        if self.library is not None and self.tree is not None and self.tree.subs is not None:
            groups = LZ.default_groups(self.tree.subs.mol)
            for kk in pos:
                c = cands[kk]
                if c["formula"] is None:
                    continue
                sp = F.FormulaSpace(F.vec(c["formula"], self.els), self.els)
                m2 = self._cons[c["id"]]
                r = LZ.localize(c["formula"], F.fmt(pv, self.els), m2["mz"], m2["rel"], self.library, sp, groups=None, tp_mz=c["mz"])
                r["n_atoms"] = self.tree.subs.mol.n
                if r.get("ok"):
                    r["n_groups"] = sum(1 for gset in groups.values() if gset & set(r["region_atoms"]))
                c["localization"] = r
        lgn = np.concatenate([lgn, self._hidden_candidates(cands, space, table, pv)])
        self.cands, self.lgn = cands, lgn
        self._close_runs()
        self.timing["candidati"] = round(time.perf_counter() - t0, 2)

    def _hidden_candidates(self, cands: list, space, table, pv) -> np.ndarray:
        """Masses of the parent's fragments whose ratio to the parent changes with the treatment (or whose peak moves away from the parent's): a possible
        product hidden under an in-source fragment. Those without a feature of their own become rows with a negative id (no peaks of their own: no
        kinetics, no MS2); a feature that is already a candidate is only flagged. Returns the normalised intensity of the new rows."""
        self.hidden_ids = set()
        have = {id(c): c for c in cands}
        prt = float(self.al.rt[self.parent_group]) if self.parent_group is not None else (self.parent_rt_hint or 0.0)
        new = []
        for j, d in enumerate(d for d in self.isf_hidden if d["candidate"]):
            if any(abs(c["mz"] - d["mz"]) <= d["mz"] * 5e-6 and abs(c["rt"] - prt) <= 0.5 for c in cands):
                continue
            cc = space.candidates(d["mz"], 3.0)
            c = {"id": -(j + 1), "mz": d["mz"], "rt": prt, "area_max": 0.0, "areas": [0.0] * len(self.lc), "time_step": self._step(), "hidden": True, "isf_hidden": d,
                 "n_formulas": int(len(cc))}
            if len(cc):
                v = space.G[int(cc[0])]
                c["formula"], c["ppm"] = F.fmt(v, self.els), float(space.error_ppm(d["mz"], int(cc[0])))
                c["derivation"] = NW.derivation_name(v, pv, self.els, table)
                c["delta"] = ("-" if (v - pv).sum() < 0 else "+") + F.fmt(np.abs(v - pv), self.els)
            else:
                c["formula"] = None
            new.append(c)
            self.hidden_ids.add(c["id"])
        cands.extend(new)
        return np.full(len(new), 0.3)

    def _step(self) -> float:
        t = np.sort(np.unique(self.times[self.times > 0]))
        return float(np.median(np.diff(t))) if len(t) > 1 else 5.0

    # ------------------------------------------------------------------ network, scoring, ranking
    def _network(self):
        t0 = time.perf_counter()
        self.progress("IIMN families of the best candidates...", 0.7, "families")
        cands = self.cands
        w = self.s["weights"]
        for k, c in enumerate(cands):
            c["priority"] = NW.priority(c, self.lgn[k], w)
        for k in sorted((k for k, c in enumerate(cands) if not c.get("hidden")), key=lambda k: -cands[k]["priority"]["score"])[:40]:
            c = cands[k]                      # live feed: the best candidates with the provisional score (before the IIMN families)
            self.progress("IIMN families of the best candidates...", 0.7, "families", {"kind": "candidate", "id": c["id"], "name": c.get("derivation") or c.get("formula") or f"m/z {c['mz']:.4f}",
                                                                                      "mz": round(c["mz"], 4), "rt": round(c["rt"], 2), "score": c["priority"]["score"]})
        self._iimn_top()
        for k, c in enumerate(cands):
            c["priority"] = NW.priority(c, self.lgn[k], w)
        # isf-coincident flags
        real = [c for c in cands if not c.get("hidden")]
        coin = FL.isf_coincident(self.al, np.array([c["id"] for c in real], int), self.isf_masses, self.al.rt[self.parent_group] if self.parent_group is not None else (self.parent_rt_hint or 0.0)) if real else np.zeros(0, bool)
        for c, f in zip(real, coin):
            c["isf_coincident"] = bool(f)
        for c in cands:
            if c.get("hidden"):
                c["isf_coincident"] = True
        # predecessors among the best candidates
        self.progress("Product network...", 0.85, "network")
        top = [k for k in sorted(range(len(cands)), key=lambda k: -cands[k]["priority"]["score"])[:300] if cands[k].get("formula")]
        sub = [cands[k] for k in top]
        cos = {}
        pad = getattr(self, "_pad", None)
        if pad is not None and sub:
            M, W, P, pos = pad
            where = {kk: j + 1 for j, kk in enumerate(pos)}            # row of the padded matrices (row 0 is the parent)
            members = [j for j, k in enumerate(top) if k in where]
            if len(members) > 1:
                sel = np.array([where[top[j]] for j in members])
                ia, ib, sc, nm = SM.all_pairs(M[sel], W[sel], P[sel], threshold=0.3)
                for a, b, s, n in zip(ia.tolist(), ib.tolist(), sc.tolist(), nm.tolist()):
                    cos[(members[a], members[b])] = cos[(members[b], members[a])] = (float(s), int(n))
        pf = F.fmt(self.ion_vec, self.els)
        preds = NW.predecessors(sub, self.els, pf, NW.transformation_table(self.els), cos=cos, step=self._step())
        by = {p["id"]: p for p in preds}
        prt = float(self.al.rt[self.parent_group]) if self.parent_group is not None else self.parent_rt_hint
        tk = {c["id"]: (c.get("kinetics") or {}).get("tmax") for c in cands}
        for c in cands:
            p = by.get(c["id"])
            c["predecessor"] = p
            if p and p["predecessor"] not in (None, "progenitore"):
                c["predecessor_kinetics"] = tk.get(p["predecessor"])
            loc = c.get("localization") or {}
            c["confidence"] = NW.confidence(c, parent_rt=prt, groups_of_region=loc.get("n_groups"))
        self.ranked = NW.rank(cands)
        self._close_runs()
        self.timing["rete"] = round(time.perf_counter() - t0, 2)

    def _iimn_top(self):
        cands = self.cands
        top = [k for k in sorted(range(len(cands)), key=lambda k: -cands[k]["priority"]["raw"]) if not cands[k].get("hidden")][: self.s["iimn_top"]]
        by_file: dict = {}
        for k in top:
            by_file.setdefault(int(np.argmax(self.al.area[cands[k]["id"]])), []).append(k)
        for f, ks in by_file.items():
            r = self.getters[f]()
            tab = r.table(1, self.s["polarity"])
            for lo, hi, ii in iimn.chunks([cands[k]["rt"] for k in ks], span=0.5):
                win = iimn.window_profiles(tab, lo, hi, floor=self.s["floor"])
                for j in ii:
                    k = ks[j]
                    fam = iimn.family(win, cands[k]["mz"], cands[k]["rt"], parent_mz=self.parent_mz, library=self.library)
                    if fam:
                        cands[k]["iimn"] = iimn.collapse(fam)
            r._tables.clear()

    # ------------------------------------------------------------------ output
    def _row(self, c: dict) -> dict:
        kin = c.get("kinetics") or {}
        ms2 = c.get("ms2") or {}
        loc = c.get("localization") or {}
        flags = []
        if (c.get("iimn") or {}).get("role", "ion") != "ion":
            flags.append(c["iimn"]["explained_text"])
        if c.get("isf_coincident"):
            flags.append("possible TP co-eluting with an ISF")
        return {"id": c["id"], "mz": round(c["mz"], 4), "rt": round(c["rt"], 2), "formula": c.get("formula"), "ppm": None if c.get("ppm") is None else round(c["ppm"], 1),
                "n_formulas": c.get("n_formulas"), "delta": c.get("delta"), "derivation": c.get("derivation"), "area_max": c["area_max"],
                "tmax": kin.get("tmax"), "class": kin.get("class_text"), "onset": kin.get("onset"),
                "score": round(c["priority"]["score"], 1), "components": {k: round(c["priority"]["components"][k], 3) for k in NW.WEIGHTS},
                "level": c["confidence"]["level"], "ms2_scans": ms2.get("n_scans", 0), "modcos": None if ms2.get("modcos") is None else round(ms2["modcos"], 3),
                "n_matched": ms2.get("n_matched", 0), "entropy": None if ms2.get("entropy") is None else round(ms2["entropy"], 3), "region": loc.get("region") if loc.get("ok") else None, "margin": None if not loc.get("ok") else round(loc["margin"], 2),
                "predecessor": (c.get("predecessor") or {}).get("predecessor"), "transformation": (c.get("predecessor") or {}).get("transformation"),
                "flags": flags, "filled": [int(k) for k in np.flatnonzero(self.filled[c["id"]])] if not c.get("hidden") and self.filled[c["id"]].any() else [], "series": [round(float(v)) for v in c["areas"]]}

    def _summary(self) -> dict:
        rows = [self._row(c) for c in self.ranked[: self.s["max_rows"]]]
        noms = [c for c in self.cands if not c.get("ms2") and not c.get("hidden")]
        incl = sorted(noms, key=lambda c: -c["priority"]["score"])[: self.s["inclusion"]]
        files = [{"name": x["name"], "label": x["label"], "time": x["time"], "type": x["type"], "kind": x["kind"], "session": s, "n_features": int(len(f))}
                 for x, s, f in zip(self.lc, self.sessions, self.feats)]
        return {"mode": "hr", "parent": {"formula": chem.fmt(self.neutral), "ion": F.fmt(self.ion_vec, self.els), "mz": round(self.parent_mz, 4),
                                         "rt": round(float(self.al.rt[self.parent_group]), 2) if self.parent_group is not None else self.parent_rt_hint, "name": self.parent_name},
                "files": files, "times": [float(t) for t in self.times], "sessions": {"labels": self.sessions, "factors": self.sess_factors},
                "saturation": self.saturation, "funnel": self.funnel.steps, "n_candidates": len(self.cands), "n_rows": len(rows), "rows": rows,
                "isf_hidden": [d for d in self.isf_hidden if d["candidate"]], "tree": self._tree_summary(), "calibration": self.tree.cal.as_dict() if self.tree else None,
                "inclusion": [{"mz": round(c["mz"], 4), "rt": round(c["rt"], 2), "rt_from": round(float(np.nanmin(self.al.left[c["id"]])), 2),
                               "rt_to": round(float(np.nanmax(self.al.right[c["id"]])), 2), "formula": c.get("formula"), "score": round(c["priority"]["score"], 1)} for c in incl],
                "timing": self.timing, "warnings": self.warnings, "settings": self.s, "weights": self.s["weights"]}

    def _tree_summary(self):
        if self.tree is None:
            return None
        out = []
        for n in self.tree.summary():
            out.append({"id": n["id"], "level": n["level"], "parent": n["parent"], "formula": n["formula"], "prec_mz": None if n["prec_mz"] is None else round(n["prec_mz"], 4),
                        "ppm": None if n["ppm"] is None else round(n["ppm"], 1), "n_scans": n["n_scans"], "ghost": n["ghost"], "ce": n["ce"],
                        "peaks": [{"mz": round(p["mz"], 4), "rel": round(p["rel"], 1), "formula": p["formula"], "ppm": round(p["ppm"], 1)} for p in sorted(n["peaks"], key=lambda p: -p["rel"])[:8]]})
        return out

    def msn_tree(self) -> list | None:
        return self._tree_summary()

    def detail(self, cid: int) -> dict:
        c = next((x for x in self.cands if x["id"] == cid), None)
        if c is None:
            raise ValueError("candidate not found")
        kin = c.get("kinetics") or {}
        out = {"id": cid, "row": self._row(c), "criteria": c["confidence"]["criteria"], "level_text": c["confidence"]["text"], "components": c["priority"]["components"],
               "times": [float(t) for t in self.times], "areas": [float(a) for a in c["areas"]], "sessions": self.sessions, "kinetics": {k: v for k, v in kin.items() if k != "profile"},
               "iimn": {k: v for k, v in (c.get("iimn") or {}).items() if k != "evidence"} if c.get("iimn") else None,
               "evidence_family": (c.get("iimn") or {}).get("evidence", []), "predecessor": c.get("predecessor"), "isotopes": c.get("isotopes")}
        if c["id"] in self._cons and self.parent_ms2 is not None:
            m2 = self._cons[c["id"]]
            loc = c.get("localization") or {}
            out["ms2"] = {"candidate": {"mz": [float(v) for v in m2["mz"]], "rel": [float(v) for v in m2["rel"]], "n_scans": m2["n_scans"]},
                          "parent": {"mz": [float(v) for v in self.parent_ms2["mz"]], "rel": [float(v) for v in self.parent_ms2["rel"]], "n_scans": self.parent_ms2["n_scans"]},
                          "evidence": loc.get("evidence", [])}
            rts = np.asarray(m2["rts"], float)
            if len(rts) >= 4:
                d = SP.deconvolve_isomers(m2["raw"][0], m2["raw"][1], rts, xic=None)
                out["isomers"] = {"k": d["k"], "components": [{"rt_apex": cm["rt_apex"], "mz": [float(v) for v in cm["spectrum"][0]], "rel": [float(v) for v in cm["spectrum"][1]]} for cm in d["components"]]}
        if c.get("localization"):
            loc = c["localization"]
            out["localization"] = {k: v for k, v in loc.items() if k not in ("evidence",)}
            out["localization"]["evidence"] = loc.get("evidence", [])
        return out

    def inclusion_csv(self, n: int | None = None) -> str:
        """Inclusion list for a second, targeted injection: the best candidates without MS2 (m/z, charge, start and end RT, note)."""
        rows = (self.result or self._summary())["inclusion"][: n or self.s["inclusion"]]
        lines = ["m/z,charge,RT_start_min,RT_end_min,note"]
        for r in rows:
            lines.append(f"{r['mz']},1,{r['rt_from']},{r['rt_to']},\"{(r['formula'] or '')} priority {r['score']}\"")
        return "\n".join(lines)


def _is_formula(text: str) -> bool:
    import re
    return bool(re.fullmatch(r"(?:[A-Z][a-z]?\d*)+", text.strip()))
