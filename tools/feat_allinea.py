"""Reference answers of `tpmine.hr.features.align` and `rtalign.build` for the parity test of `crates/mzlab-feat/src/{align,rtmap}.rs`.

    python3 tools/feat_riferimento.py OUT --solo allinea [--golden]

writes `allinea_sintetico.json`: the features of a series of synthetic files (invented ions with an RT drift per file, isomers, chains of ions a few
ppm apart, chains of ions a few hundredths of a minute apart, Fourier lobes, missing peaks) and what the Python code answers: the groups of `align`
(default parameters, and with `max_rt_span`) and the shift curves of `rtalign.build` with some values of `at`, `expected` and `reference`. Every
ion is a number; no compound names.
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from tpmine.hr import features, rtalign

N_FILES, SEED = 5, 20261011


def _ions(rng: np.random.Generator) -> list[tuple[float, float, float]]:
    """(mz, rt, area) of the invented ions: singles, isomer pairs, ppm chains, RT chains."""
    ions = [(rng.uniform(150, 700), rng.uniform(1, 20), 10 ** rng.uniform(5.5, 8.5)) for _ in range(350)]
    for _ in range(40):
        m, r, a = rng.uniform(150, 700), rng.uniform(1, 20), 10 ** rng.uniform(6, 8)
        ions += [(m, r, a), (m, r + 0.3, a * 0.7)]
    for _ in range(20):
        m, r = rng.uniform(150, 700), rng.uniform(1, 20)
        ions += [(m * (1 + 3e-6 * i), r, 10 ** rng.uniform(5.5, 7)) for i in range(8)]
    for _ in range(20):
        m, r = rng.uniform(150, 700), rng.uniform(1, 18)
        ions += [(m, r + 0.08 * i, 10 ** rng.uniform(5.5, 7)) for i in range(12)]
    return ions


def _file(rng: np.random.Generator, ions, k: int) -> dict:
    drift = lambda r: (0.05 * k + 0.02 * k * np.sin(r / 4.0)) * (1 if k % 2 else -1)       # noqa: E731
    rows = []
    for m, r, a in ions:
        if rng.random() > 0.85:
            continue
        rt = np.float32(r + drift(r) + rng.normal(0, 0.01))
        area = a * 10 ** rng.normal(0, 0.1)
        w = float(np.float32(0.1 + rng.uniform(0, 0.1)))
        rows.append((float(np.round(m * (1 + rng.normal(0, 1e-6)), 6)), float(rt), area, area / 40.0, float(rt) - w / 2, float(rt) + w / 2))
    rows.sort(key=lambda t: t[0])
    c = np.array(rows)
    art = (np.arange(len(c)) % 37) == 0
    f32 = lambda v: [float(x) for x in np.float32(v)]                                         # noqa: E731
    return {"mz": c[:, 0].tolist(), "rt": f32(c[:, 1]), "area": c[:, 2].tolist(), "height": f32(c[:, 3]), "left": f32(c[:, 4]),
            "right": f32(c[:, 5]), "artefact": art.tolist()}


def _features(d: dict) -> features.Features:
    n = len(d["mz"])
    a = lambda k: np.array(d[k], float)                                                       # noqa: E731
    return features.Features(mz=a("mz"), rt=a("rt"), apex=np.zeros(n, int), area=a("area"), height=a("height"), npts=np.full(n, 8), left=a("left"),
                             right=a("right"), background=np.zeros(n, bool), artefact=np.array(d["artefact"], bool))


def _nan(a) -> list:
    return [None if np.isnan(v) else float(v) for v in np.asarray(a, float).ravel()]


def _alinea(al) -> dict:
    return {"mz": al.mz.tolist(), "rt": al.rt.tolist(), "area": al.area.ravel().tolist(), "height": al.height.ravel().tolist(),
            "apex_rt": _nan(al.apex_rt), "width": _nan(al.right - al.left)}


def _mappa(al) -> dict:
    m = rtalign.build(al)
    probe = [1.0, 3.3, 7.7, 12.5, 19.9, 25.0]
    return {"ok": bool(m.ok), "anchors": int(m.n_anchors), "grid": m.grid.tolist(), "shift": m.shift.ravel().tolist(), "rms": m.rms.tolist(),
            "probe": probe, "at": [[float(m.at(k, r)) for r in probe] for k in range(al.n_files)],
            "expected": [[float(m.expected(r, k)) for r in probe] for k in range(al.n_files)],
            "reference": [[float(m.reference(r, k)) for r in probe] for k in range(al.n_files)]}


def un_file_allinea(out: Path, golden: Path | None) -> int:
    rng = np.random.default_rng(SEED)
    ions = _ions(rng)
    files = [_file(rng, ions, k) for k in range(N_FILES)]
    fts = [_features(d) for d in files]
    al = features.align(fts)
    res = {"files": files, "align": _alinea(al), "align_span": _alinea(features.align(fts, max_rt_span=0.5)),
           "align_keep": _alinea(features.align(fts, drop_artefacts=False)), "rtmap": _mappa(al)}
    text = json.dumps(res, separators=(",", ":"))
    (out / "allinea_sintetico.json").write_text(text)
    if golden is not None:
        (golden / "allinea_sintetico.json").write_text(text)
    return len(al)
