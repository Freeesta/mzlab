# TPMINE-PRIVATE
"""A synthetic LC-HRMS irradiation series of caffeine (Orbitrap-like header, centroids, one MS2 of each analyte at its apex): the parent decays, a
hydroxylated product (+O, late) and a demethylated one (-CH2, early) appear, a set of background ions is in every file. Used by the tests of the
high-resolution TP Mine engine."""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402

from msn_synth import mass_of  # noqa: E402

PARENT = mass_of("C8H11N4O2")
HYDROXY = mass_of("C8H11N4O3")
DEMETHYL = mass_of("C7H9N4O2")
BACKGROUND = [102.0550, 118.0863, 131.0816, 150.0583, 166.0863, 184.0733, 226.1550, 255.2330, 279.1590, 284.2950, 322.2730, 391.2843]
BGC = {b: 1.5 + 0.7 * j for j, b in enumerate(BACKGROUND)}          # apex of every background ion: the same in every file
BGH = {b: 4e5 * (1 + (j % 3)) for j, b in enumerate(BACKGROUND)}
FRAG = {PARENT: [(138.0662, 100), (123.0553, 30), (110.0713, 25), (83.0604, 10)], HYDROXY: [(153.0771, 100), (138.0662, 40), (110.0713, 20), (127.0614, 15)],
        DEMETHYL: [(124.0505, 100), (96.0556, 40), (69.0447, 12)]}


def _g(t, c, s):
    return np.exp(-0.5 * ((t - c) / s) ** 2)


ANCHORS = [(211.0970, 1.5), (243.1521, 2.5), (275.2008, 3.2), (307.1455, 6.0), (339.2010, 6.8), (371.1763, 7.5), (403.2264, 8.2), (435.1512, 9.0)]   # strong ions of every file (RT alignment anchors)
ISF_FRAGMENT = mass_of("C6H8N3O")              # a fragment of the parent that the source also makes (2 % of the parent in every file)


def write_series(folder: Path, times=(-1, 0, 5, 10, 20, 40), seed=5, hidden_isf: bool = False, weak: float = 1.0, drift: dict | None = None, anchors: bool = False) -> list[dict]:
    """Writes one file per time (-1: dark adsorption) and returns [{name, path, time, type}]. weak: factor on the +O product (0.2: its first time stays under the detection height). hidden_isf: a product of the treatment with the mass
    of the parent's fragment, co-eluting with the parent and consumed with it: it never makes a feature of its own (the fragment is there in
    every file) but the ratio fragment / parent grows with the treatment. drift: {time: minutes} added to the RT of every ion of that file. anchors: eight strong
    ions (ANCHORS) in every file, for the RT alignment."""
    rng = np.random.default_rng(seed)
    out = []
    for t in times:
        k = max(t, 0)
        amp = {PARENT: 6e7 * np.exp(-k / 18.0), HYDROXY: 0.0 if t <= 0 else weak * 4e6 * (1 - np.exp(-k / 30.0)), DEMETHYL: 0.0 if t <= 0 else 3e6 * (k / 8.0) * np.exp(-k / 8.0)}
        d = (drift or {}).get(t, 0.0)
        rtc = {PARENT: 5.0 + d, HYDROXY: 4.6 + d, DEMETHYL: 4.2 + d}
        specs, rts, tic = [], np.arange(0, 10, 0.04), []
        i = 0
        for rt in rts:
            mz, it = [], []
            for m, a in amp.items():
                y = a * _g(rt, rtc[m], 0.07)
                if y > 3e4:
                    mz.append(m * (1 + rng.normal(0, 0.3e-6))); it.append(y * rng.uniform(0.95, 1.05))
            if hidden_isf:
                y = (0.02 * amp[PARENT] + (0.0 if t <= 0 else 2.5e6 * (1 - np.exp(-k / 15.0)))) * _g(rt, rtc[PARENT], 0.07)
                if y > 3e4:
                    mz.append(ISF_FRAGMENT * (1 + rng.normal(0, 0.3e-6))); it.append(y * rng.uniform(0.95, 1.05))
            for b in BACKGROUND:
                mz.append(b * (1 + rng.normal(0, 0.3e-6))); it.append(BGH[b] * _g(rt, BGC[b] + d, 0.25) * rng.uniform(0.9, 1.1))
            if anchors:
                for m, r in ANCHORS:
                    y = 6e6 * _g(rt, r + d, 0.07)
                    if y > 3e4:
                        mz.append(m * (1 + rng.normal(0, 0.3e-6))); it.append(y * rng.uniform(0.95, 1.05))
            for _ in range(15):
                mz.append(float(rng.uniform(100, 400))); it.append(float(rng.uniform(2e4, 8e4)))
            o = np.argsort(mz)
            mz, it = np.array(mz)[o], np.array(it)[o]
            specs.append(ds._spec(i, rt, mz, it)); tic.append(float(it.sum())); i += 1
            for m, a in amp.items():
                if abs(rt - rtc[m]) < 0.021 and a > 1e6:
                    fm = np.array([f for f, _ in FRAG[m]]) * (1 + rng.normal(0, 0.3e-6, len(FRAG[m])))
                    fi = np.array([v for _, v in FRAG[m]]) * 1e4 * rng.uniform(0.9, 1.1, len(FRAG[m]))
                    specs.append(ds._spec(i, rt + 0.003, fm, fi, level=2, prec=f"{m:.6f}", ce=30)); i += 1
        name = f"cafe_{'dark' if t < 0 else 't%03d' % t}.mzML"
        chroms = [ds._chrom(0, "TIC", np.array(rts), np.array(tic))]
        p = folder / name
        p.write_text(ds._hr_head("exploris", False, p.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                     + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
        out.append({"name": name, "path": str(p), "time": float(t), "type": "sample"})
    return out
