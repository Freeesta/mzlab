"""Statistics of the comparison: bootstrap of the compounds (the unit of resampling is the compound, so all the rows, files and replicates of a compound stay
together), paired difference of two tools on the same resampled compounds, exact McNemar, Holm, non-inferiority. numpy only; a fixed seed gives the same numbers."""
from __future__ import annotations

import warnings
from math import comb

import numpy as np

N_BOOT = 2000
SEME = 20261012
ALFA = 0.05
MARGINI = {"P1": ("alto", -0.02), "P2": ("basso", 0.05), "P3": ("alto", -0.02), "P5": ("basso", 0.01)}       # pre-registered; "alto": higher is better


def indici_bootstrap(n: int, strati=None, n_boot: int = N_BOOT, seme: int = SEME) -> np.ndarray:
    """(n_boot, n) indices of the compounds drawn with replacement; with `strati` (one label per compound) each stratum is resampled on its own."""
    rng = np.random.default_rng(seme)
    if strati is None:
        return rng.integers(0, n, size=(n_boot, n))
    strati = np.asarray(strati)
    out = np.empty((n_boot, n), int)
    col = 0
    for s in sorted(set(strati.tolist())):
        pos = np.flatnonzero(strati == s)
        out[:, col:col + len(pos)] = pos[rng.integers(0, len(pos), size=(n_boot, len(pos)))]
        col += len(pos)
    return out


def _stat(x: np.ndarray, quale: str) -> np.ndarray:
    """The statistic of every resample (rows of x): the mean of 0/1 values, or the median ignoring the missing values."""
    if quale == "media":
        return x.mean(1)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")                 # a resample with no measured compound gives NaN, not a warning
        return np.nanmedian(x, axis=1)


def _punto(x: np.ndarray, quale: str) -> float:
    if quale == "media":
        return float(np.mean(x)) if len(x) else float("nan")
    return float(np.nanmedian(x)) if len(x) and not np.all(np.isnan(x)) else float("nan")


def intervallo(x, quale: str = "media", strati=None, n_boot: int = N_BOOT, seme: int = SEME) -> dict:
    """Value and 95 % percentile interval of a metric measured per compound."""
    x = np.asarray(x, float)
    if len(x) == 0 or (quale != "media" and np.all(np.isnan(x))):
        return {"n": int(len(x)), "valore": None, "ic_basso": None, "ic_alto": None}
    s = _stat(x[indici_bootstrap(len(x), strati, n_boot, seme)], quale)
    lo, hi = np.nanpercentile(s, [2.5, 97.5])
    return {"n": int(len(x)), "valore": _punto(x, quale), "ic_basso": float(lo), "ic_alto": float(hi)}


def differenza(a, b, quale: str = "media", strati=None, n_boot: int = N_BOOT, seme: int = SEME) -> dict:
    """Paired difference a - b (same compounds, same resamples): value, 95 % interval, and the two-sided bootstrap p-value (0 is the null)."""
    a, b = np.asarray(a, float), np.asarray(b, float)
    if len(a) != len(b):
        raise ValueError("the two tools must be measured on the same compounds")
    if len(a) == 0 or (quale != "media" and (np.all(np.isnan(a)) or np.all(np.isnan(b)))):
        return {"n": int(len(a)), "valore": None, "ic_basso": None, "ic_alto": None, "p": None}
    ix = indici_bootstrap(len(a), strati, n_boot, seme)
    d = _stat(a[ix], quale) - _stat(b[ix], quale)
    d = d[~np.isnan(d)]
    lo, hi = np.percentile(d, [2.5, 97.5])
    p = min(1.0, 2 * min(((d <= 0).sum() + 1) / (len(d) + 1), ((d >= 0).sum() + 1) / (len(d) + 1)))
    return {"n": int(len(a)), "valore": _punto(a, quale) - _punto(b, quale), "ic_basso": float(lo), "ic_alto": float(hi), "p": float(p)}


def mcnemar_esatto(a, b) -> dict:
    """Exact two-sided McNemar test on paired 0/1 outcomes: only the discordant pairs count, under the null each is a fair coin."""
    a, b = np.asarray(a, int), np.asarray(b, int)
    n10, n01 = int(((a == 1) & (b == 0)).sum()), int(((a == 0) & (b == 1)).sum())
    n = n10 + n01
    if n == 0:
        return {"solo_a": 0, "solo_b": 0, "p": 1.0}
    k = min(n10, n01)
    p = min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n)
    return {"solo_a": n10, "solo_b": n01, "p": float(p)}


def holm(p: list[float], alfa: float = ALFA) -> dict:
    """Holm step-down: adjusted p-values (monotone, capped at 1) and which hypotheses are rejected at `alfa`, in the order of the input."""
    m = len(p)
    ordine = sorted(range(m), key=lambda i: (p[i], i))
    adj = [0.0] * m
    corrente = 0.0
    for r, i in enumerate(ordine):
        corrente = max(corrente, min(1.0, (m - r) * p[i]))
        adj[i] = corrente
    return {"p_corretto": adj, "rifiutata": [x < alfa for x in adj]}


def non_inferiore(diff: dict, direzione: str, margine: float) -> bool | None:
    """Non-inferiority of a - b: the interval of the difference must stay on the right side of the margin (a negative margin when higher is better:
    the lower end above it; a positive one when lower is better: the upper end below it). None if there is nothing to compare."""
    if diff.get("valore") is None:
        return None
    return bool(diff["ic_basso"] > margine) if direzione == "alto" else bool(diff["ic_alto"] < margine)


def superiore(diff: dict, direzione: str, rifiutata: bool) -> bool | None:
    """Superiority of a over b: the whole interval on the favourable side of zero and the null rejected after Holm."""
    if diff.get("valore") is None:
        return None
    lato = diff["ic_basso"] > 0 if direzione == "alto" else diff["ic_alto"] < 0
    return bool(lato and rifiutata)
