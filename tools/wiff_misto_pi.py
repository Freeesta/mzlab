"""Synthetic .wiff + .wiff.scan whose first sample the engine reads and whose second it does not (crates/mzlab-lettori/tests/sciex_sonda.rs).

    python3 tools/wiff_misto_pi.py OUT

Writes OUT/mistopi.wiff(.scan), no Log: s1 Full Scan (code 15, native), s2 product-ion scan with code 6 (a variant: only the readers module reads it), 12 cycles each.
Built with the Writer of wiff_riferimento.py. No laboratory data is used.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import FULL, PRODUCT, Writer, header, ion_spray  # noqa: E402


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    src = {"Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    w = Writer()
    w.sample(1, [("scan", FULL, 1000.0 * (k + 1), 1.0e5) for k in range(12)], {"Experiment0/ExperimentHeader": header(15, 30.0)} | src)
    w.sample(2, [("scan", PRODUCT, 1000.0 * (k + 1), 1.0e4) for k in range(12)], {"Experiment0/ExperimentHeader": header(6, 305.0)} | src)
    w.save(out, "mistopi")


if __name__ == "__main__":
    main()
