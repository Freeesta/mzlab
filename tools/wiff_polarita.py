"""Synthetic MRM `.wiff` + `.wiff.scan` whose samples differ in polarity (crates/mzlab-core/tests/wiff_polarity.rs).

    python3 tools/wiff_polarita.py OUT

Writes OUT/polarita.wiff(.scan) and OUT/polarita.json. Sample 1: no method (and none below it to borrow), so no polarity is known and none is written.
Sample 2: the experiment header says negative while the ion spray voltage is positive (the header wins). Sample 3: the header says positive. Built with the Writer of wiff_riferimento.py;
the numbers are invented, no laboratory data is used.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import MRM, Writer, header, ion_spray, mrm_method  # noqa: E402

TRS = [(364.5, 176.0, 50.0, "tpA", 40.0, 10.0, 25.0, 5.0), (364.5, 159.0, 25.5, "tpB", 41.0, 10.5, 35.0, 6.0), (250.25, 100.5, 100.0, "ref", 30.0, 9.0, 15.0, 4.0)]


def method(polarity: int) -> dict:
    return {"Experiment0/ExperimentHeader": header(4, 0.0, polarity), "Experiment0/MassRangeEx/MassRangeEx": mrm_method(TRS),
            "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    blocks = [("mrm", v, 1000.0 * k, sum(v)) for k, v in enumerate(MRM)]
    w = Writer()
    w.sample(1, blocks, None)
    w.sample(2, blocks, method(1))
    w.sample(3, blocks, method(0))
    w.save(out, "polarita")
    (out / "polarita.json").write_text(json.dumps({"1": None, "2": "negative", "3": "positive", "transitions": len(TRS)}))


if __name__ == "__main__":
    main()
