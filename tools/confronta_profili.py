#!/usr/bin/env python3
"""Run the same mzflow plan with two mzlab-cli binaries (e.g. target/release and target/cli) and check that every output file is byte-identical, printing the wall time of each.
Usage: confronta_profili.py PLAN.json IN_DIR BIN_A BIN_B [--workers N]. Used to prove that the `cli` profile only changes speed."""
import argparse, subprocess, sys, tempfile, time
from pathlib import Path

# Files whose bytes legitimately change between runs (timings, heap peaks).
VARIABLE = {"prov.json", "corsa.json"}


def run(binary, plan, indir, out, workers):
    t = time.perf_counter()
    subprocess.run([binary, "flow", "run", plan, "--in", indir, "--out", out, "--workers", str(workers)],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return time.perf_counter() - t


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("plan"); ap.add_argument("indir"); ap.add_argument("bin_a"); ap.add_argument("bin_b")
    ap.add_argument("--workers", type=int, default=1)
    a = ap.parse_args()
    with tempfile.TemporaryDirectory() as tmp:
        outs, times = [], []
        for i, b in enumerate((a.bin_a, a.bin_b)):
            out = str(Path(tmp) / f"o{i}")
            times.append(run(b, a.plan, a.indir, out, a.workers))
            outs.append(Path(out))
        files = lambda d: {p.relative_to(d).as_posix() for p in d.rglob("*") if p.is_file() and p.name not in VARIABLE}
        fa, fb = files(outs[0]), files(outs[1])
        bad = sorted(fa ^ fb) + sorted(f for f in fa & fb if (outs[0] / f).read_bytes() != (outs[1] / f).read_bytes())
    print(f"{a.bin_a}: {times[0]:.2f} s | {a.bin_b}: {times[1]:.2f} s | files compared: {len(fa & fb)}")
    if bad:
        print("DIFFERENT:", *bad[:20], sep="\n  ")
        return 1
    print("identical")
    return 0


if __name__ == "__main__":
    sys.exit(main())
