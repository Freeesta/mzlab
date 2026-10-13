# TPMINE-PRIVATE
"""A synthetic unit-resolution LC-MS series (Full Scan only, centroids shifted by +0.30 Da like the real instrument) for the low-resolution engine.
Truth: the parent 241.0641 decays; two isomers of the +O product (257.0590) at RT 7.4 and 8.3 grow with different kinetics; unexpected ions: 333.20
@ 6.8 grows with its 13C peak at 334.20 (an isotope, not an ion of its own), 355.10 @ 5.2 grows and falls (a product of a product), 289.10 @ 4.4 is
flat in every treated file (not at t0, not in the blank), 311.10 @ 3.3 is in every file, blank included. Noise: random peaks that never fall
within 1 Da of a noise peak of the scan before, so they never form a row."""
from __future__ import annotations

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "py"))
from tpmine import demo  # noqa: E402

SHIFT = demo.SHIFT
TIMES = (0, 5, 10, 15, 30, 45, 60)


def _pool(t, blank):
    """(m/z, RT, sigma, amplitude) of what is in a file of treatment time t."""
    out = [(311.10, 3.3, 0.07, 4.0e5), (157.09, 3.0, 0.08, 1.5e5)]
    if blank:
        return out
    k = float(t)
    out.append((demo.PARENT_MZ, 9.0, 0.07, 3.0e6 * np.exp(-0.05 * k)))
    out.append((257.0590, 7.4, 0.07, 1.2e6 * (1 - np.exp(-0.06 * k))))
    out.append((257.0590, 8.3, 0.07, 6.0e5 * (1 - np.exp(-0.03 * k))))
    g = 8.0e5 * (1 - np.exp(-0.05 * k))
    out.append((333.20, 6.8, 0.07, g))
    out.append((334.20, 6.8, 0.07, 0.11 * g))
    out.append((355.10, 5.2, 0.07, 9.0e5 * (np.exp(-0.01 * k) - np.exp(-0.15 * k))))
    if k > 0:
        out.append((289.10, 4.4, 0.07, 5.0e5))
    return out


def _noise(rng, n: int, prev: np.ndarray) -> np.ndarray:
    """m/z of the noise peaks of one scan, none within 1 Da of a noise peak of the scan before: like the real instrument, noise does not repeat
    from scan to scan at the same m/z, so it never forms a chromatographic peak (a row) of its own."""
    out = rng.uniform(100, 400, n)
    for _ in range(50):
        bad = np.array([bool(len(prev)) and np.min(np.abs(prev - m)) < 1.0 for m in out])
        if not bad.any():
            break
        out[bad] = rng.uniform(100, 400, int(bad.sum()))
    return out


def write_series(folder, times=TIMES, n_noise: int = 40, seed: int = 11, scan_step: float = 0.05, blank: bool = True) -> list[dict]:
    """Writes one Full Scan file per time (and a blank) and returns the file list for Experiment."""
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(seed)
    out = []
    for t, is_blank in [(t, False) for t in times] + ([(None, True)] if blank else []):
        comps = _pool(t, is_blank)
        specs = []
        prev = np.empty(0)
        for i, rt in enumerate(np.arange(0.5, 14.0, scan_step)):
            noise = _noise(rng, n_noise, prev)
            prev = noise
            mz = list(noise)
            it = list(rng.lognormal(np.log(3e3), 0.8, n_noise))
            for m, c, s, a in comps:
                v = a * demo._gauss(rt, c, s) * rng.normal(1, 0.04)
                if v > 100:
                    mz.append(m + SHIFT + rng.normal(0, 0.03))
                    it.append(v)
            o = np.argsort(mz)
            specs.append(demo._spectrum(i, 1, float(rt), np.array(mz)[o], np.array(it)[o]))
        name = "lr_blank.mzML" if is_blank else f"lr_FullMass-t{t}.mzML"
        p = folder / name
        p.write_text('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
                     f'<run id="{p.stem}"><spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")
        out.append({"name": name, "path": str(p), "time": None if is_blank else float(t), "type": "blank" if is_blank else "sample"})
    return out
