"""Synthetic SIM (Q1MI) `.wiff` pair of a 3200 QTRAP: reference for crates/mzlab-core/tests/wiff_sim.rs and tests_e2e/e2e_wiff_sim.py.

    python3 tools/wiff_sim.py OUT

Writes OUT/sim.wiff + .wiff.scan with two samples and two Q1MI experiments per cycle (positive with three ions, negative with one; the method table
holds each ion as an MRM-like row with Q3 = 0, which is a hypothesis: no real SIM file was seen), and OUT/sim.json with the ions and the apex RT of
every peak. Ion 237.1 has two isobaric peaks (isomers at two RTs). Also OUT/sim_periodi.wiff (the same method with a second period: refused with
`periods=2`), OUT/sim_vuoti.wiff (one sample whose cycle 10 has an empty row in experiment 1: left out, the ions keep their columns), and for the SIM
table of the Data view OUT/sim3.wiff (three named samples, one positive experiment of seven ions, one peak per ion; OUT/sim3.json) with
OUT/sim3_cut.wiff, the same pair with its .wiff.scan cut at 60 %.
Invented numbers, no laboratory data.
"""
import json
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wiff_riferimento as w  # noqa: E402

CYCLES = 300
CYCLE_MS = 1200.0
# (Q1, dwell ms, name); experiment 1 positive, experiment 2 negative
POS = [(237.1, 50.0, "ionA"), (253.1, 50.0, "ionB"), (180.1, 50.0, "ionC")]
NEG = [(235.1, 100.0, "ionN")]
# apex RT (min) and height per ion and sample; two peaks on 237.1
PEAKS = {
    1: {237.1: [(2.5, 8.0e4), (4.0, 3.0e4)], 253.1: [(1.5, 5.0e4)], 180.1: [(5.0, 2.0e4)], 235.1: [(3.0, 1.0e4)]},
    2: {237.1: [(2.55, 6.0e4), (4.05, 4.5e4)], 253.1: [(1.52, 2.0e4)], 180.1: [(5.1, 1.0e4)], 235.1: [(3.02, 7.0e3)]},
}
SIGMA_MIN = 0.05


def value(sample, q1, t_min):
    base = 50.0 + (int(t_min * 1000 + q1) % 7)
    return base + sum(h * math.exp(-0.5 * ((t_min - rt) / SIGMA_MIN) ** 2) for rt, h in PEAKS[sample][q1])


def table(ions):
    return w.mrm_method([(q1, 0.0, dwell, name, 30.0, 10.0, 0.0, 0.0) for q1, dwell, name in ions])


def main(out=None):
    out = Path(out if out is not None else sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    method = {
        "Experiment0/ExperimentHeader": w.header(1, 0.0, polarity=0),
        "Experiment0/MassRangeEx/MassRangeEx": table(POS),
        "Experiment1/ExperimentHeader": w.header(1, 0.0, polarity=1),
        "Experiment1/MassRangeEx/MassRangeEx": table(NEG),
        "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": w.ion_spray(5500.0),
    }
    wr = w.Writer()
    for n in (1, 2):
        blocks = []
        for k in range(CYCLES):
            for e, ions in enumerate((POS, NEG)):
                t_ms = CYCLE_MS * k + 600.0 * e
                v = [round(value(n, q1, t_ms / 60000.0), 1) for q1, _, _ in ions]
                blocks.append(("mrm", v, t_ms, sum(v)))
        wr.sample(n, blocks, method, log="3200 QTRAP")
    wr.save(out, "sim")
    wr = w.Writer()
    wr.sample(1, blocks, method | {"Period1/Experiment0/ExperimentHeader": w.header(1, 0.0, polarity=0)}, log="3200 QTRAP")
    wr.save(out, "sim_periodi")
    wr = w.Writer()
    wr.sample(1, [("mrm", [], b[2], 0.0) if i == 20 else b for i, b in enumerate(blocks)], method, log="3200 QTRAP")
    wr.save(out, "sim_vuoti")
    info = {"ioni": [{"q1": q1, "dwell": d, "name": nm, "polarita": 1, "esperimento": 1} for q1, d, nm in POS]
            + [{"q1": q1, "dwell": d, "name": nm, "polarita": -1, "esperimento": 2} for q1, d, nm in NEG],
            "campioni": {str(n): {str(q1): [rt for rt, _ in p] for q1, p in PEAKS[n].items()} for n in PEAKS},
            "cicli": CYCLES}
    (out / "sim.json").write_text(json.dumps(info))

    sim3(out)


# the SIM table: seven ions (Q1, name) of one positive experiment, three named samples, one peak each (apex RT min, height); invented numbers
SIM3 = [(253.1, "ionA"), (267.1, "ionB"), (251.1, "ionC"), (271.1, "ionD"), (285.1, "ionE"), (225.1, "ionF"), (210.1, "ionG")]
SIM3_SAMPLES = ["Sample_A", "Sample_B", "Sample_C"]
SIM3_CYCLES = 400


def sim3_peak(s, j):
    return 1.5 + 0.9 * j + 0.03 * s, (9.0e4 - 1.0e4 * j) * (1.0 - 0.3 * s)


def sim3(out):
    method = {"Experiment0/ExperimentHeader": w.header(1, 0.0, polarity=0),
              "Experiment0/MassRangeEx/MassRangeEx": table([(q1, 20.0, nm) for q1, nm in SIM3]),
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": w.ion_spray(5500.0)}
    wr = w.Writer()
    for s, name in enumerate(SIM3_SAMPLES):
        blocks = []
        for k in range(SIM3_CYCLES):
            t = CYCLE_MS * k / 60000.0
            v = [round(40.0 + (k * 7 + j) % 11 + h * math.exp(-0.5 * ((t - rt) / SIGMA_MIN) ** 2), 1)
                 for j in range(len(SIM3)) for rt, h in [sim3_peak(s, j)]]
            blocks.append(("mrm", v, CYCLE_MS * k, sum(v)))
        wr.sample(s + 1, blocks, method, log="3200 QTRAP", data_name=name)
    wr.save(out, "sim3")
    wr.save(out, "sim3_cut")
    scan = (out / "sim3_cut.wiff.scan").read_bytes()
    (out / "sim3_cut.wiff.scan").write_bytes(scan[:len(scan) * 6 // 10])
    (out / "sim3.json").write_text(json.dumps({"ioni": [{"q1": q1, "name": nm} for q1, nm in SIM3], "campioni": SIM3_SAMPLES,
                                                "apici": [[round(sim3_peak(s, j)[0], 3) for j in range(len(SIM3))] for s in range(3)],
                                                "cicli": SIM3_CYCLES}))


if __name__ == "__main__":
    main()
