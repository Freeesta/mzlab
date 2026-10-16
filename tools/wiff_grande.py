"""A synthetic SCIEX .wiff + .wiff.scan pair of a realistic size, to measure the loading of the browser (tests_e2e/e2e_wiff_carico.py) without laboratory data.

    python3 tools/wiff_grande.py OUT [--name grande] [--samples 6] [--scans 1300] [--points 1500] [--seed 7]

Writes OUT/<name>.wiff and OUT/<name>.wiff.scan in the layout of the 3200 QTRAP (the encoder and the container of tools/wiff_riferimento.py): `--samples` samples of `--scans` full scans each,
`--points` non-zero bins per scan (a chromatographic peak and noise; 3333 or more = every bin, a profile scan). With the defaults the .wiff.scan is about 20 MB, like a run of 20-60 min of the laboratory.
"""
import argparse
import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wiff_riferimento as wr  # noqa: E402

BINS = 3333  # the bins of the first range of the block (100.02-300 by 0.06)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--name", default="grande")
    ap.add_argument("--samples", type=int, default=6)
    ap.add_argument("--scans", type=int, default=1300)
    ap.add_argument("--points", type=int, default=1500)
    ap.add_argument("--seed", type=int, default=7)
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    rnd, w = random.Random(a.seed), wr.Writer()
    method = {"Experiment0/ExperimentHeader": wr.header(15, 30.0), "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": wr.ion_spray(5500.0)}
    for n in range(1, a.samples + 1):
        blocks = []
        for k in range(a.scans):
            height = round(60000 * math.exp(-((k - a.scans / 2) / (a.scans / 20)) ** 2))
            bins = range(BINS) if a.points >= BINS else rnd.sample(range(BINS), a.points)  # all the bins: a profile scan
            pts = {b: 1 + rnd.randrange(30) for b in bins}
            for j in range(-4, 5):
                pts[2151 + j] = 5 + round(height * math.exp(-(j / 2.5) ** 2))
            pts = sorted(pts.items())
            blocks.append(("scan", pts, 2000.0 * k, float(sum(c for _, c in pts))))
        w.sample(n, blocks, method)
    w.save(out, a.name)
    print(f"{a.name}.wiff + .wiff.scan: {(out / (a.name + '.wiff.scan')).stat().st_size / 1e6:.1f} MB, {a.samples} samples x {a.scans} scans")


if __name__ == "__main__":
    main()
