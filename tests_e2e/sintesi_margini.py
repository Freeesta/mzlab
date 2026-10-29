"""Synthetic LR mzML for the margin-plot tests: a flat baseline ion plus two narrow peaks, so every XIC window has a visible baseline."""
import sys, os; sys.path.insert(0, str(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools")))
import numpy as np
from pathlib import Path
import dati_sintetici as ds


def margini_mzml(path: Path) -> Path:
    """600 scans, RT 0.5-20 min: ion 250.3 flat at 2e4 (baseline), 300.3 and 350.3 narrow peaks at RT 14.4 and 8.0, light noise."""
    rng = np.random.default_rng(7)
    rts = np.linspace(0.5, 20.0, 600)
    specs, tic, bpc = [], [], []
    for i, rt in enumerate(rts):
        mz = [150.2, 250.3, 300.3, 350.3, 420.3]
        it = [1.5e4 * rng.normal(1, .03), 2e4 * rng.normal(1, .03), 2e4 + 1e6 * np.exp(-.5 * ((rt - 14.4) / .05) ** 2), 2e4 + 6e5 * np.exp(-.5 * ((rt - 8.0) / .05) ** 2), 1e4 * rng.normal(1, .03)]
        mz = np.array(mz); it = np.array(it)
        specs.append(ds._spec(i, rt, mz, it)); tic.append(it.sum()); bpc.append(it.max())
    ds._write(path, specs, [ds._chrom(0, "TIC", rts, tic), ds._chrom(1, "BPC", rts, bpc)])
    return path
