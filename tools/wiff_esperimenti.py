"""Synthetic `.wiff` + `.wiff.scan` with twenty experiments (crates/mzlab-core/tests/wiff_many_experiments.rs).

    python3 tools/wiff_esperimenti.py OUT

Writes OUT/esperimenti.wiff(.scan) and OUT/esperimenti.json. Experiments 0-16 are full scans, 17-19 product-ion scans with their own collision energy,
so a reader that stops the method at sixteen experiments loses the energies. Built with the Writer of wiff_riferimento.py; the numbers are invented.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import FULL, PRODUCT, Writer, header, ion_spray, mass_range  # noqa: E402

N, CE = 20, {17: 21.0, 18: 32.5, 19: 44.0}


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    method = {"Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    for e in range(N):
        method[f"Experiment{e}/ExperimentHeader"] = header(11, 100.0 + e) if e in CE else header(15, 30.0)
        method[f"Experiment{e}/MassRangeEx/MassRangeEx"] = mass_range(CE[e]) if e in CE else bytes(8)
    blocks = [("scan", PRODUCT if e in CE else FULL, 1000.0 * c + 10.0 * e, 1.0) for c in range(2) for e in range(N)]
    w = Writer()
    w.sample(1, blocks, method)
    w.save(out, "esperimenti")
    (out / "esperimenti.json").write_text(json.dumps({"ce": [CE.get(e) for e in range(N)]}))


if __name__ == "__main__":
    main()
