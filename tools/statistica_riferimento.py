"""Reference answers of the POST routes `api/stat/profile|blank|reference|pca` for crates/mzlab-core/tests/stat_parity.rs.

    python3 tools/statistica_riferimento.py OUT.json

Writes [{path, body, status, result}]: invented tables (fixed seeds: lognormal areas, zeros, ties, adduct and neutral-loss pairs), answered by
`api.dispatch` (the JSON of the page's requests). No data of the laboratory, no compound names.
"""
import io
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.api import dispatch  # noqa: E402


def table(rng, nf: int, nfile: int, times, roles) -> tuple[list, list, list]:
    """features x files areas with time courses (rise, fall, rise-and-fall, flat), blank noise, zeros as null and integer ties."""
    m = np.zeros((nf, nfile))
    for f in range(nf):
        kind = f % 5
        base = float(np.exp(rng.normal(10.5, 1.5)))
        for j in range(nfile):
            t = times[j] or 0.0
            if roles[j] == "blank":
                m[f, j] = base * (0.02 if kind < 3 else 1.0) * rng.uniform(0.5, 1.5) if rng.random() > 0.2 else 0.0
            else:
                shape = [1 + t / 30, np.exp(-t / 40), np.exp(-((t - 30) / 20) ** 2), 1.0, 0.0][kind]
                m[f, j] = base * shape * rng.uniform(0.8, 1.2) if rng.random() > 0.08 else 0.0
    m = np.round(m / 7) * 7                                    # ties
    rows = [[None if v == 0 and k % 2 else float(v) for k, v in enumerate(r)] for r in m]
    mz = (rng.uniform(150, 600, nf)).round(4)
    rt = rng.uniform(1, 12, nf).round(3)
    for i in range(0, nf - 12, 6):                             # Na/H adduct pairs and water losses at the same RT
        mz[i + 1], rt[i + 1] = mz[i] + 21.981942, rt[i]
        mz[i + 2], rt[i + 2] = mz[i] - 18.010565, rt[i] + 0.04
    return rows, mz.tolist(), rt.tolist()


def case(path: str, body: dict) -> dict:
    raw = json.dumps(body).encode()
    status, _, out = dispatch(None, "POST", path, {}, io.BytesIO(raw), len(raw))[:3]
    return {"path": path, "body": body, "status": status, "result": json.loads(out)}


def main(argv) -> int:
    rng = np.random.default_rng(20261012)
    cases = []
    roles = ["blank", "blank", "sample", "sample", "sample", "sample", "sample", "sample", "sample", "sample", "sample", "qc"]
    times = [None, None, 0, 0, 0, 15, 15, 15, 60, 60, 60, None]
    groups = [None, None, "t0", "t0", "t0", "t15", "t15", "t15", "t60", "t60", "t60", None]
    rows, mz, rt = table(rng, 90, len(roles), times, roles)
    # profile
    pts = [{"file": f"f{k}.mzML", "time": t, "rt": 5.5, "area": None if k == 4 else 1000.0 * (k + 1) % 7000, "height": 90.0 * (k + 2)} for k, t in enumerate([30, 0, None, 60, 15, 0, 120, 15])]
    cases.append(case("/api/stat/profile", {"points": pts, "metric": "area"}))
    cases.append(case("/api/stat/profile", {"points": pts, "metric": "height", "istd": {p["file"]: {"area": 500.0 * (1 + k % 3) if k != 2 else 0, "height": 40.0 + k} for k, p in enumerate(pts)}}))
    cases.append(case("/api/stat/profile", {"points": pts, "metric": "volume"}))
    # blank
    for stat in ("max", "mean"):
        for factor, md in ((3, 1), (10, 2), (0, 3)):
            cases.append(case("/api/stat/blank", {"matrix": rows, "roles": roles, "factor": factor, "stat": stat, "min_detect": md}))
    cases.append(case("/api/stat/blank", {"matrix": rows, "roles": ["sample"] * len(roles)}))
    cases.append(case("/api/stat/blank", {"matrix": rows, "roles": ["blank"] * len(roles)}))
    cases.append(case("/api/stat/blank", {"matrix": rows, "roles": roles, "stat": "median"}))
    # reference
    sample_roles = ["blank" if r == "blank" else ("qc" if r == "qc" else "sample") for r in roles]
    base = {"matrix": rows, "roles": sample_roles, "times": times, "groups": groups}
    cases.append(case("/api/stat/reference", base))
    cases.append(case("/api/stat/reference", {**base, "mz": mz, "rt": rt}))
    cases.append(case("/api/stat/reference", {**base, "reference": [2, 3, 4], "factor": 5, "stat": "mean", "min_detect": 2, "pseudo": 2500.0}))
    cases.append(case("/api/stat/reference", {**base, "groups": None, "floor": 50.0}))
    cases.append(case("/api/stat/reference", {**base, "reference": [0, 99]}))
    cases.append(case("/api/stat/reference", {**base, "roles": ["sample"] * len(roles), "times": [5] * len(roles)}))
    cases.append(case("/api/stat/reference", {**base, "times": times[:-1]}))
    # pca
    cases.append(case("/api/stat/pca", {"matrix": rows, "roles": roles, "groups": groups}))
    for scaling, transform in (("auto", "log"), ("none", "none"), ("pareto", "none")):
        cases.append(case("/api/stat/pca", {"matrix": rows, "roles": roles, "groups": groups, "scaling": scaling, "transform": transform, "ncomp": 4, "min_intensity": 500.0, "min_samples": 4}))
    cases.append(case("/api/stat/pca", {"matrix": rows, "roles": roles, "factor": 5, "stat": "mean", "ncomp": 3}))
    cases.append(case("/api/stat/pca", {"matrix": rows, "roles": roles[:7] + ["dark"] * 5, "groups": groups}))
    cases.append(case("/api/stat/pca", {"matrix": rows, "roles": roles, "scaling": "robust"}))
    cases.append(case("/api/stat/pca", {"matrix": [], "roles": roles}))
    big, _, _ = table(rng, 400, 14, [None, None] + [0, 0, 0, 20, 20, 20, 40, 40, 40, 80, 80, 80], ["blank", "blank"] + ["sample"] * 12)
    cases.append(case("/api/stat/pca", {"matrix": big, "roles": ["blank", "blank"] + ["sample"] * 12, "groups": [None, None] + ["a"] * 3 + ["b"] * 3 + ["c"] * 3 + ["d"] * 3}))
    open(argv[0], "w", encoding="utf-8").write(json.dumps(cases))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
