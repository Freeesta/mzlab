# TPMINE-PRIVATE
"""The window-by-window search of unexpected ions that Experiment.discover replaced (one XIC per window and file, in Python). Kept only to prove
that the vectorised search finds the same ions and to measure the speed-up: legacy_discover(experiment) has the same effect as discover()."""
import numpy as np


def legacy_discover(self):
    """Ions that rise in time, are absent at t0 / in the blank, and do not correspond to any candidate: 'unexpected ions'."""
    t = self._table(self.full[0])
    lo, hi = float(t.mz.min()), float(t.mz.max())
    step = self.tol
    centers = np.arange(lo + step, hi - step, step)
    known = [c["mz_x"] for c in self.entries]
    n0 = len(self.entries)
    found: list[dict] = []
    uid = 100000
    sk, mp = int(self.thr["peak"].get("smooth_points", 3)), int(self.thr["peak"]["min_points"])
    for i, mz in enumerate(centers):
        if i % 50 == 0:
            self.progress("Searching for unexpected ions...", i / len(centers))
        if any(abs(mz - m) <= self.tol for m in known):
            continue
        hit = False
        for k, x in enumerate(self.full):
            if x.type != "sample" or not x.time:
                continue
            rt, y = self.trace(k, mz)
            if len(y) < 10 or y.max() <= 0:
                continue
            nz = y[y > 0]
            if y.max() < 8 * float(np.median(nz)) if len(nz) else True:
                continue
            hit = True
            break
        if not hit:
            continue
        uid += 1
        e = {"id": uid, "kind": "unexpected", "name": f"unexpected ion m/z {mz - self.offset:.2f}", "alternatives": [],
             "delta": "", "formula": "", "formula_dict": None, "neutral_mass": None, "mz": mz - self.offset, "mz_x": float(mz),
             "steps": 1, "adduct": self.adduct, "delta_dict": None}
        a = self._analyse_base(e)
        if a["label"] in ("forte", "possibile") and a["ref_rt"] is not None and self._rises(a):
            e["_area"] = max([r["area"] for r in a["rows"] if r["detected"]] or [0.0])
            found.append(e)
            self.entries.append(e)       # temporary: lets the isotope check see them
    # keep the best of neighbouring windows (same peak seen by two windows) and drop isotope peaks
    found.sort(key=lambda e: -e["_area"])
    kept: list[dict] = []
    for e in found:
        a = self._analyse_base(e)
        if any(abs(e["mz_x"] - f["mz_x"]) <= 1.2 * self.tol and abs(a["ref_rt"] - self._base[f["id"]]["ref_rt"]) <= self.rt_tol for f in kept):
            continue
        kept.append(e)
    self.entries = self.entries[:n0] + kept
    for e in kept:                     # the grid centre is arbitrary: report the observed centroid of the ion
        obs = self.refine(e)
        if obs is not None:
            e["mz_x"], e["mz"] = obs, obs - self.offset
            e["name"] = f"unexpected ion m/z {e['mz']:.1f}"
    kept = [e for e in kept if not any(abs(e["mz_x"] - m) <= self.tol for m in known)]   # a window next to a candidate can catch the candidate's own ion
    good = []
    for e in kept:
        a = self.analyse(e)
        if a["label"] in ("forte", "possibile"):
            good.append(e)
    good.sort(key=lambda e: (-(self._final[e["id"]]["score"] or 0), -e["_area"]))
    self.entries = self.entries[:n0] + good[:int(self.s["max_unexpected"])]
    return len(self.entries) - n0
