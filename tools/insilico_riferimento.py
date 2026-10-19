#!/usr/bin/env python3
"""Golden data for the in-silico ranking of the MS2 engine (SOLO test reference; pure Python, no RDKit).

The explained peaks of the 39 structures come from `tests/golden/attributio/sintetica/frammenti.json` (made with Attributio's RDKit fragmenter);
this script adds invented intensities and the MetFrag-style fragmenter score on them:
    w(peak) = mz^0.421 * intensity^0.1307;  score = sum(w of explained peaks) / sum(w of all peaks)
Writes tests/data/attributio/ms2_engine/insilico.json.   Use: python3 tools/insilico_riferimento.py
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "tests/golden/attributio/sintetica/frammenti.json"
OUT = ROOT / "tests/data/attributio/ms2_engine/insilico.json"


def weight(mz, it):
    return mz ** 0.421 * it ** 0.1307


def main():
    out = []
    for k, m in enumerate(json.loads(SRC.read_text())["molecole"]):
        if not m.get("ok"):
            continue
        peaks = [p for p in m["peaks"] if p > 0]
        ints = [round(100.0 + ((j * 37 + k * 11) % 900), 1) for j in range(len(peaks))]
        explained = sorted({e[0] for e in m["explained"]})
        w_all = sum(weight(p, i) for p, i in zip(peaks, ints))
        w_exp = sum(weight(p, i) for p, i in zip(peaks, ints) if p in explained)
        out.append({"smiles": m["smiles"], "pol": m["pol"], "peaks": peaks, "intensities": ints,
                    "explained": explained, "score": w_exp / w_all if w_all else 0.0})
    OUT.write_text(json.dumps({"molecole": out}))
    print(len(out), "molecules")


if __name__ == "__main__":
    main()
