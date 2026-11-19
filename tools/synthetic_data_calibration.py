"""A synthetic calibration series for `quant.curve@1` (every number invented): the integrated peaks of 8 standard levels, a blank, a QC and four
unknowns, for two analytes and an internal standard that follows a varying injection volume, as `chrom.transitions` would give them.

    python3 tools/synthetic_data_calibration.py OUT_DIR     # writes responses.csv and standards.csv

Values are rounded when written, so the files are the same on every platform (the last bit of `random.gauss` does not reach them).
"""
from __future__ import annotations

import random
import sys
from pathlib import Path

LEVELS = [0.5, 1, 2, 5, 10, 20, 50, 100]          # ng/mL
UNKNOWNS = {"u1": 0.3, "u2": 7.0, "u3": 35.0, "u4": 150.0}  # u1 below and u4 above the calibrated range
QC = 15.0


def calibration_series(out: Path, seed: int = 11) -> tuple[Path, Path]:
    """A: linear (300 + 1200·x), CV 3 %; B: slightly curved (50 + 800·x − 1.5·x²), CV 2 %; IS: 5·10⁴ counts. Every area is multiplied by the
    injection volume of its vial (1 ± 5 %), so the ratio to the IS cancels it. S/N = 40·x for A (B 25·x), the IS 400."""
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    g = random.Random(seed)
    vials = [(f"std{i + 1}", f"L{i + 1}", x, "standard") for i, x in enumerate(LEVELS)]
    vials += [("blank", "", 0.0, "blank"), ("qc1", "QC", QC, "qc")] + [(n, "", x, "sample") for n, x in UNKNOWNS.items()]
    resp = ["sample,name,area,height,snr"]
    for sample, _, x, _ in vials:
        v = 1 + g.gauss(0, 0.05)
        a = (300 + 1200 * x) * v * (1 + g.gauss(0, 0.03))
        b = (50 + 800 * x - 1.5 * x * x) * v * (1 + g.gauss(0, 0.02))
        istd = 5e4 * v * (1 + g.gauss(0, 0.01))
        for name, area, sn in (("A", a, 40 * x), ("B", b, 25 * x), ("IS", istd, 400)):
            resp.append(f"{sample},{name},{area:.0f},{area / 12:.0f},{sn:.1f}" if sn > 0 else f"{sample},{name},{area:.0f},{area / 12:.0f},")
    stds = ["sample;level;concentration;unit;role"]
    for sample, level, x, role in vials:
        conc = "" if role in ("sample", "blank") else f"{x:g}".replace(".", ",")
        stds.append(f"{sample};{level};{conc};{'ng/mL' if conc else ''};{role}")
    r, s = out / "responses.csv", out / "standards.csv"
    # bytes, not write_text: on Windows text mode would turn every \n into \r\n and the files would differ from the committed ones
    r.write_bytes(("\n".join(resp) + "\n").encode("utf-8"))
    s.write_bytes(("\n".join(stds) + "\n").encode("utf-8"))
    return r, s


if __name__ == "__main__":
    if len(sys.argv) != 2: sys.exit(__doc__)
    print(*calibration_series(Path(sys.argv[1])))
