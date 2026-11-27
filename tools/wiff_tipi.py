"""Synthetic `.wiff` pair `tipi` for the sample chooser: reference for crates/mzlab-core/tests/wiff_listing.rs, tests/test_samples_list_parita.py and tests_e2e/e2e_wiff_choose.py.

    python3 tools/wiff_tipi.py OUT

Writes OUT/tipi.wiff + .wiff.scan with three samples: 1 = five full scans, 2 = SIM only (seven cycles of two ions, one block per cycle, no point count in the index),
3 = an injection that acquired nothing (no Idx). The listing must say 5, 7 and 0 scans, kinds ms, sim and none, and `why` = `empty` for sample 3 only. Invented numbers.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wiff_riferimento as w  # noqa: E402


def main(out=None):
    out = Path(out if out is not None else sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    ions = [(237.1, 0.0, 50.0, "ionA", 30.0, 10.0, 0.0, 0.0), (253.1, 0.0, 50.0, "ionB", 30.0, 10.0, 0.0, 0.0)]
    wr = w.Writer()
    wr.sample(1, [("scan", w.FULL, 1000.0 * k, 1.0) for k in range(5)], {"Experiment0/ExperimentHeader": w.header(15, 30.0)})
    wr.sample(2, [("mrm", [100.0 + k, 50.0], 800.0 * k, 150.0 + k) for k in range(7)],
              {"Experiment0/ExperimentHeader": w.header(1, 0.0, polarity=0), "Experiment0/MassRangeEx/MassRangeEx": w.mrm_method(ions)})
    wr.sample(3, [])
    del wr.streams["SampleSubtree/Sample3/Idx"]
    wr.save(out, "tipi")


if __name__ == "__main__":
    main()
