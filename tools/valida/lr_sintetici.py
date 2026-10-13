"""Synthetic MRM series for the low-resolution criteria (six calibration levels, treated files, optional faults); every number is invented.

Transitions: a quantifier (263.1 > 181.1) and its qualifier (263.1 > 152.0) whose area grows with the level, and a flat one (300.2 > 120.1). The lowest level
may leave the quantifier below the threshold (`sotto_soglia`); `guasto` breaks the growth of the quantifier at one level (the Spearman criterion must fail)."""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

from . import ROOT

sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as DS     # noqa: E402

LIVELLI = (1.0, 2.0, 5.0, 10.0, 20.0, 50.0)                      # mg/L, the six standards
TRATTATI = (3.0, 7.0)                                            # mg/L equivalent of the treated files t0, t30
TRANSIZIONI = ((263.1, 181.1, 5.00, 60.0, True, "Quant"), (263.1, 152.0, 5.02, 24.0, True, "Qual"), (300.2, 120.1, 8.00, 400.0, False, "Flat"))


def _chrom(i: int, q1: float, q3: float, t: np.ndarray, y: np.ndarray, nome: str) -> str:
    return DS._chrom(i, f"SRM SIC Q1={q1} Q3={q3} sample=1 period=1 experiment=1 transition={i} ce=20 name={nome}", t, y)


def scrivi_mrm(path: Path, conc: float, seme: int, sotto_soglia: bool = False, valore_guasto: float | None = None, piatta: bool = False) -> None:
    """One MRM file at the concentration `conc` (mg/L): Gaussian peaks and a little noise; `valore_guasto` replaces the quantifier amplitude (a broken growth)."""
    rng = np.random.default_rng(seme)
    t = 0.5 + 0.02 * np.arange(600)
    chroms = []
    for i, (q1, q3, rt, amp, dip, nome) in enumerate(t for t in TRANSIZIONI if piatta or t[4]):
        a = amp * conc if dip else amp
        if dip and sotto_soglia and nome == "Quant":
            a = 4.0
        if valore_guasto is not None and nome == "Quant":
            a = valore_guasto
        y = a * np.exp(-0.5 * ((t - rt) / 0.06) ** 2) + 2.0 * rng.random(t.size)
        chroms.append(_chrom(i, q1, q3, t, y, nome))
    DS._write(Path(path), [], chroms)


def serie_mrm(cartella: Path, seme: int = 5, guasto: bool = False, piatta: bool = False) -> list[Path]:
    """The files `S_MRM-STD_L1..L6` (the lowest below the threshold) and `S_MRM-t0`, `S_MRM-t30`; with `guasto` the quantifier of L4 is as weak as L2's; `piatta` adds the transition that does not depend on the level (it must fail the growth criterion)."""
    cartella.mkdir(parents=True, exist_ok=True)
    out = []
    for k, c in enumerate(LIVELLI):
        p = cartella / f"S_MRM-STD_L{k + 1}-{c:g}ppm.mzML"
        scrivi_mrm(p, c, seme + k, sotto_soglia=(k == 0), valore_guasto=(60.0 * LIVELLI[1] if guasto and k == 3 else None), piatta=piatta)
        out.append(p)
    for t, c in zip((0, 30), TRATTATI):
        p = cartella / f"S_MRM-t{t}.mzML"
        scrivi_mrm(p, c, seme + 10 + t, piatta=piatta)
        out.append(p)
    return out
