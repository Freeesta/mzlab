"""Synthetic .wiff + .wiff.scan with samples of different experiment types in ONE file (e2e_wiff_tipi.py, mixed-type list test).

    python3 tools/wiff_misto.py OUT

Writes OUT/misto.wiff(.scan): s1 Full Scan, s2 Product Ion (one precursor, 12 cycles), s3 MRM (3 transitions), s4 Full Scan, s5 Product Ion (one scan),
s6 Product Ion without a method of its own, s7 Product Ion whose scan blocks do not start with the scan marker (unreadable: not an MRM). Each sample has its own method with the header code of its experiment (15 full, 11 product ion);
OUT/tipi.json: the type the list must show for each sample (full | ms2 | mrm | infusion | empty). Today s2 (12 cycles of one precursor, no column) is a
targeted MS2 by the rule of infusione.rs (one key over the whole run is not an infusion) and s6 (no method) a full scan. Built with the Writer of wiff_riferimento.py. No laboratory data is used.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wiff_riferimento  # noqa: E402
from wiff_riferimento import FULL, MRM, PRODUCT, Writer, header, ion_spray  # noqa: E402


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    src = {"Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
    full_m = {"Experiment0/ExperimentHeader": header(15, 30.0)} | src
    pi_m = {"Experiment0/ExperimentHeader": header(11, 305.0)} | src
    w = Writer()
    w.sample(1, [("scan", FULL, 1000.0 * (k + 1), 1.0e5) for k in range(12)], full_m)
    w.sample(2, [("scan", PRODUCT, 1000.0 * (k + 1), 1.0e4) for k in range(12)], pi_m)
    w.sample(3, [("mrm", v, 500.0 * (k + 1), sum(v)) for k, v in enumerate(MRM)])
    w.sample(4, [("scan", FULL, 1000.0 * (k + 1), 1.0e5) for k in range(12)], full_m)
    w.sample(5, [("scan", PRODUCT, 1000.0, 1.0e4)], pi_m)
    w.sample(6, [("scan", PRODUCT, 1000.0 * (k + 1), 1.0e4) for k in range(12)])
    scan_block = wiff_riferimento.scan_block      # s7: the marker byte is wrong, so the block is not a scan
    wiff_riferimento.scan_block = lambda pts, version=2: b"\x00" + scan_block(pts, version)[1:]
    w.sample(7, [("scan", PRODUCT, 1000.0 * (k + 1), 1.0e4) for k in range(12)], pi_m)
    wiff_riferimento.scan_block = scan_block
    w.save(out, "misto")
    (out / "tipi.json").write_text(json.dumps({"1": "full", "2": "ms2", "3": "mrm", "4": "full", "5": "ms2", "6": "full", "7": "empty"}))


if __name__ == "__main__":
    main()
