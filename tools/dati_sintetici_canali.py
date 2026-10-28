"""The files of the channel tests (see `dati_sintetici.py`, which calls `canali_casi` from `make`, and `mzlab-cli canali`): targeted MS2 in LC, MSn in blocks
(an infusion) and DDA with 70 precursors, with Thermo-style filter strings. Every number is invented."""
from __future__ import annotations

from pathlib import Path

import numpy as np

from dati_sintetici import _log, _spec, _write


def _canali_scan(i, rt, rng, level, mz0, filt, prec=None, ce=None):
    mz = np.sort(rng.uniform(60, 400, 6)); it = rng.lognormal(_log(2e3), 0.5, 6)
    return _spec(i, rt, mz, it, level=level, prec=prec, ce=ce, filt=filt)


def canali_mirato(path: Path, rng) -> None:
    """Targeted MS2 in LC: a survey scan and the product-ion scans of two FIXED precursors (222.1 and 305.0, 35 eV) at every cycle."""
    specs = []
    for c, rt in enumerate(np.linspace(0.5, 20.0, 100)):
        specs.append(_canali_scan(len(specs), rt, rng, 1, 0, "ITMS + c ESI Full ms [50.00-400.00]"))
        for k, p in enumerate((222.1, 305.0)):
            specs.append(_canali_scan(len(specs), rt + 0.003 * (k + 1), rng, 2, 0, f"ITMS + c ESI Full ms2 {p:.2f}@cid35.00 [50.00-{p + 10:.2f}]", prec=p, ce=35.0))
    _write(path, specs, [])


def canali_blocchi(path: Path, rng) -> None:
    """MSn in blocks (infusion): a survey scan, then 20 scans of MS2 317.2 > 20 of MS3 317.2>261.1 > 20 of MS3 317.2>299.2, each held in turn."""
    specs, rt = [], 0.0
    for path_ in ((317.2,), (317.2, 261.1), (317.2, 299.2)):
        specs.append(_canali_scan(len(specs), rt, rng, 1, 0, "ITMS + c ESI Full ms [100.00-400.00]"))
        stages = " ".join(f"{m:.2f}@cid{25 + 5 * k}.00" for k, m in enumerate(path_))
        for _ in range(20):
            rt += 0.01
            specs.append(_canali_scan(len(specs), rt, rng, len(path_) + 1, 0, f"ITMS + c ESI d Full ms{len(path_) + 1} {stages} [50.00-330.00]", prec=path_[-1], ce=25.0 + 5 * (len(path_) - 1)))
    _write(path, specs, [])


def canali_dda(path: Path, rng) -> None:
    """DDA: a survey scan and one MS2 of a different precursor (70 distinct ones) at every cycle."""
    specs = []
    for c, rt in enumerate(np.linspace(0.5, 20.0, 70)):
        specs.append(_canali_scan(len(specs), rt, rng, 1, 0, "FTMS + p ESI Full ms [100.00-500.00]"))
        p = 150.0 + 4.0 * c
        specs.append(_canali_scan(len(specs), rt + 0.005, rng, 2, 0, f"FTMS + p ESI d Full ms2 {p:.4f}@hcd30.00 [50.00-{p + 10:.2f}]", prec=p, ce=30.0))
    _write(path, specs, [])


def canali_casi(out: Path) -> Path:
    """The files of the channel tests (`mzlab-cli canali`): K_MIRATO, K_BLOCCHI, K_DDA, in their own folder so that the globs of the other files are unchanged."""
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(97)
    canali_mirato(out / "K_MIRATO-t0.mzML", rng); canali_blocchi(out / "K_BLOCCHI-t0.mzML", rng); canali_dda(out / "K_DDA-t0.mzML", rng)
    return out
