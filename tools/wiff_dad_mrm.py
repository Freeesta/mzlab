"""Synthetic 3200 QTRAP `.wiff` pairs with no scans and an optical detector (DAD): reference for crates/mzlab-desktop/tests/dad.rs and tests_e2e/e2e_dad_mrm.py.

    python3 tools/wiff_dad_mrm.py OUT

Writes OUT/dadmrm.wiff (one MRM experiment, two transitions), OUT/dadsim.wiff (one Q1MI experiment, two ions) and OUT/dadms2.wiff (one product-ion
experiment), all with the same DAD matrix: 191 wavelengths, 210-400 nm, 1 nm step, a strong band at 274 nm (compound A, 2.0 min) and a weak one at
230 nm (compound B, 3.5 min) on a flat baseline; OUT/nodad.wiff is the MRM one without a DAD; OUT/dadmrm.json holds the expected band means at the
apexes. Invented numbers, no laboratory data.
"""
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wiff_riferimento as w  # noqa: E402

NM0, STEP, NCH = 210.0, 1.0, 191
ROWS, ROW_MIN = 300, 0.02                  # one DAD row every 1.2 s over 6 min
BASE = 2.0                                 # mAU
# (apex min, time sigma min, band nm, band sigma nm, height mAU)
BANDS = [(2.0, 0.08, 274.0, 8.0, 300.0), (3.5, 0.08, 230.0, 6.0, 40.0)]
CYCLES, CYCLE_MS = 360, 1000.0
TRS = [(300.0, 150.0, 50.0, "cmpA", 40.0, 10.0, 25.0, 5.0), (250.0, 120.0, 50.0, "cmpB", 30.0, 9.0, 20.0, 4.0)]
IONS = [(300.0, 0.0, 50.0, "ionA", 30.0, 10.0, 0.0, 0.0), (250.0, 0.0, 50.0, "ionB", 30.0, 10.0, 0.0, 0.0)]


def absorbance(t, nm):
    return BASE + sum(h * math.exp(-0.5 * ((t - rt) / st) ** 2) * math.exp(-0.5 * ((nm - c) / sn) ** 2) for rt, st, c, sn, h in BANDS)


def dad():
    ts = [ROW_MIN * k for k in range(ROWS)]
    rows = [[absorbance(t, NM0 + STEP * j) for j in range(NCH)] for t in ts]
    return ts, rows


def ms_blocks():
    """Two currents per cycle, each with a peak at its compound's apex."""
    out = []
    for k in range(CYCLES):
        t = CYCLE_MS * k / 60000.0
        v = [round(100.0 + 5.0e4 * math.exp(-0.5 * ((t - rt) / 0.06) ** 2), 1) for rt, *_ in BANDS]
        out.append(("mrm", v, CYCLE_MS * k, sum(v)))
    return out


def ms2_blocks():
    """Product-ion scans of one precursor, one every 6 s, with a fragment peak at compound A's apex."""
    out = []
    for k in range(60):
        t = 6000.0 * k
        height = round(4.0e4 * math.exp(-0.5 * ((t / 60000.0 - BANDS[0][0]) / 0.1) ** 2))
        pts = [(1500 + j, 5 + round(height * math.exp(-(j / 2.5) ** 2))) for j in range(-4, 5)] + [(800 + 37 * j, 3 + j % 4) for j in range(20)]
        pts.sort()
        out.append(("scan", pts, t, float(sum(c for _, c in pts))))
    return out


def band_mean(t, lo, hi):
    """The mean of the channels in lo..=hi at time t, as the engine computes an XWC."""
    nms = [NM0 + STEP * j for j in range(NCH) if lo <= NM0 + STEP * j <= hi]
    return sum(absorbance(t, nm) for nm in nms) / len(nms)


def main(out=None):
    out = Path(out if out is not None else sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    ts, rows = dad()
    for name, code, table in (("dadmrm", 4, TRS), ("dadsim", 1, IONS)):
        method = {"Experiment0/ExperimentHeader": w.header(code, 0.0, polarity=0), "Experiment0/MassRangeEx/MassRangeEx": w.mrm_method(table),
                  "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": w.ion_spray(5500.0)}
        wr = w.Writer()
        wr.sample(1, ms_blocks(), method, log="3200 QTRAP", dad=(ts, NM0, STEP, rows))
        wr.save(out, name)
    wr = w.Writer()
    wr.sample(1, ms_blocks(), {"Experiment0/ExperimentHeader": w.header(4, 0.0, polarity=0), "Experiment0/MassRangeEx/MassRangeEx": w.mrm_method(TRS)}, log="3200 QTRAP")
    wr.save(out, "nodad")
    wr = w.Writer()
    wr.sample(1, ms2_blocks(), {"Experiment0/ExperimentHeader": w.header(11, 300.0, polarity=0),
                                "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": w.ion_spray(5500.0)}, log="3200 QTRAP", dad=(ts, NM0, STEP, rows))
    wr.save(out, "dadms2")
    a_rt, b_rt = BANDS[0][0], BANDS[1][0]
    info = {"rows": ROWS, "nm": [NM0, NM0 + STEP * (NCH - 1), NCH],
            "a": {"rt": a_rt, "nm0": 270.0, "nm1": 278.0, "max": band_mean(a_rt, 270.0, 278.0)},
            "b": {"rt": b_rt, "nm0": 228.0, "nm1": 232.0, "max": band_mean(b_rt, 228.0, 232.0)},
            "twc_max": max(sum(r) for r in rows)}
    (out / "dadmrm.json").write_text(json.dumps(info))
    return info


if __name__ == "__main__":
    print(json.dumps(main()))
