# TPMINE-PRIVATE
"""Ion identity families of the candidates (WP6) and the fragment library of the parent.

For each candidate: the features that elute with it (profile correlation, same apex) and whose m/z differs from it by an adduct, an isotope, a
neutral loss in the source or the mass of a dimer. The family collapses to a neutral mass and a representative [M+H]+; a candidate that is an
adduct, an isotopologue or an in-source fragment of a stronger co-eluting ion is marked as such (kept, shown last), not removed. numpy only: the
profiles of a retention-time window are a dense (features x scans) matrix and all the correlations are one matrix product."""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from mzlab import ionfamily
from mzlab.chem import elements as E

from . import formula as F

PROTON = E.PROTON
H = E.MASS["H"]
# m/z shift of the adduct relative to [M+H]+ (so the same table serves both directions)
ADDUCT_FROM_H = {
    "[M+Na]+": E.MASS["Na"] - E.ELECTRON - PROTON,
    "[M+K]+": E.MASS["K"] - E.ELECTRON - PROTON,
    "[M+NH4]+": E.MASS["N"] + 4 * H - E.ELECTRON - PROTON,
    "[M+H+ACN]+": E.mass({"C": 2, "H": 3, "N": 1}),
}
# the same, relative to [M-H]- (negative mode)
ADDUCT_FROM_DEPROT = {
    "[M+FA-H]-": E.mass({"C": 1, "H": 2, "O": 2}),
    "[M+Cl]-": E.MASS["Cl"] + E.ELECTRON + PROTON,
    "[M+Na-2H]-": E.MASS["Na"] - E.ELECTRON - PROTON,
}
ISOTOPES = {"13C": 1.003355, "34S/13C2": 1.99580, "13C2": 2.00671}                  # distance above the monoisotopic ion
# losses in the source, besides ionfamily.NEUTRAL_LOSSES: the alkene losses typical of tert-butyl, isopropyl and morpholine-like groups
EXTRA_LOSSES = ["C4H8", "C3H6", "C2H4", "C4H9N", "C4H8O"]
NH3 = E.mass({"N": 1, "H": 3})


def _loss_table() -> dict:
    out = {}
    for name in list(ionfamily.NEUTRAL_LOSSES) + EXTRA_LOSSES:
        try:
            out[name] = E.mass(E.parse_formula(name))
        except Exception:      # noqa: BLE001
            pass
    return out


LOSSES = _loss_table()


@dataclass
class Window:
    """Dense profiles of the features whose trace lies in a retention-time window of one file."""
    mz: np.ndarray                  # intensity-weighted m/z of every feature
    height: np.ndarray
    apex: np.ndarray                # position of the apex in the window (0 .. n_scans - 1)
    Y: np.ndarray                   # (features, scans) summed intensity per scan
    Yn: np.ndarray                  # centred and normalised rows (correlation = dot product)
    rt: np.ndarray                  # retention time of every scan of the window
    artefact: np.ndarray

    def __len__(self):
        return len(self.mz)


def window_profiles(table, rt_lo: float, rt_hi: float, floor: float = 2e4, min_points: int = 6, min_height: float = 1e5, max_gap: int = 2) -> Window:
    """Features with a trace in [rt_lo, rt_hi] of a PeakTable, as dense profiles (one channel grid of 4 ppm; segments cut where more than
    `max_gap` scans are missing; at least `min_points` points and `min_height`)."""
    from .features import CHANNEL_PPM, fourier_artefacts
    i0 = int(np.searchsorted(table.rt, rt_lo))
    i1 = int(np.searchsorted(table.rt, rt_hi, "right"))
    n = max(i1 - i0, 0)
    empty = Window(np.zeros(0), np.zeros(0), np.zeros(0, int), np.zeros((0, n)), np.zeros((0, n)), table.rt[i0:i1], np.zeros(0, bool))
    if n < 3:
        return empty
    k = (table.pos >= i0) & (table.pos < i1) & (table.inten >= floor)
    mz, it, pos = table.mz[k].astype(np.float64), table.inten[k].astype(np.float64), table.pos[k].astype(np.int64) - i0
    if len(mz) == 0:
        return empty
    b = np.floor(np.log(mz) / np.log1p(CHANNEL_PPM)).astype(np.int64)
    key = b * (n + 1) + pos
    o = np.argsort(key, kind="stable")
    ks = key[o]
    cell = np.flatnonzero(np.r_[True, ks[1:] != ks[:-1]])
    I = np.add.reduceat(it[o], cell)
    MZw = np.add.reduceat((it * mz)[o], cell)
    uk = ks[cell]
    B, P = uk // (n + 1), uk % (n + 1)
    brk = np.r_[True, (B[1:] != B[:-1]) | (P[1:] - P[:-1] > max_gap)]
    st = np.flatnonzero(brk)
    seg = np.cumsum(brk) - 1
    cnt = np.diff(np.r_[st, len(P)])
    mx = np.maximum.reduceat(I, st)
    good = np.flatnonzero((cnt >= min_points) & (mx >= min_height))
    if len(good) == 0:
        return empty
    remap = np.full(len(st), -1)
    remap[good] = np.arange(len(good))
    r = remap[seg]
    ok = r >= 0
    Y = np.zeros((len(good), n))
    np.add.at(Y, (r[ok], P[ok]), I[ok])
    tot = np.add.reduceat(I, st)
    mzc = (np.add.reduceat(MZw, st) / tot)[good]
    apex = np.argmax(Y, axis=1)
    Yc = Y - Y.mean(1, keepdims=True)
    Yn = Yc / np.maximum(np.linalg.norm(Yc, axis=1, keepdims=True), 1e-12)
    return Window(mz=mzc, height=Y.max(1), apex=apex, Y=Y, Yn=Yn, rt=table.rt[i0:i1], artefact=fourier_artefacts(mzc, apex, Y.max(1)))


# ---------------------------------------------------------------------------------------------------------------------- relations
def relations(delta: float, mz_ref: float, ppm: float = 5.0, abs_da: float = 0.002, adducts: dict | None = None) -> list[dict]:
    """What `delta` (= m/z of Y minus m/z of X) can be. Each hypothesis has a role (adduct, isotope, loss) and `of`: the ion it derives from, "x"
    (Y is the adduct / isotopologue / in-source fragment of X) or "y" (X is that of Y). Several hypotheses can fit: all are returned.
    `adducts`: the adduct table relative to the base ion ([M+H]+ by default; `ADDUCT_FROM_DEPROT` for [M-H]-)."""
    tol = max(mz_ref * ppm * 1e-6, abs_da)
    out = []
    for name, d in (ADDUCT_FROM_H if adducts is None else adducts).items():
        if abs(delta - d) <= tol:
            out.append({"role": "adduct", "name": name, "of": "x", "delta": d})            # Y is the adduct of X (X = [M+H]+)
        if abs(delta + d) <= tol:
            out.append({"role": "adduct", "name": name, "of": "y", "delta": -d})           # X is the adduct of Y
    for name, d in ISOTOPES.items():
        if abs(delta - d) <= tol:
            out.append({"role": "isotope", "name": name, "of": "x", "delta": d})           # Y is an isotopologue of X
        if abs(delta + d) <= tol:
            out.append({"role": "isotope", "name": name, "of": "y", "delta": -d})          # X is an isotopologue of Y
    for name, d in LOSSES.items():
        if abs(delta + d) <= tol:
            out.append({"role": "loss", "name": name, "of": "x", "delta": -d})             # Y = X - loss: Y is an in-source fragment of X
        if abs(delta - d) <= tol:
            out.append({"role": "loss", "name": name, "of": "y", "delta": d})              # X = Y - loss: X is an in-source fragment of Y
    return out


def dimer_of(mz_x: float, mz_y: float, ppm: float = 5.0, abs_da: float = 0.002, sign: int = 1) -> bool:
    """Is Y the [2M+H]+ dimer of X = [M+H]+ ?  2 (X - proton) + proton. `sign` -1: [2M-H]- of [M-H]-."""
    tol = max(mz_y * ppm * 1e-6, abs_da)
    return abs(mz_y - (2 * (mz_x - sign * PROTON) + sign * PROTON)) <= tol


def family(win: Window, mz: float, apex_rt: float, r_min: float = 0.85, apex_scans: int = 2, ppm: float = 5.0, parent_mz: float | None = None,
           library: "FragmentLibrary | None" = None) -> dict | None:
    """The family of the feature of `win` at `mz` (5 ppm) closest to `apex_rt`: members that elute with it (r >= r_min, apex within `apex_scans`)
    and that stand in a known relation to it. Artefacts of the Fourier transform are left out. None when the candidate is not in the window.
    With `parent_mz` and the fragment `library` of the parent: a candidate whose m/z is a fragment of the library and that elutes with the parent
    is an in-source fragment of it, whatever the neutral loss (the library knows the real ones, from the MSn tree)."""
    if len(win) == 0:
        return None
    cand = np.flatnonzero(np.abs(win.mz - mz) <= mz * ppm * 1e-6)
    if len(cand) == 0:
        return None
    c = int(cand[np.argmin(np.abs(win.rt[win.apex[cand]] - apex_rt))])
    r = win.Yn @ win.Yn[c]
    near = (r >= r_min) & (np.abs(win.apex - win.apex[c]) <= apex_scans) & ~win.artefact
    near[c] = False
    members = []
    for j in np.flatnonzero(near):
        rel = relations(float(win.mz[j] - win.mz[c]), float(win.mz[c]), ppm)
        dimer = dimer_of(float(win.mz[c]), float(win.mz[j]), ppm)
        dimer_rev = dimer_of(float(win.mz[j]), float(win.mz[c]), ppm)
        if rel or dimer or dimer_rev:
            members.append({"mz": float(win.mz[j]), "r": float(r[j]), "height": float(win.height[j]), "delta": float(win.mz[j] - win.mz[c]), "relations": rel,
                            "dimer_of_x": bool(dimer), "x_is_dimer_of_y": bool(dimer_rev)})
    if parent_mz is not None and library is not None and len(library):
        pj = np.flatnonzero(np.abs(win.mz - parent_mz) <= parent_mz * ppm * 1e-6)
        pj = pj[np.argsort(-win.height[pj])] if len(pj) else pj
        lm = library.masses()
        hit = np.flatnonzero(np.abs(lm - win.mz[c]) <= win.mz[c] * ppm * 1e-6)
        if len(pj) and len(hit) and not any(m["mz"] == float(win.mz[pj[0]]) for m in members):
            j = int(pj[0])
            if r[j] >= r_min and abs(int(win.apex[j]) - int(win.apex[c])) <= max(apex_scans, 4):          # the flat top of a saturated parent moves its apex
                name = list(library.entries)[int(hit[0])]
                members.append({"mz": float(win.mz[j]), "r": float(r[j]), "height": float(win.height[j]), "delta": float(win.mz[j] - win.mz[c]),
                                "relations": [{"role": "loss", "name": f"{name}", "library": True, "of": "y", "delta": float(win.mz[j] - win.mz[c])}],
                                "dimer_of_x": False, "x_is_dimer_of_y": False})
    return {"mz": float(win.mz[c]), "height": float(win.height[c]), "rt": float(win.rt[win.apex[c]]), "members": members}


def collapse(fam: dict) -> dict:
    """Neutral mass and representative [M+H]+ of a family, and what the candidate is.

    The candidate X is *explained* when a member Y shows X as its adduct, in-source fragment (Y at least 2 times higher: two co-eluting products
    that differ by a neutral loss are not a fragment and its precursor), isotopologue (Y at least 1.5 times higher) or as the [2M+H]+ of its monomer; then Y is the representative and X is kept with its role. Otherwise X is the representative ([M+H]+, or an unexplained
    ion taken as such) and the members are its evidence (adducts, isotopes, dimer).

    NH4 / NH3 ambiguity: +17.02655 is both [M+NH4]+ of an [M+H]+ and [M+H]+ of a fragment that lost NH3. Both readings are kept in `ambiguous`;
    the sodium adduct of X decides for the ammonium reading, otherwise it is left undecided and X is not explained away by either."""
    x = fam["mz"]
    out = {"mz": x, "representative_mz": x, "neutral_mass": x - PROTON, "role": "ion", "role_name": None, "explained_by": None, "explained_text": "",
           "evidence": [], "ambiguous": None}
    nh = ADDUCT_FROM_H["[M+NH4]+"]
    na_on_x = any(r["name"] == "[M+Na]+" and r["of"] == "x" for m in fam["members"] for r in m["relations"])
    explained, evidence, ambiguous = [], [], []
    for m in fam["members"]:
        for rel in m["relations"]:
            if abs(abs(rel["delta"]) - nh) < 5e-3 and rel["role"] in ("adduct", "loss"):
                ambiguous.append(m)
                if not na_on_x:
                    continue                                                   # undecided: neither evidence nor explanation
            if rel["of"] == "x":                                              # Y derives from X: evidence for X
                evidence.append({"kind": rel["role"], "name": rel["name"], "mz": m["mz"], "r": m["r"], "text": f"{rel['name']} at {m['delta']:+.4f} (r={m['r']:.2f})"})
            elif (m["height"] >= 1.5 * fam["height"]) if rel["role"] == "isotope" else (m["height"] >= 2.0 * fam["height"]):
                explained.append((m, rel))                          # an isotopologue, adduct or in-source fragment is weaker than the ion it comes from
        if m["dimer_of_x"]:
            evidence.append({"kind": "dimer", "name": "[2M+H]+", "mz": m["mz"], "r": m["r"], "text": f"[2M+H]+ at {m['mz']:.4f} (r={m['r']:.2f})"})
        if m["x_is_dimer_of_y"]:
            explained.append((m, {"role": "dimer", "name": "[2M+H]+", "of": "y"}))
    out["evidence"] = evidence
    if ambiguous:
        first = ambiguous[0]
        lo = min(x, first["mz"])
        out["ambiguous"] = {"readings": [f"[M+NH4]+ of an [M+H]+ at m/z {lo:.4f}", "[M+H]+ of a fragment that lost NH3"], "decided": "ammonium" if na_on_x else None,
                            "note": "[M+Na]+ pair present" if na_on_x else "decided by the [M+Na]+/[M+H]+ pair on the same neutral mass, or by the MS2 (loss of 17.0265 as base peak)"}
    if explained:
        m, rel = max(explained, key=lambda t: t[0]["height"])
        out.update(role=rel["role"], role_name=rel["name"], explained_by=m["mz"], representative_mz=m["mz"], neutral_mass=m["mz"] - PROTON)
        out["explained_text"] = {"adduct": f"{rel['name']} of an ion at m/z {m['mz']:.4f}", "isotope": f"isotopologue ({rel['name']}) of the ion at m/z {m['mz']:.4f}",
                                 "loss": (f"{rel['name']} fragment of the MSn tree: in-source from the ion at m/z {m['mz']:.4f}" if rel.get("library")
                                          else f"in-source fragment (loss of {rel['name']}) of the ion at m/z {m['mz']:.4f}"), "dimer": f"monomer of the dimer at m/z {m['mz']:.4f}"}[rel["role"]]
    return out


def families(win: Window, candidates, **kw) -> list[dict | None]:
    """`family` + `collapse` for each (mz, apex_rt) of `candidates` in one window (keywords of `family`); None where the candidate is not in it."""
    out = []
    for mz, rt in candidates:
        fam = family(win, float(mz), float(rt), **kw)
        out.append(None if fam is None else {**fam, **{"collapsed": collapse(fam)}})
    return out


def chunks(rts, span: float = 0.6, margin: float = 0.15):
    """Group candidate retention times into windows [lo, hi] that each cover their candidates +- `margin`: returns a list of (lo, hi, indices)."""
    rts = np.asarray(rts, float)
    order = np.argsort(rts, kind="stable")
    out, i = [], 0
    while i < len(order):
        start = rts[order[i]]
        j = i
        while j < len(order) and rts[order[j]] <= start + span:
            j += 1
        out.append((float(start - margin), float(rts[order[j - 1]] + margin), order[i:j]))
        i = j
    return out


# ---------------------------------------------------------------------------------------------------------------------- fragment library
class FragmentLibrary:
    """Formulas of the fragments of the parent seen in the MSn tree and in the DDA MS2 of the parent, each with the sets of atoms that can give it.

    entries[formula] = {"vec", "mz", "source": {"msn", "dda"}, "rel": strongest relative intensity (%), "idx", "w"}: idx / w = candidate
    substructures (indices into `subs`) and their weights; from the tree they are constrained by the hierarchy (WP3), from the DDA they are not."""

    def __init__(self, els, subs=None):
        self.els, self.subs = els, subs
        self.entries: dict = {}

    def __len__(self):
        return len(self.entries)

    def add(self, formula: str, mz: float, source: str, rel: float, idx=None, w=None):
        e = self.entries.get(formula)
        vec = F.vec(formula, self.els)
        if e is None:
            self.entries[formula] = {"vec": vec, "mz": float(mz), "source": {source}, "rel": float(rel), "idx": idx, "w": w}
            return
        e["source"].add(source)
        e["rel"] = max(e["rel"], float(rel))
        if idx is not None:
            if e["idx"] is None:
                e["idx"], e["w"] = idx, w
            else:                                         # seen in two places: the true atoms satisfy both constraints
                both = np.intersect1d(e["idx"], idx)
                if len(both):
                    wa = dict(zip(e["idx"].tolist(), e["w"].tolist()))
                    ww = np.array([wa[i] for i in both.tolist()])
                    e["idx"], e["w"] = both, ww / ww.sum()
                elif len(idx) < len(e["idx"]):
                    e["idx"], e["w"] = idx, w

    def masses(self) -> np.ndarray:
        return np.array([e["mz"] for e in self.entries.values()])

    def formulas(self) -> list[str]:
        return list(self.entries)

    def contains(self, vec) -> str | None:
        v = np.asarray(vec)
        for f, e in self.entries.items():
            if np.array_equal(e["vec"], v):
                return f
        return None


def build_library(tree, dda_peaks=None, min_rel: float = 1.0, node_peak_rel: float | None = None) -> FragmentLibrary:
    """Library from an `msn.MsnTree` (formulas of the nodes and of their clean peaks, with the atom sets constrained by the tree) and, if given, the
    consensus MS2 of the parent from the DDA data `dda_peaks = (mz, relative intensity in %)`: peaks of at least `min_rel` % that are a sub-formula
    of the parent ion, with atom sets from the molecular graph without constraint (weighted like in WP3). `node_peak_rel`: the clean peaks of a node
    enter only above this relative intensity (%); None (the default) = only the precursors of the nodes: measured on the real products, a library
    with all the clean peaks (300 formulas) makes the unshifted / shifted classification of the fragments nearly random and the localisation fails,
    with the precursors of the tree and the parent's DDA peaks (23 formulas) it works."""
    lib = FragmentLibrary(tree.els, tree.subs)
    for n in tree.nodes:
        if n["formula"] is None:
            continue
        idx_w = tree.cand.get(n["id"]) if tree.subs is not None else None
        if n["id"] != "MS1":
            lib.add(n["formula"], n["prec_mz"], "msn", 100.0, *(idx_w if idx_w is not None else (None, None)))
        if n["ghost"] or node_peak_rel is None:
            continue
        for j, p in enumerate(n["peaks"]):
            if p["rel"] < node_peak_rel:
                continue
            ip = tree.peak_cand.get((n["id"], j)) if tree.subs is not None else None
            lib.add(p["formula"], p["mz"], "msn", p["rel"], *(ip if ip is not None else (None, None)))
    if dda_peaks is not None:
        mz, rel = (np.asarray(v, float) for v in dda_peaks)
        top = F.vec(tree.nodes[0]["formula"], tree.els)
        for m, r in zip(mz, rel):
            if r < min_rel:
                continue
            c = tree.space.candidates(float(m), 5.0, 0.002, within=top)
            if len(c) == 0:
                continue
            vec = tree.space.G[int(c[0])]
            idx, w = tree.subs.candidates(vec) if tree.subs is not None else (None, None)
            lib.add(F.fmt(vec, tree.els), float(m), "dda", float(r), idx, w)
    return lib
