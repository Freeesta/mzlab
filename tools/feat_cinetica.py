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


# ---------------------------------------------------------------------------------------------------------------- V1 series with known kinetics
SIGMA_ADD, SIGMA_REL, PSEUDO = 3e5, 0.10, 1e6        # additive noise (area), relative noise, "3 x the noise" of reference_difference


def _rumoroso(rng, v):
    v = np.asarray(v, float)
    return np.clip(v * (1 + rng.normal(0, SIGMA_REL, v.shape)) + rng.normal(0, SIGMA_ADD, v.shape), 0, None)


def _esca_casuale(rng, t, n: int) -> list[dict]:
    """Decoys that mimic a product: formation 1 - exp(-k t) with k drawn at random (log-uniform) with no relation to the parent."""
    out = []
    for i in range(n):
        k = float(np.exp(rng.uniform(np.log(0.003), np.log(3.0))))
        out.append({"id": f"DEC-{i:03d}", "ruolo": "esca_casuale", "forma": "monotona", "generazione": 1, "k_form": k, "true": (1e7 * (1 - np.exp(-k * t)) * rng.uniform(0.8, 3.0))})
    return out


def _accoppiato(si, rng, o, t, kd_padre: float) -> dict:
    """The same product with the formation rate of a direct route: k_form = the decay rate of the parent (15 % scatter), the same amplitude."""
    kf = float(kd_padre * np.exp(rng.normal(0, 0.15)))
    p = {"kf": kf} if o.forma == "monotona" else {"kf": kf, "kd": o.k_dec}
    old = np.asarray(o.amp[-len(t):], float)
    scale = old.max() / max(float(si.curva(o.forma, t, {"kf": o.k_form, "kd": o.k_dec} if o.forma == "bateman" else {"kf": o.k_form}).max()), 1e-12)
    return {"id": o.id, "ruolo": o.ruolo, "forma": o.forma, "generazione": 1, "k_form": kf, "true": scale * si.curva(o.forma, t, p)}


def serie_v1(seme: int = 7, n_casuali: int = 200) -> dict:
    """Areas per file of the true products, the isomers, the second generation, the noise coincidences and random-kinetics decoys of the 'completo'
    profile of `tools/valida/sintetici_identita.py`, with noise; the parent saturated and its isotopic series (not clipped)."""
    from valida import sintetici_identita as si
    c = si.costruisci(seme, "completo")
    rng = np.random.default_rng(seme + 100)
    k0 = 2                                              # files: blank, dark, t0, then the treated ones
    t = c.tempi[k0:]
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    yp = np.minimum(padre.amp, padre.clip)
    ions = []
    for o in c.ioni:
        if o.ruolo in ("tp1", "tp2", "isomero"):
            ions.append({"id": o.id, "ruolo": o.ruolo, "forma": o.forma, "generazione": o.generazione, "k_form": o.k_form, "k_dec": o.k_dec, "true": o.amp[k0:]})
        elif o.ruolo == "coincidenza":
            ions.append({"id": o.id, "ruolo": o.ruolo, "forma": "rumore", "generazione": None, "true": np.full(len(c.tempi) - k0, float(rng.lognormal(np.log(6e5), 0.5)))})
    acc = [_accoppiato(si, rng, o, t, padre.k_dec) for o in c.ioni if o.ruolo in ("tp1", "isomero") and o.forma in ("monotona", "bateman")]
    ions += _esca_casuale(rng, t, n_casuali)
    for d in ions:
        full = np.concatenate([[0.0, 0.0], np.asarray(d["true"], float)]) if d["ruolo"] == "esca_casuale" else None
        d["y"] = _rumoroso(rng, d.pop("true")).tolist()
        d["ctrl"] = _rumoroso(rng, np.zeros(2) if full is not None or d["forma"] != "decrescente" else np.full(2, d["y"][0])).tolist()
    for d in acc:
        d["y"] = _rumoroso(rng, d.pop("true")).tolist()
        d["ctrl"] = _rumoroso(rng, np.zeros(2)).tolist()
    return {"times": t.tolist(), "pseudo": PSEUDO, "ions_coupled": acc, "t_sat": c.P["t_sat"], "parent": {"k_dec": padre.k_dec, "y": _rumoroso(rng, yp[k0:]).tolist(),
            "y_isotope": _rumoroso(rng, 0.2 * padre.amp[k0:]).tolist()}, "ions": ions}


def scrivi_v1(out: Path, golden: Path | None) -> int:
    doc = serie_v1()
    txt = json.dumps(doc, separators=(",", ":"))
    (out / "cinetica_v1.json").write_text(txt, encoding="utf-8")
    if golden is not None:
        golden.mkdir(parents=True, exist_ok=True)
        (golden / "cinetica_v1.json").write_text(txt, encoding="utf-8")
    return len(doc["ions"])
