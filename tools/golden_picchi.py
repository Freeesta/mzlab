#!/usr/bin/env python3
"""Golden data for the peak finders (tests/golden/picchi_qualita.json): synthetic traces (single peak, partly resolved doubles, shoulder,
tail, noise) with the limits and areas of the CWT detector of the Python engine (the reference of the Rust port).
The "prominence + local minimum" mode has no Python twin: its expectations (peaks, limits at the valley) are asserted in
crates/mzlab-core/tests/picchi_golden.rs. No compound names. Usage: python3 tools/golden_picchi.py [--scrivi]"""
import json, sys
from pathlib import Path
import numpy as np
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from mzlab.ionfamily import detect_peaks  # noqa: E402

OUT = ROOT / "tests" / "golden" / "picchi_qualita.json"
STEP = 0.015

def gauss(rt, c, w, h):
    return h * np.exp(-0.5 * ((rt - c) / w) ** 2)

def trace(peaks, tail=0.0, noise=50.0, seed=7, n=400):
    rt = np.arange(n) * STEP
    y = sum(gauss(rt, c, w, h) for c, w, h in peaks)
    if tail > 0:
        k = np.exp(-np.arange(n) * STEP / tail); k /= k.sum()
        y = np.convolve(y, k)[:n]
    y = y + 10 * noise + noise * np.random.RandomState(seed).uniform(-1, 1, n)
    return rt, y

CASES = {
    "singolo": trace([(3.0, 0.06, 1e5)]),
    "doppio": trace([(2.5, 0.06, 1e5), (2.5 + 3.4 * 0.06, 0.06, 0.7e5)]),
    "doppio_stretto": trace([(2.5, 0.06, 1e5), (2.5 + 2.4 * 0.06, 0.06, 0.8e5)]),
    "spalla": trace([(2.5, 0.06, 1e5), (2.5 + 1.4 * 0.06, 0.06, 0.4e5)]),
    "coda": trace([(2.0, 0.05, 1e5)], tail=0.12),
}

def build():
    out = {}
    for name, (rt, y) in CASES.items():
        rt = np.round(rt, 6); y = np.array([float(f"{v:.7g}") for v in y])
        pk = sorted(({"apex_i": int(p["apex_i"]), "lo": int(p["lo"]), "hi": int(p["hi"]), "area": float(p["area"])}
                     for p in detect_peaks(rt, y)["peaks"]), key=lambda p: p["apex_i"])
        out[name] = {"rt": rt.tolist(), "y": y.tolist(), "cwt": pk}
    return out

if __name__ == "__main__":
    d = build()
    for k, v in d.items(): print(k, [(p["apex_i"], p["lo"], p["hi"]) for p in v["cwt"]])
    if "--scrivi" in sys.argv:
        OUT.write_text(json.dumps(d, separators=(",", ":")) + "\n"); print("scritto", OUT, OUT.stat().st_size)
