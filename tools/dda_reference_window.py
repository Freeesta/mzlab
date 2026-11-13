"""Reference of `/api/dda_window` (the Rust route is `Routes::dda_window`, crates/mzlab-core/src/ddafinestra.rs): the MS2 started from the survey
scans of a time window, grouped by precursor within the tolerance of the MS2 profile (ppm of a high-resolution profile, else 0.5 Da, as `tolOf`
in dda.js). Groups: the most intense precursor first; members in order of acquisition. Parity test: crates/mzlab-core/tests/hr_parity.rs, through
`tools/hr_reference.py`, which serves it as `/api/dda_window` of `mzlab.api` while it writes the reference answers (the package itself has no such route)."""


def dda_window(run, prof: dict, r0: float, r1: float) -> dict:
    lo, hi = min(r0, r1), max(r0, r1)
    ms1 = {s.index for s in run.scans if s.level == 1 and lo <= float(s.rt) <= hi}
    d = int(prof["dec"]) + 1
    ppm = float(prof["tol"]) if prof.get("hr") and prof.get("unit") == "ppm" else 0.0
    tol = (lambda m: m * ppm * 1e-6) if ppm > 0 else (lambda m: 0.5)
    key = lambda s: round(float(s.precursor), d) if s.precursor else round(float(sum(s.iso) / 2), d) if s.iso else None
    kids = sorted(((key(s), s.index, s) for s in run.scans if s.level > 1 and s.parent in ms1 and key(s) is not None), key=lambda t: (t[0], t[1]))
    groups = []
    for m, _, s in kids:
        if groups and m - groups[-1]["prec"] <= tol(groups[-1]["prec"]):
            groups[-1]["_s"].append(s)
        else:
            groups.append({"prec": m, "_s": [s]})
    out = []
    for g in groups:
        ss = sorted(g["_s"], key=lambda s: s.index)
        out.append({"prec": g["prec"], "sid": [s.index for s in ss], "rt": [round(float(s.rt), 4) for s in ss], "pint": [s.pint for s in ss]})
    top = lambda g: max((p for p in g["pint"] if p is not None), default=-1.0)
    out.sort(key=lambda g: (-top(g), g["prec"]))
    return {"ms1": len(ms1), "n": len(kids), "groups": out}
