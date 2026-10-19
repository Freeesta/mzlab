#!/usr/bin/env python3
"""Golden data for the MS2 engine (SOLO test reference; the engine itself is Rust).

Replays the search of Attributio `ms2_engine/matcher.py::_search_mode` on invented spectra with `ms_entropy.FlashEntropySearch`
and writes the library (MSP and GNPS-JSON texts), the queries and the hits to tests/data/attributio/ms2_engine/golden.json.
Use: python3 tools/ms2_engine_riferimento.py   (dev dependencies: pip install ms_entropy numpy)
"""
import json
from pathlib import Path

import numpy as np
from ms_entropy import FlashEntropySearch, clean_spectrum

OUT = Path(__file__).resolve().parent.parent / "tests/data/attributio/ms2_engine/golden.json"
PPM, MIN_SCORE = 10.0, 0.5


def spectrum(rng, prec):
    n = int(rng.integers(5, 14))
    mz = np.sort(np.round(rng.uniform(40, prec - 3, n), 4))
    it = np.round(rng.uniform(100, 9000, n), 1)
    return mz, it


def main():
    rng = np.random.default_rng(20261019)
    lib = []
    for k in range(60):
        pol = "POSITIVE" if k % 3 else "NEGATIVE"
        prec = round(float(rng.uniform(150, 700)), 4)
        mz, it = spectrum(rng, prec)
        lib.append({"id": f"CCMSLIB{k:05d}", "name": f"Invented compound {k}", "precursor_mz": prec,
                    "ion_mode": pol, "adduct": "[M+H]+" if pol == "POSITIVE" else "[M-H]-",
                    "peaks": [[float(a), float(b)] for a, b in zip(mz, it)]})
    queries = []
    for k in range(40):
        src = lib[int(rng.integers(0, len(lib)))]
        mz = np.array([p[0] for p in src["peaks"]])
        it = np.array([p[1] for p in src["peaks"]])
        kind = k % 4
        prec = src["precursor_mz"]
        if kind == 0:
            mz = mz + rng.normal(0, 0.003, mz.size).clip(-0.006, 0.006)
            it = it * rng.uniform(0.6, 1.5, it.size)
        elif kind == 1:
            keep = rng.random(mz.size) < 0.6
            keep[0] = True
            mz, it = mz[keep], it[keep]
        elif kind == 2:
            mz, it = spectrum(rng, prec)
        else:
            prec = prec * (1 + 7e-6)
        queries.append({"query_id": k, "precursor_mz": float(prec), "ion_mode": src["ion_mode"],
                        "peaks_mz": [float(x) for x in mz], "peaks_int": [float(x) for x in it]})
    hits = {}
    for mode in ("POSITIVE", "NEGATIVE"):
        sel = [s for s in lib if s["ion_mode"] == mode]
        search = FlashEntropySearch()
        spectra = [{"precursor_mz": s["precursor_mz"], "peaks": np.array(s["peaks"], dtype=np.float32), "_id": s["id"]} for s in sel]
        sorted_spectra = search.build_index(spectra)
        for q in [q for q in queries if q["ion_mode"] == mode]:
            peaks = np.column_stack((np.array(q["peaks_mz"], dtype=np.float32), np.array(q["peaks_int"], dtype=np.float32)))
            peaks = clean_spectrum(peaks, min_mz=10.0, max_mz=1000.0, noise_threshold=0.01)
            res = search.search(precursor_mz=q["precursor_mz"], peaks=peaks, ms2_tolerance_in_da=0.02,
                                ms1_tolerance_in_da=q["precursor_mz"] * PPM / 1e6, method="identity")
            scores = res["identity_search"]
            rows = [{"id": sorted_spectra[int(i)]["_id"], "score": float(scores[i])} for i in np.where(scores >= MIN_SCORE)[0]]
            rows.sort(key=lambda r: -r["score"])
            hits[str(q["query_id"])] = rows
    OUT.write_text(json.dumps({"ppm": PPM, "min_score": MIN_SCORE, "library": lib, "queries": queries, "hits": hits}, indent=0))
    print(sum(1 for v in hits.values() if v), "queries with hits of", len(queries))


if __name__ == "__main__":
    main()
