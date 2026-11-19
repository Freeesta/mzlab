"""The measurements behind «Integrate every peak shown» on low-resolution traces (mzlab_core::peakq_lrauto, `mzlab-cli peak auto-lr`), reproducible:
synthetic jagged SIM/MRM traces of tools/synthetic_data_jagged.py (seeded) plus two harder cases and extra spikes, scored against their truth.
Two sets of 760 peaks: «mild» (4 % and 8 % jitter) and «harsh» (15 % and 25 %). `python3 tools/lr_auto_misura.py [default|sweep|hr|click|all]`
prints one line per configuration: peaks found, windows holding two peaks (merged), windows with no peak (false), peaks cut in two (split), and the
area error (median / p90) of the peaks alone in their window. Every number is invented; nothing here is lab data.
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import synthetic_data_jagged as J  # noqa: E402

CLI = ROOT / "target/release/mzlab-cli"
SEEDS = range(1, 21)
# a narrow pair far apart and a small neighbour 3.6 sigma from a big peak, on top of the cases of synthetic_data_jagged.py
EXTRA = {"narrow": ([(1.5, 0.035, 1.0e4, 0.0), (3.5, 0.04, 3.0e3, 0.0)], 2),
         "unequal": ([(2.0, 0.06, 4.0e4, 0.0), (2.0 + 3.6 * 0.06, 0.06, 6.0e3, 0.0)], 2)}
# (jitter, extra spike size in noise units) of each set
SETS = {"mild": ((0.04, 8), (0.08, 15)), "harsh": ((0.15, 15), (0.25, 15))}


def files(tmp: Path, name: str) -> list:
    """(csv, truth) of every trace of the set: each case x seed x jitter, with 1 % of the points raised by a spike (seed 1000 + seed)."""
    for k, (peaks, n) in EXTRA.items():
        J.CASES[k], J.EXPECTED[k] = peaks, n
    out = []
    for jit, spk in SETS[name]:
        J.JITTER = jit
        for case in J.CASES:
            for seed in SEEDS:
                t, y, truth = J.trace(case, seed)
                sp = np.random.default_rng(1000 + seed).random(t.size) < 0.01
                y = y.copy()
                y[sp] += spk * 1.5 * np.sqrt(np.maximum(y[sp], 1))
                out.append((J.write_csv(tmp / f"{name}-{jit}-{case}-{seed}.csv", t, y), truth))
    J.JITTER = 0.04
    return out


def flag(kv: str) -> list:
    """A `NAME=VALUE` setting as CLI arguments: modes and the unit are typed flags, numbers stay `--set`."""
    k, v = kv.split("=", 1)
    typed = {"area_unit": "--area-unit", "baseline": "--baseline", "lr_split": "--split"}
    return [typed[k], v] if k in typed else ["--set", kv]


def score(fs: list, cmd: str, sets: list) -> str:
    """One line: the windows of `mzlab-cli CMD --set ...` on every file against the true apexes and areas."""
    def one(x):
        f, tr = x
        r = json.loads(subprocess.run([str(CLI), *cmd.split(), str(f), *[a for s in ["area_unit=min", *sets] for a in flag(s)]], capture_output=True, text=True, check=True).stdout)
        ap, w = tr["apex"], [(p["t0"], p["t1"], p["area"]) for p in r["peaks"]]
        inside = [[a for a in ap if lo <= a <= hi] for lo, hi, _ in w]
        err = [abs(k[0][2] / ar - 1) for a, ar in zip(ap, tr["area"]) for k in [[x for x in w if x[0] <= a <= x[1]]] if len(k) == 1]
        return (sum(any(lo <= a <= hi for lo, hi, _ in w) for a in ap), sum(len(c) > 1 for c in inside), sum(not c for c in inside),
                sum(sum(lo <= a <= hi for lo, hi, _ in w) > 1 for a in ap), err, r.get("params", {}).get("half"))
    with ThreadPoolExecutor(8) as ex:
        res = list(ex.map(one, fs))
    found, merged, false, split = (sum(r[i] for r in res) for i in range(4))
    err = [e for r in res for e in r[4]]
    exp = sum(len(tr["apex"]) for _, tr in fs)
    half = sorted({r[5] for r in res if r[5] is not None})
    return (f"found {found}/{exp}  merged {merged}  false {false}  split {split}  area |err| median {np.median(err) * 100:.1f} % p90 "
            f"{np.percentile(err, 90) * 100:.1f} %" + (f"  half used {half[0]}-{half[-1]}" if half else ""))


def click(tmp: Path) -> str:
    """The single click before this route (`peak edges --at APEX --mrm` at every true apex) on the 150 peaks of the base cases, 10 seeds."""
    found = over = 0
    for case in J.CASES:
        if case in EXTRA:
            continue
        for seed in range(1, 11):
            t, y, tr = J.trace(case, seed)
            f = J.write_csv(tmp / "click.csv", t, y)
            w = [e for a in tr["apex"] for e in [json.loads(subprocess.run([str(CLI), "peak", "edges", str(f), "--at", str(a), "--mrm"], capture_output=True,
                                                                         text=True, check=True).stdout)["edges"]] if e]
            found += sum(any(lo <= a <= hi for lo, hi in w) for a in tr["apex"])
            over += sum(w[i][0] < w[j][1] - 1e-9 and w[j][0] < w[i][1] - 1e-9 for i in range(len(w)) for j in range(i + 1, len(w)))
    return f"LR click   base 150 peaks: apex inside a window {found}, pairs of windows that overlap {over}"


def main(what: str) -> None:
    if not CLI.exists():
        subprocess.run(["cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-cli"], cwd=ROOT, check=True)
    with tempfile.TemporaryDirectory() as d:
        tmp = Path(d)
        fs = {k: files(tmp, k) for k in SETS}
        if what in ("default", "all"):
            for k in SETS:
                print(f"LR auto    {k:5s} defaults: {score(fs[k], 'peak auto-lr', [])}", flush=True)
        if what in ("hr", "all"):
            for k in SETS:
                print(f"HR auto    {k:5s} peak auto baseline=edges: {score(fs[k], 'peak auto', ['baseline=edges'])}", flush=True)
        if what in ("sweep", "all"):
            for k in SETS:
                for half in (0, 2, 3, 4, 5):
                    print(f"LR auto    {k:5s} lr_half={half} (0 = from the peaks): {score(fs[k], 'peak auto-lr', [f'lr_half={half}'])}", flush=True)
                for sn in (3, 5, 8):
                    for val in (5, 10, 15, 20):
                        print(f"LR auto    {k:5s} lr_sn={sn} lr_valley={val}: {score(fs[k], 'peak auto-lr', [f'lr_sn={sn}', f'lr_valley={val}'])}", flush=True)
        if what in ("click", "all"):
            print(click(tmp), flush=True)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "default")
