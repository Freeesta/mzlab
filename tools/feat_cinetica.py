"""Reference answers of the kinetics of `crates/mzlab-feat/src/kinetic.rs` (test crates/mzlab-feat/tests/kinetic_parity.rs).

Cases: synthetic profiles over 13 times with a known shape (formation, rise and fall, flat, decay, second generation, noise), fitted by
`tpmine.hr.kinetics` (`fit_chain`, `generation`) and by the four shapes written here with the same machinery (square-root space, closed-form scale,
AICc). `cinetica_forme.json` holds the fits, `cinetica_generazione.json` the generation. No compound names."""
import json
import math
from pathlib import Path

import numpy as np

from tpmine.hr import kinetics as K

TEMPI = [0.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0, 90.0, 120.0, 180.0, 240.0]
FORME = ["formation", "rise_fall", "flat", "decay"]


def profili() -> dict[str, list[float]]:
    t = np.array(TEMPI)
    rng = np.random.default_rng(31)
    bat = lambda k1, k2: k1 / (k2 - k1) * (np.exp(-k1 * t) - np.exp(-k2 * t))      # noqa: E731
    ritardo = lambda kp, ka, kb: kp * ka * (np.exp(-kp * t) / ((ka - kp) * (kb - kp)) + np.exp(-ka * t) / ((kp - ka) * (kb - ka)) + np.exp(-kb * t) / ((kp - kb) * (ka - kb)))      # noqa: E731
    base = {"formazione": 1 - np.exp(-0.08 * t), "bateman": bat(0.2, 0.03) / bat(0.2, 0.03).max(), "piatto": np.ones_like(t) * 0.6,
            "calo": np.exp(-0.05 * t), "seconda": ritardo(0.04, 0.12, 0.02) / ritardo(0.04, 0.12, 0.02).max(),
            "uguali": t * np.exp(-0.1 * t) / (np.exp(-1) / 0.1)}
    out = {k: (3e7 * v).tolist() for k, v in base.items()}
    for k in ("formazione", "bateman", "seconda"):
        out[k + "_rumore"] = (3e7 * np.clip(base[k] * (1 + rng.normal(0, 0.12, len(t))), 0, None)).tolist()
    out["zeri"] = [0.0] * len(t)
    return out


def _sqrt_norm(c, cols):
    c = np.asarray(c, float)[..., cols]
    return np.sqrt(np.clip(c / np.maximum(c.max(-1, keepdims=True), 1e-300), 0, None))


def forme_py(t, y, cols=None) -> list[dict]:
    """The four shapes on the columns `cols`, ordered by AICc."""
    t, y = np.asarray(t, float), np.asarray(y, float)
    cols = list(range(len(t))) if cols is None else cols
    tt, yy = t[cols], y[cols]
    yt = np.sqrt(np.clip(yy / max(yy.max(), 1e-300), 0, None))
    g = K.GRID
    fam = {"formation": (1 - np.exp(-g[:, None] * tt), g[:, None]), "flat": (np.ones((1, len(tt))), np.zeros((1, 1))), "decay": (np.exp(-g[:, None] * tt), g[:, None])}
    ks2 = np.array([[a, b] for a in g for b in g])
    fam["rise_fall"] = (K.chain_curve(tt, ks2), ks2)
    out = []
    for name, (C, ks) in fam.items():
        G = np.sqrt(np.clip(C / np.maximum(C.max(1, keepdims=True), 1e-300), 0, None))
        gg = np.maximum((G * G).sum(1), 1e-300)
        gy = G @ yt
        sse = (yt * yt).sum() - gy * gy / gg
        i = int(np.argmin(sse))
        npar = {"formation": 1, "rise_fall": 2, "flat": 0, "decay": 1}[name]
        out.append({"form": name, "rates": [float(v) for v in np.atleast_1d(ks[i])] if npar else [], "scale": float(gy[i] / gg[i]), "sse": float(sse[i]),
                    "aicc": K.aicc(float(sse[i]), len(tt), npar + 1), "order": FORME.index(name)})
    return sorted(out, key=lambda r: (r["aicc"], r["order"]))


def caso_forme(nome: str, y: list[float]) -> dict:
    fits = forme_py(TEMPI, y)
    loo = [forme_py(TEMPI, y, [j for j in range(len(y)) if j != i])[0]["form"] for i in range(len(y))] if max(y) > 0 else []
    return {"name": nome, "times": TEMPI, "y": y, "fits": fits, "leave_one_out": loo}


def caso_generazione(nome: str, y: list[float]) -> dict:
    g = K.generation(TEMPI, y)
    return {"name": nome, "times": TEMPI, "y": y, "verdict": g["verdict"], "aicc_first": g["aicc_first"], "aicc_second": g["aicc_second"],
            "delta": g["delta"], "rates_first": g["first"]["rates"], "rates_second": g["second"]["rates"]}


def scrivi(out: Path, golden: Path | None) -> int:
    pr = profili()
    docs = {"cinetica_forme.json": [caso_forme(k, v) for k, v in pr.items() if max(v) > 0],
            "cinetica_generazione.json": [caso_generazione(k, v) for k, v in pr.items() if max(v) > 0]}
    for nome, doc in docs.items():
        txt = json.dumps(doc, separators=(",", ":"))
        (out / nome).write_text(txt, encoding="utf-8")
        if golden is not None:
            golden.mkdir(parents=True, exist_ok=True)
            (golden / nome).write_text(txt, encoding="utf-8")
    return len(pr)
