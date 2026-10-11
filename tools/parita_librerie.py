"""Dati d'oro di parità per le librerie spettrali (SOLO per i test): punteggi di riferimento calcolati con `ms_entropy` (Flash entropy search,
Apache-2.0) e `matchms` (Apache-2.0) su spettri SINTETICI (nessun nome di composto) -> tests/golden/librerie/.
Uso: python3 tools/parita_librerie.py [--uscita DIR]   (dipendenze di sviluppo: pip install ms_entropy matchms; vedi pyproject.toml)
I test Rust (crates/mzlab-lib/tests/parita.rs) confrontano il percorso senza quantizzazione (1e-6) e quello con l'indice .mzlib (1e-4)."""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
TOL_FRAG, TOL_PREC = 0.02, 0.01


def spectrum(rng, n: int, prec: float) -> tuple[np.ndarray, np.ndarray]:
    """n peaks at least 0.15 Da apart, all below prec - 5, intensities within a factor 60 (so none is below 1% of the highest)."""
    top = prec - 5.0
    mz, m = [], float(rng.uniform(50, 70))
    gap = max((top - m) / max(n, 1) * 1.7, 0.4)
    while len(mz) < n and m < top:
        mz.append(round(m, 5))
        m += float(rng.uniform(0.15, gap))
    it = np.round(rng.uniform(150, 9000, len(mz)), 2)
    # a first peak of its own: ms_entropy 1.5.3 misses a shifted (neutral-loss) pair in the hybrid search when it involves the first peak of both spectra
    mz = [round(float(rng.uniform(20, 40)), 5)] + mz
    it = np.concatenate([[float(np.round(rng.uniform(150, 9000), 2))], it])
    return np.array(mz), it


def variant(rng, mz, it, prec, kind: str):
    """A second spectrum related to the first one: same compound with noise, analogue (half of the peaks shifted), or chimeric."""
    if kind == "same":
        return mz + rng.normal(0, 0.003, mz.size).clip(-0.006, 0.006), np.round(it * rng.uniform(0.6, 1.5, it.size), 2), prec + float(rng.uniform(-0.004, 0.004))
    if kind == "analogue":
        d = float(rng.choice([14.01565, 15.99491, 79.95682, -2.01565, 42.01057, 28.0313]))
        m = mz.copy()
        sel = rng.random(m.size) < 0.5
        m[sel] += d
        m = np.round(m + rng.normal(0, 0.002, m.size).clip(-0.004, 0.004), 5)
        order = np.argsort(m)
        return m[order], np.round(it * rng.uniform(0.7, 1.4, it.size), 2)[order], round(prec + d, 5)
    raise ValueError(kind)


def clean_sorted(mz, it):
    o = np.argsort(mz)
    return np.asarray(mz)[o], np.asarray(it)[o]


def pair_cases(rng, n=80):
    from matchms import Spectrum
    from matchms.similarity import CosineGreedy, ModifiedCosineGreedy
    from ms_entropy import calculate_entropy_similarity

    out = []
    for k in range(n):
        prec = round(float(rng.uniform(200, 800)), 4)
        a = spectrum(rng, int(rng.integers(5, 28)), prec)
        kind = ["same", "analogue", "same", "other"][k % 4]
        if kind == "other":
            pb = round(float(rng.uniform(200, 800)), 4)
            b = spectrum(rng, int(rng.integers(5, 28)), pb)
            b = (b[0], b[1], pb)
        else:
            m, i, pb = variant(rng, a[0], a[1], prec, kind)
            b = (m, i, pb)
            if len(b[0]) > 1 and np.any(np.diff(b[0]) < 0.12):
                kind = "other"
        a_s = Spectrum(mz=a[0], intensities=a[1], metadata={"precursor_mz": prec})
        b_s = Spectrum(mz=b[0], intensities=b[1], metadata={"precursor_mz": b[2]})
        na, nb = np.stack([a[0], a[1] / a[1].sum()], 1), np.stack([b[0], b[1] / b[1].sum()], 1)      # clean_spectra=False wants intensities summing to 1
        ent = float(calculate_entropy_similarity(na, nb, ms2_tolerance_in_da=TOL_FRAG, clean_spectra=False))
        cos = CosineGreedy(tolerance=TOL_FRAG, mz_power=0.0, intensity_power=0.5).pair(a_s, b_s)
        mod = ModifiedCosineGreedy(tolerance=TOL_FRAG, mz_power=0.0, intensity_power=0.5).pair(a_s, b_s)
        out.append({
            "kind": kind,
            "a": {"mz": a[0].tolist(), "it": a[1].tolist(), "prec": prec},
            "b": {"mz": np.asarray(b[0]).tolist(), "it": np.asarray(b[1]).tolist(), "prec": b[2]},
            "tol": TOL_FRAG,
            "entropy": ent,
            "cosine": float(cos["score"]), "matches": int(cos["matches"]),
            "mod_cosine": float(mod["score"]), "mod_matches": int(mod["matches"]),
        })
    return out


def flash_case(rng, n_lib=150, n_q=24):
    from ms_entropy import FlashEntropySearch

    lib, queries = [], []
    for i in range(n_lib):
        prec = round(float(rng.uniform(180, 700)), 4)
        mz, it = spectrum(rng, int(rng.integers(6, 30)), prec)
        lib.append({"id": i, "precursor_mz": prec, "peaks": np.stack([mz, it], 1)})
    # queries: noisy copies of some library spectra, analogues of others, and unrelated ones
    for q in range(n_q):
        base = lib[int(rng.integers(0, n_lib))]
        kind = ["same", "analogue", "other"][q % 3]
        if kind == "other":
            prec = round(float(rng.uniform(180, 700)), 4)
            mz, it = spectrum(rng, int(rng.integers(6, 30)), prec)
        else:
            m, i, prec = variant(rng, base["peaks"][:, 0], base["peaks"][:, 1], base["precursor_mz"], kind)
            mz, it = clean_sorted(m, i)
            if np.any(np.diff(mz) < 0.12):
                mz, it = spectrum(rng, 12, prec)
        queries.append({"kind": kind, "prec": prec, "mz": mz, "it": it})
    fes = FlashEntropySearch(max_ms2_tolerance_in_da=0.024)
    fes.build_index([{"id": s["id"], "precursor_mz": s["precursor_mz"], "peaks": s["peaks"].copy()} for s in lib], max_indexed_mz=1500.00005, clean_spectra=True)
    ids = [fes[k]["id"] for k in range(len(lib))]
    res = []
    for q in queries:
        r = fes.search(precursor_mz=q["prec"], peaks=np.stack([q["mz"], q["it"]], 1), ms1_tolerance_in_da=TOL_PREC, ms2_tolerance_in_da=TOL_FRAG, method="all")
        entry = {"kind": q["kind"], "prec": q["prec"], "mz": q["mz"].tolist(), "it": q["it"].tolist()}
        for name, key in (("identity", "identity_search"), ("open", "open_search"), ("nl", "neutral_loss_search"), ("hybrid", "hybrid_search")):
            v = np.asarray(r[key], dtype=float)
            entry[name] = {str(ids[k]): round(float(v[k]), 10) for k in range(len(ids)) if v[k] > 1e-9}
        res.append(entry)
    return {
        "tol_frag": TOL_FRAG, "tol_prec": TOL_PREC,
        "library": [{"id": s["id"], "prec": s["precursor_mz"], "mz": s["peaks"][:, 0].tolist(), "it": s["peaks"][:, 1].tolist()} for s in lib],
        "queries": res,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--uscita", default=str(ROOT / "tests" / "golden" / "librerie"))
    a = ap.parse_args()
    out = Path(a.uscita)
    out.mkdir(parents=True, exist_ok=True)
    import ms_entropy
    import matchms
    meta = {"fonte": "ms_entropy %s, matchms %s; spettri sintetici (nessun composto)" % (getattr(ms_entropy, "__version__", "1.5.3"), matchms.__version__)}
    rng = np.random.default_rng(20261010)
    pares = {"formato": "pares/1", **meta, "casi": pair_cases(rng)}
    flash = {"formato": "flash/1", **meta, **flash_case(rng)}
    for name, d in (("pares.json", pares), ("flash.json", flash)):
        (out / name).write_text(json.dumps(d, separators=(",", ":")), encoding="utf-8")
        print(name, (out / name).stat().st_size, "byte")
    n = {k: sum(1 for q in flash["queries"] if q[k]) for k in ("identity", "open", "nl", "hybrid")}
    print("query con risultato:", n)


if __name__ == "__main__":
    main()
