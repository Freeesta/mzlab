#!/usr/bin/env python3
"""Times the hot paths of the `hr` .wasm with and without extra wasm-opt passes, in Node, on the synthetic series of tools/feat_mirato.py.

    python3 tools/bench_hr.py [--extra "--merge-similar-functions"] [--reps 7] [--max-slowdown 3]

Builds the `hr` module once (the cargo output is the same for both), runs wasm-bindgen (nodejs target) and tools/wasm_opt.sh twice (`@motore` alone, then
`@motore` plus --extra, or the named set when --extra starts with `@`, e.g. `@estesi`), alternates the two builds ABAB in the same process, prints median and spread per path and the ratio, and checks that the SHA-256 of
every output is the same. Exit 1 when a median is slower than --max-slowdown percent or an output differs. No dependency beyond cargo, wasm-bindgen, node.
"""
import argparse
import json
import os
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FEATURES = "detect,proofs,finder,finder_edge,finder_msn,flow_ms2"


def run(*a, **k):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8", **k).stdout


def build_pkg(work, name, extra):
    """The pkg of one variant: raw cargo wasm -> wasm-bindgen (nodejs) -> wasm-opt through the site's script."""
    pkg = work / name
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    run("tools/wasm_opt.sh", str(pkg / "mzlab_feat_wasm_bg.wasm"), "99999999", "hr", *(extra.split() if extra.startswith("@") else ["@motore", *extra.split()]))
    return pkg


def timings(pkg, reps, idx):
    return json.loads(run("node", "tools/bench_hr.js", str(pkg), str(reps), *idx).splitlines()[-1])


def spread(v):
    q = statistics.quantiles(v, n=10)
    return (q[-1] - q[0]) / statistics.median(v) * 100


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--extra", default="--merge-similar-functions")
    ap.add_argument("--reps", type=int, default=7)
    ap.add_argument("--rounds", type=int, default=7, help="ABAB rounds; the samples of all rounds are pooled")
    ap.add_argument("--max-slowdown", type=float, default=3.0)
    a = ap.parse_args()
    run("python3", "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(work := Path(tempfile.mkdtemp()) / "ser"))
    idx = sorted(str(p) for p in work.glob("*.mzidx"))
    run("tools/cargo_wasm.sh", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", FEATURES)
    pkgs = {"base": build_pkg(work.parent, "base", ""), "variant": build_pkg(work.parent, "variant", a.extra)}
    size = {k: (p / "mzlab_feat_wasm_bg.wasm").stat().st_size for k, p in pkgs.items()}
    runs = {"base": [], "variant": []}
    for _ in range(a.rounds):
        for k in ("base", "variant", "variant", "base"):
            runs[k].append(timings(pkgs[k], a.reps, idx))
    bad = 0
    print(f"wasm bytes: base {size['base']} variant {size['variant']}")
    for name in runs["base"][0]["paths"]:
        m = {k: [t for r in runs[k] for t in r["paths"][name]] for k in runs}
        med = {k: statistics.median(v) for k, v in m.items()}
        ratio = (med["variant"] / med["base"] - 1) * 100
        same = all(r["sha"][name] == runs["base"][0]["sha"][name] for k in runs for r in runs[k])
        bad += (ratio > a.max_slowdown) + (not same)
        sp = {k: spread(v) for k, v in m.items()}   # (p90 - p10) / median, percent: the noise of the machine on this path
        print(f"{name:14} base {med['base']:8.2f} ms (spread {sp['base']:.1f} %)  variant {med['variant']:8.2f} ms (spread {sp['variant']:.1f} %)  "
              f"{ratio:+.1f} %  output {'identical' if same else 'DIFFERENT'}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
