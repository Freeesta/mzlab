"""Per-stage diagnosis of the area of the feature table against the true area and the XIC area, on `tools/area_synthetic.py` files.

    python3 tools/area_diagnosis.py DIR [--cli PATH] [--set INSTANCE.NAME=VALUE ...] [--scen a,b]

runs `mzlab-cli flow run` (index + features.detect) on every DIR/<scenario>.mzML and, for every ion of DIR/truth.json, takes its largest row
and walks the area from the truth to the table one stage at a time, each a log2 ratio to the stage before (median and 95th percentile of
the signed values over the ions of a scenario):
  sampling   XIC (+-10 ppm, trapezoid on the real RT) over the whole peak / true area
  bounds     the same XIC over the scans of the row / the whole peak
  rule       sum(y) x median scan interval over the row / trapezoid
  floor      only the points above the intensity floor / all points
  mz window  +-6 ppm (the trace of `area_from=window`) / +-10 ppm
  table      the area of the row / the +-6 ppm sum above the floor (channels, stitching, pieces)
  vs XIC     the area of the row / the XIC over the row (trapezoid, +-10 ppm): the number a user sees when comparing to the chromatogram.
Python here only reads results; the areas are computed by the Rust engine.
"""
import json
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
PLAN = {"format": "mzflow/2", "inputs": [{"name": "a.mzML", "type": "raw"}], "steps": [
    {"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:a.mzML"]}},
    {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}}]}
DTYPE = {"f64": "<f8", "f32": "<f4", "u32": "<u4", "u16": "<u2", "u8": "u1", "i8": "i1"}
FLOOR, XIC_PPM, WIN_PPM = 2e4, 10.0, 6.0
STAGES = ("sampling", "bounds", "rule", "floor", "mz window", "table")


def read_features(path: Path) -> dict:
    data = path.read_bytes()
    n = struct.unpack("<Q", data[8:16])[0]
    meta = json.loads(data[16:16 + n])
    start = -(-(16 + n) // 64) * 64
    return {a["name"]: np.frombuffer(data, dtype=np.dtype(DTYPE[a["dtype"]]), count=a["count"], offset=start + a["offset"])
            for a in meta["arrays"]}


def run(cli: Path, mzml: Path, sets: list[str]) -> dict:
    with tempfile.TemporaryDirectory() as d:
        d = Path(d)
        (d / "data").mkdir()
        (d / "data" / "a.mzML").symlink_to(mzml.resolve())
        (d / "plan.json").write_text(json.dumps(PLAN))
        args = [str(cli), "flow", "run", str(d / "plan.json"), "--in", str(d / "data"), "--out", str(d / "out")]
        for s in sets:
            args += ["--set", s]
        subprocess.run(args, check=True, capture_output=True)
        f = next(p for p in (d / "out").rglob("*.mzfeatf"))
        return {k: v.copy() for k, v in read_features(f).items()}


def trace(pts: dict, mz: float, ppm: float, floor: float = 0.0) -> np.ndarray:
    keep = (np.abs(pts["mz"] - mz) <= ppm * 1e-6 * mz) & (pts["it"] >= floor)
    return np.bincount(pts["scan"][keep], weights=pts["it"][keep], minlength=len(pts["rt"]))


def trapz(y: np.ndarray, t: np.ndarray) -> float:
    return float(np.sum(0.5 * (y[1:] + y[:-1]) * np.diff(t))) if len(y) > 1 else 0.0


def stages(ion: dict, pts: dict, f: dict, dt: float) -> dict | None:
    """The areas of one ion stage by stage, from its largest row; None when no row has its m/z and RT."""
    t = pts["rt"] * 60
    a, z = ion["comps"][0], ion["comps"][-1]
    lo_t, hi_t = a[0] - 6 * a[1] / 60, z[0] + 6 * z[1] / 60
    near = np.where((np.abs(f["mz"] - ion["mz"]) <= 10e-6 * ion["mz"]) & (f["rt"] >= lo_t) & (f["rt"] <= hi_t))[0]
    if len(near) == 0:
        return None
    r = near[np.argmax(f["area"][near])]
    lo, hi = int(f["scan_lo"][r]), int(f["scan_hi"][r])
    whole = (pts["rt"] >= lo_t) & (pts["rt"] <= hi_t)
    x10, x6f = trace(pts, ion["mz"], XIC_PPM), trace(pts, ion["mz"], WIN_PPM, FLOOR)
    s = slice(lo, hi + 1)
    return {"true": ion["area"], "sampling": trapz(x10[whole], t[whole]), "bounds": trapz(x10[s], t[s]),
            "rule": float(x10[s].sum() * dt), "floor": float(trace(pts, ion["mz"], XIC_PPM, FLOOR)[s].sum() * dt),
            "mz window": float(x6f[s].sum() * dt), "table": float(f["area"][r]), "rows": len(near), "all rows": float(f["area"][near].sum())}


def summary(rows: list[dict]) -> dict:
    out, prev = {}, "true"
    for st in STAGES:
        v = np.log2([r[st] / r[prev] for r in rows])
        out[st] = (float(np.median(v)), float(np.percentile(np.abs(v), 95)))
        prev = st
    v = np.log2([r["table"] / r["bounds"] for r in rows])
    out["vs XIC"] = (float(np.median(v)), float(np.percentile(np.abs(v), 95)))
    v = np.log2([r["table"] / r["true"] for r in rows])
    out["vs true"] = (float(np.median(v)), float(np.percentile(np.abs(v), 95)))
    out["split"] = sum(r["rows"] > 1 for r in rows)
    return out


def main(argv) -> int:
    d = Path(argv[0])
    cli = Path(argv[argv.index("--cli") + 1]) if "--cli" in argv else ROOT / "target" / "release" / "mzlab-cli"
    sets = [argv[i + 1] for i, a in enumerate(argv) if a == "--set"]
    truth = json.loads((d / "truth.json").read_text())
    scen = argv[argv.index("--scen") + 1].split(",") if "--scen" in argv else list(truth["scenarios"])
    print("signed log2 per stage: median / 95th pct of |.|;", " ".join(sets) or "defaults")
    print(f"{'scenario':9}" + "".join(f"{s:>16}" for s in (*STAGES, "vs XIC", "vs true")) + "  found split")
    for name in scen:
        pts = dict(np.load(d / f"{name}.npz"))
        f = run(cli, d / f"{name}.mzML", sets)
        dt = float(np.median(np.diff(pts["rt"]))) * 60
        rows = [r for r in (stages(i, pts, f, dt) for i in truth["ions"]) if r]
        s = summary(rows)
        print(f"{name:9}" + "".join(f"{s[k][0]:>+8.3f}/{s[k][1]:<7.3f}" for k in (*STAGES, "vs XIC", "vs true"))
              + f"  {len(rows):>2}/{len(truth['ions'])} {s['split']:>3}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
