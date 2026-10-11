"""Small synthetic Full Scan mzML files (MS1 only) for tests that need data but not the real lab files (run anywhere, e.g. in CI)."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from pathlib import Path
import numpy as np
from mzlab import demo


def write_full(path, t, blank=False, std=False, seed=1):
    """One Full Scan mzML (levels 1 only): a peak at m/z 253 that grows with the time t (or a standard, or a blank)."""
    rng = np.random.default_rng(seed)
    specs = []
    for i, rt in enumerate(np.arange(0.5, 10.0, 0.1)):
        mz = list(rng.uniform(100, 400, 20)); it = list(rng.lognormal(np.log(3e3), 0.8, 20))
        if not blank:
            a = 8e5 if std else 8e5 * np.exp(-0.03 * (t or 0)) + 1e5
            v = a * np.exp(-0.5 * ((rt - 5.2) / 0.1) ** 2)
            if v > 100: mz.append(253.3 + rng.normal(0, 0.02)); it.append(v)
        o = np.argsort(mz)
        specs.append(demo._spectrum(i, 1, float(rt), np.array(mz)[o], np.array(it)[o]))
    head = ('<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
            f'<run id="{Path(path).stem}"><spectrumList count="{len(specs)}">')
    Path(path).write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")


def make_series(folder, times, blank=True, std=True):
    """Files s_t<time>.mzML (+ blank.mzML, std_5ppm.mzML); returns the list of paths in the order given."""
    folder = Path(folder); folder.mkdir(parents=True, exist_ok=True); out = []
    for k, t in enumerate(times):
        p = folder / f"s_t{t}.mzML"; write_full(p, t, seed=k); out.append(str(p))
    if blank: p = folder / "blank.mzML"; write_full(p, None, blank=True); out.append(str(p))
    if std: p = folder / "std_5ppm.mzML"; write_full(p, None, std=True); out.append(str(p))
    return out


def make_subtraction(folder, blank=True):
    """SUB_t0, SUB_t10, SUB_t20 (+ SUB_blank): parent that decays, product that appears, background line, RT drift, gain 1.3 (tools/dati_sintetici.py)."""
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tools"))
    import dati_sintetici
    return [str(p) for p in dati_sintetici.subtraction_series(folder, blank)]
