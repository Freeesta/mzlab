#!/usr/bin/env python3
"""Speed of `mzlab-cli quant targets` on a large synthetic set: unit-resolution MS1 files (centroid scans of whole-ish ion counts, a target
ion present in most files, a random background of other ions), a targets table, then wall time, CPU time, peak RSS and passes per file for
each worker count, from mzML (index built in the run) and from `.mzidx` (`--cache`). With `--per-target K` it also times the old route, one
`trace xic | peak auto-lr` pair of processes per target, on K targets of one indexed file, and extrapolates it to the whole set.

usage: python3 tools/bench_quant_targets.py --cli target/release/mzlab-cli [--files 245] [--targets 250] [--scans 1200] [--peaks 150]
       [--workers 1,4] [--per-target 20] [--dir DIR] [--keep]
Prints one JSON object on stdout; progress on stderr. Synthetic data only (fixed seed), written to DIR (default a temporary folder).
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_sintetici import _spec  # noqa: E402
from dati_sintetici_mzml import _write  # noqa: E402

STEP_MIN = 1.0 / 60.0  # one scan per second


def targets(n: int, rt_end: float, rng) -> list[dict]:
    mz = rng.choice(np.arange(100, 1000), size=n, replace=False).astype(float)
    centre = rng.uniform(1.0, rt_end - 1.0, size=n)
    return [{"id": f"t{i:03d}", "mz": float(mz[i]), "rt": float(round(centre[i], 3))} for i in range(n)]


def table(ts: list[dict], path: Path) -> None:
    rows = ["id,mz,rt_min,rt_max,polarity,role"]
    for i, t in enumerate(ts):
        role = "standard" if i == 0 else ""
        rows.append(f"{t['id']},{t['mz']},{round(t['rt'] - 0.3, 3)},{round(t['rt'] + 0.3, 3)},+,{role}")
    path.write_text("\n".join(rows) + "\n", encoding="utf-8")


def one_file(path: Path, ts: list[dict], scans: int, peaks: int, seed: int) -> None:
    rng = np.random.default_rng(seed)
    present = rng.random(len(ts)) < 0.8
    height = rng.lognormal(6.0, 1.5, size=len(ts))
    # the standard row (the first, where --offset auto measures) is in every file and tall
    present[0], height[0] = True, 5e4
    t_mz = np.array([t["mz"] for t in ts])
    t_rt = np.array([t["rt"] for t in ts])
    specs = []
    for i in range(scans):
        rt = i * STEP_MIN
        bg_mz = np.round(rng.uniform(100.0, 1000.0, size=peaks), 1)
        bg_y = np.round(rng.exponential(30.0, size=peaks))
        g = present * height * np.exp(-((rt - t_rt) ** 2) / (2 * 0.05 ** 2))
        y = np.round(g + rng.poisson(1.0, size=len(ts)))
        keep = y > 0
        mz = np.concatenate([bg_mz, t_mz[keep]])
        it = np.concatenate([bg_y, y[keep]])
        o = np.argsort(mz, kind="stable")
        specs.append(_spec(i, rt, mz[o], it[o]))
    _write(path, specs, [])


def usage(cmd: list[str]) -> dict:
    """Runs `cmd`: exit code, stdout, stderr, wall s, CPU s (user + system of the child) and peak RSS MB."""
    import resource
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.perf_counter()
    p = subprocess.run(cmd, capture_output=True)
    r = {"wall_s": round(time.perf_counter() - t0, 2), "code": p.returncode, "stdout": p.stdout,
         "stderr": p.stderr.decode("utf-8", "replace")}
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    r["cpu_s"] = round((after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime), 2)
    # ru_maxrss of the children is the largest child so far: each measured run is started from a fresh helper process below
    r["max_rss_mb"] = round(after.ru_maxrss / 1024.0, 1)
    return r


def measured(cmd: list[str]) -> dict:
    """`usage` in a fresh Python process, so the peak RSS is that of this run alone."""
    here = Path(__file__).resolve()
    code = ("import json,sys;sys.path.insert(0,%r);import %s as b;"
            "r=b.usage(%r);r['stdout_bytes']=len(r.pop('stdout'));print(json.dumps(r))") % (str(here.parent), here.stem, cmd)
    out = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def passes(stderr: str, files: int) -> float:
    lines = [ln for ln in stderr.splitlines() if ln.startswith("[") and "/" in ln.split("]")[0]]
    return round(len(lines) / files, 2)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--cli", required=True)
    ap.add_argument("--files", type=int, default=245)
    ap.add_argument("--targets", type=int, default=250)
    ap.add_argument("--scans", type=int, default=1200)
    ap.add_argument("--peaks", type=int, default=150)
    ap.add_argument("--workers", default="1,4")
    ap.add_argument("--per-target", type=int, default=0)
    ap.add_argument("--dir")
    ap.add_argument("--keep", action="store_true")
    a = ap.parse_args()
    cli = str(Path(a.cli).resolve())
    root = Path(a.dir) if a.dir else Path(tempfile.mkdtemp(prefix="bench_quant_"))
    runs = root / "runs"
    runs.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(7)
    ts = targets(a.targets, a.scans * STEP_MIN, rng)
    table(ts, root / "targets.csv")
    t0 = time.perf_counter()
    for k in range(a.files):
        f = runs / f"S_{k + 1:03d}.mzML"
        if not f.exists():
            one_file(f, ts, a.scans, a.peaks, 100 + k)
        if (k + 1) % 25 == 0:
            print(f"made {k + 1}/{a.files} files", file=sys.stderr)
    made = time.perf_counter() - t0
    size = sum(p.stat().st_size for p in runs.iterdir())
    base = [cli, "quant", "targets", "--targets", str(root / "targets.csv"), str(runs), "--da", "0.5", "--offset", "auto"]
    out: dict = {"schema": "bench-quant-targets/1", "files": a.files, "targets": a.targets, "scans_per_file": a.scans,
                 "peaks_per_scan": a.peaks + a.targets, "rows": a.files * a.targets, "mzml_mb": round(size / 2 ** 20, 1),
                 "made_s": round(made, 1), "cores": os.cpu_count(), "runs": []}
    tables = set()
    cache = root / "cache"
    for w in [int(x) for x in a.workers.split(",")]:
        for source in ("mzML", "mzidx"):
            cmd = base + ["--workers", str(w)]
            if source == "mzidx":
                # the first run of this worker count filled the cache: every index is read back, none built
                cmd += ["--cache", str(cache)]
            else:
                shutil.rmtree(cache, ignore_errors=True)
            if source == "mzidx" and not cache.exists():
                measured(cmd)
            print(f"run: {w} workers from {source}", file=sys.stderr)
            r = measured(cmd + ["--out", str(root / f"t_{w}_{source}.csv")])
            if r["code"] != 0:
                print(r["stderr"], file=sys.stderr)
                return 1
            tables.add((root / f"t_{w}_{source}.csv").read_bytes())
            out["runs"].append({"workers": w, "source": source, "wall_s": r["wall_s"], "cpu_s": r["cpu_s"],
                                "max_rss_mb": r["max_rss_mb"], "passes_per_file": passes(r["stderr"], a.files)})
    out["same_bytes"] = len(tables) == 1
    if a.per_target:
        # the index of the first file comes from the cache the runs above filled, as the hand-written disk cache gave it
        one = [str(runs / "S_001.mzML"), "--cache", str(cache)]
        t0 = time.perf_counter()
        for t in ts[: a.per_target]:
            x = subprocess.run([cli, "trace", "xic", *one, "--mz", str(t["mz"]), "--da", "0.5", "--format", "csv"],
                               capture_output=True, check=True).stdout
            subprocess.run([cli, "peak", "auto-lr", "-", "--from", str(t["rt"] - 0.3), "--to", str(t["rt"] + 0.3)],
                           input=x, capture_output=True, check=True)
        each = (time.perf_counter() - t0) / a.per_target
        out["per_target_route"] = {"targets_timed": a.per_target, "s_per_target_and_file": round(each, 4),
                                   "processes": 2 * a.files * a.targets,
                                   "extrapolated_s": round(each * a.files * a.targets, 1),
                                   "note": "index read from --cache, not built"}
    if not a.keep:
        shutil.rmtree(root, ignore_errors=True)
    print(json.dumps(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
