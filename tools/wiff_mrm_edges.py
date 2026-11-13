"""Synthetic MRM .wiff + .wiff.scan with the edge rows of a dense MRM run (crates/mzlab-core/tests/wiff_mrm_edges.rs).

    python3 tools/wiff_mrm_edges.py OUT

Writes OUT/mrmedges.wiff(.scan): one MRM experiment of three transitions over four rows: a block, an empty row (size 0, a cycle that acquired
nothing), a dense block whose first cps is negative noise (not the `-(N + 0.01)` head of a sparse block), and a block. Also OUT/mrmperiods.wiff and
OUT/fullperiods.wiff: an MRM and a full scan method with a second period, both refused with `err.file.wiffPeriods` (never folded into Period0).
Built with the Writer of wiff_riferimento.py; the numbers are invented, no laboratory data is used.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import FULL, Writer, header, ion_spray, mrm_method  # noqa: E402

ROWS = [[10.0, 20.0, 30.0], [], [-2.5, 40.0, 50.0], [60.0, 70.0, 80.0]]


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    trs = [(300.0 + j, 150.0 + j, 20.0, f"t{j}", 30.0, 10.0, 20.0, 5.0) for j in range(3)]
    method = {"Experiment0/ExperimentHeader": header(4, 0.0), "Experiment0/MassRangeEx/MassRangeEx": mrm_method(trs),
              "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    w = Writer()
    w.sample(1, [("mrm", v, 1000.0 * k, sum(v)) for k, v in enumerate(ROWS)], method)
    w.save(out, "mrmedges")
    w = Writer()
    w.sample(1, [("mrm", v, 1000.0 * k, sum(v)) for k, v in enumerate(ROWS) if v],
             method | {"Period1/Experiment0/ExperimentHeader": header(4, 0.0)})
    w.save(out, "mrmperiods")
    w = Writer()
    w.sample(1, [("scan", FULL, 1000.0 * k, 1.0e5) for k in range(3)],
             {"Experiment0/ExperimentHeader": header(15, 30.0), "Period1/Experiment0/ExperimentHeader": header(15, 30.0)})
    w.save(out, "fullperiods")


if __name__ == "__main__":
    main()
