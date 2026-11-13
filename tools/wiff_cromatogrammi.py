"""Synthetic .wiff + .wiff.scan pairs whose chromatogram list depends on the experiments of the sample (tests/test_wiff_cromatogrammi.py, e2e_wiff_cromatogrammi.py).

    python3 tools/wiff_cromatogrammi.py OUT

Writes OUT/fsmrm.wiff(.scan) (experiment 1 Full Scan, experiment 2 MRM of three transitions, three cycles), OUT/duemrm.wiff(.scan) (two MRM experiments of two
and three transitions, three cycles) and OUT/solofs.wiff(.scan) (one Full Scan experiment, three cycles), and OUT/attesi.json: the chromatogram ids each
sample must list, in order ('SRM' stands for any SRM chromatogram). Built with the Writer of wiff_riferimento.py. No laboratory data is used.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import FULL, LATE, MRM, Writer, header, ion_spray, mrm_method  # noqa: E402

SRC = {"IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
TR3 = [(364.5, 176.0, 50.0, "tA", 40.0, 10.0, 25.0, 5.0), (364.5, 159.0, 25.5, "tB", 41.0, 10.5, 35.0, 6.0), (250.25, 100.5, 100.0, "tC", 30.0, 9.0, 15.0, 4.0)]
TR2 = [(300.5, 120.0, 40.0, "tD", 40.0, 10.0, 20.0, 5.0), (280.0, 90.5, 40.0, "tE", 40.0, 10.0, 30.0, 5.0)]


def experiment(e, code, trs=None):
    m = {f"Experiment{e}/ExperimentHeader": header(code, 30.0 if code == 15 else 0.0)} | {f"Experiment{e}/{k}": v for k, v in SRC.items()}
    return m | ({f"Experiment{e}/MassRangeEx/MassRangeEx": mrm_method(trs)} if trs else {})


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    scans = [FULL, LATE, FULL]
    w = Writer()
    blocks = []
    for k in range(3):
        blocks += [("scan", scans[k], 3000.0 * k + 1000.0, 1000.0 + k), ("mrm", MRM[k], 3000.0 * k + 1500.0, sum(MRM[k]))]
    w.sample(1, blocks, experiment(0, 15) | experiment(1, 4, TR3))
    w.save(out, "fsmrm")
    w = Writer()
    blocks = []
    for k in range(3):
        blocks += [("mrm", MRM[k][:2], 3000.0 * k + 1000.0, sum(MRM[k][:2])), ("mrm", MRM[k + 1], 3000.0 * k + 1500.0, sum(MRM[k + 1]))]
    w.sample(1, blocks, experiment(0, 4, TR2) | experiment(1, 4, TR3))
    w.save(out, "duemrm")
    w = Writer()
    w.sample(1, [("scan", scans[k], 3000.0 * k + 1000.0, 1000.0 + k) for k in range(3)], experiment(0, 15))
    w.save(out, "solofs")
    many = lambda kind, e: f"{kind} sample=1 period=1 experiment={e}"
    (out / "attesi.json").write_text(json.dumps({
        "fsmrm": [many("TIC", 1), many("TIC", 2), many("BPC", 2), "SRM", "SRM", "SRM", many("BPC", 1)],
        "duemrm": [many("TIC", 1), many("BPC", 1), "SRM", "SRM", many("TIC", 2), many("BPC", 2), "SRM", "SRM", "SRM"],
        "solofs": ["TIC", "BPC"],
    }, indent=1))


if __name__ == "__main__":
    main()
